//! Downloads one page at a time: fetch it, expand its long posts, save, then move on.
//!
//! Holding a whole account in memory before processing it is what let interrupted downloads
//! lose posts on resume, so the order here is a contract, not an implementation detail.

use std::collections::{HashMap, HashSet};
use std::path::PathBuf;
use std::sync::atomic::Ordering;
use std::time::{Duration, Instant};

use serde::{Deserialize, Serialize};
use tauri::{AppHandle, Emitter, Runtime};

use crate::error::AppError;
use crate::models::{
    CheckpointMeta, DownloadRequest, ExportContext, PostFilter, ProgressEvent, ProgressPhase, RawPost,
    SourceType, UserProfile, WeiboPost,
};
use crate::services::cache;
use crate::services::downloader::{
    count_visible_chars, format_date_range_label, image_extension, needs_long_text,
    normalize_raw_post_for_export, parse_weibo_date_to_timestamp,
};
use crate::services::file_naming::{dedupe_filenames, post_filename};
use crate::services::image_store::ImageStoreService;
use crate::services::source::{ProgressSink, WeiboSource};
use crate::services::weibo_api::{to_weibo_post, WeiboApiClient};
use crate::state::AppState;

/// What a download run actually produced, so the UI can report "done" honestly.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DownloadOutcome {
    /// Posts in the cache after the run, including posts saved by earlier sessions of the same download.
    pub saved: usize,
    /// Long posts whose full text could not be fetched and were kept truncated.
    pub long_text_failed: usize,
    /// Images that could not be downloaded; exports fall back to the remote URL for those.
    pub image_failed: usize,
    /// Page this run continued from, when it resumed an interrupted download.
    pub resumed_from_page: Option<usize>,
    /// Why the run stopped before reaching the end of the timeline, when it did.
    pub stopped_early: Option<String>,
}

impl DownloadOutcome {
    /// Whether the run got everything it asked for.
    pub fn is_complete(&self) -> bool {
        self.long_text_failed == 0 && self.image_failed == 0 && self.stopped_early.is_none()
    }

    /// One line for the log and the completion event.
    pub fn summary(&self) -> String {
        let mut summary = format!("下载完成！共 {} 条微博", self.saved);
        if self.long_text_failed > 0 {
            summary.push_str(&format!("，其中 {} 条长微博未能获取全文", self.long_text_failed));
        }
        if self.image_failed > 0 {
            summary.push_str(&format!("，{} 张图片未能下载", self.image_failed));
        }
        if let Some(reason) = &self.stopped_early {
            summary.push_str(&format!("；{}", reason));
        }
        summary
    }
}

#[derive(Default)]
pub struct DownloadService;

/// Reports progress to the app window.
struct AppSink<'a, R: Runtime>(&'a AppHandle<R>);

impl<R: Runtime> ProgressSink for AppSink<'_, R> {
    fn emit(&self, phase: ProgressPhase, current: usize, total: usize, message: &str) {
        let _ = self
            .0
            .emit("download-progress", ProgressEvent::new(phase, current, total, message));
    }
}

/// Checkpoints written before downloads processed pages one by one recorded the last *fetched* page,
/// so resuming them skipped posts that were fetched but never handled. Only checkpoints that carry
/// this schema are safe to resume.
const CHECKPOINT_SCHEMA: u32 = 2;
const CHECKPOINT_EVERY_POSTS: usize = 50;
const CHECKPOINT_EVERY: Duration = Duration::from_secs(10);
/// Weibo may return an empty page in the middle of a timeline; two in a row means the end.
const MAX_EMPTY_PAGES: usize = 2;
/// Several pages in a row that only contain already-saved posts mean pagination is stuck.
const MAX_REPEATED_PAGES: usize = 3;
/// Every failed long text is counted; only the first few are reported to the log one by one.
const MAX_LOGGED_LONG_TEXT_FAILURES: usize = 3;

impl DownloadService {
    pub fn new() -> Self {
        Self
    }

    pub async fn run<R: Runtime>(
        &self,
        request: &DownloadRequest,
        state: &AppState,
        app: &AppHandle<R>,
    ) -> Result<DownloadOutcome, AppError> {
        state.reset();

        let client = WeiboApiClient::new(app.clone());
        let sink = AppSink(app);
        let result = self.run_with(request, state, &client, &sink).await;

        if let Err(error) = &result {
            if !matches!(error, AppError::Cancelled) {
                sink.emit(ProgressPhase::Error, 0, 0, &format!("下载失败: {error}"));
            }
        }

        result
    }
}

impl DownloadService {
    /// Runs a download against any data source. `run` is this with the real client and the app window.
    pub async fn run_with<S: WeiboSource>(
        &self,
        request: &DownloadRequest,
        state: &AppState,
        source: &S,
        sink: &dyn ProgressSink,
    ) -> Result<DownloadOutcome, AppError> {
        sink.emit(ProgressPhase::FetchingUserInfo, 0, 1, "正在获取用户信息...");

        let user = match request.source_type {
            SourceType::Profile => source.get_user_info(&request.uid).await?,
            SourceType::Favorites => UserProfile {
                uid: "favorites".to_string(),
                screen_name: "我的收藏".to_string(),
            },
        };

        sink.emit(ProgressPhase::FetchingUserInfo, 1, 1, &format!("用户: {}", user.screen_name));

        let starttime = request.date_range.start_timestamp;
        let endtime = request.date_range.end_timestamp;
        let export_context = ExportContext {
            date_range_label: format_date_range_label(starttime, endtime),
            type_label: match request.source_type {
                SourceType::Profile => "微博备份",
                SourceType::Favorites => "收藏微博",
            }
            .to_string(),
        };
        let context = RunContext {
            request,
            state,
            source,
            sink,
            export_context: &export_context,
        };

        let mut walk = WalkState::resume(&context).await;
        let mut page = (walk.last_page + 1) as i64;

        // `Ok` means the walk reached the end of the timeline, or as far as it usefully could.
        let walked = loop {
            if state.is_cancelled() {
                break Err(AppError::Cancelled);
            }

            match self.handle_page(&context, &mut walk, page).await {
                Ok(PageStep::Continue) => {}
                Ok(PageStep::Finished(reason)) => break Ok(reason),
                Err(error) => break Err(error),
            }

            page += 1;
        };

        // Leave a resumable checkpoint behind whatever interrupted the walk.
        let stopped_early = match walked {
            Ok(reason) => {
                walk.save_checkpoint(&context).await?;
                reason
            }
            Err(error) => {
                if walk.dirty {
                    // A failing write must not hide the error that ended the walk.
                    let _ = walk.save_checkpoint(&context).await;
                }
                if matches!(error, AppError::Cancelled) {
                    report_cancelled(sink, &walk);
                }
                return Err(error);
            }
        };

        // Overlapping pages must never leave duplicates in the saved posts.
        let mut seen = HashSet::new();
        walk.posts.retain(|post| seen.insert(post.mblogid.clone()));

        let mut image_failed = 0;
        if request.include_images {
            match self.download_images(&context, &mut walk.posts, &mut image_failed).await {
                Err(error @ AppError::Cancelled) => {
                    report_cancelled(sink, &walk);
                    return Err(error);
                }
                Err(error) => return Err(error),
                Ok(()) => {}
            }
        }

        cache::save_cache_bundle(&walk.posts, &request.output_dir, export_context).await?;
        state.posts_exported.store(walk.posts.len(), Ordering::Relaxed);

        Ok(DownloadOutcome {
            saved: walk.posts.len(),
            long_text_failed: walk.long_text_failed,
            image_failed,
            resumed_from_page: walk.resumed_from_page,
            stopped_early,
        })
    }

    /// Fetches one page and handles everything on it before the walk may move on.
    async fn handle_page<S: WeiboSource>(
        &self,
        context: &RunContext<'_, S>,
        walk: &mut WalkState,
        page: i64,
    ) -> Result<PageStep, AppError> {
        let request = context.request;
        let starttime = request.date_range.start_timestamp;
        let endtime = request.date_range.end_timestamp;

        let (raw_posts, api_total) = match request.source_type {
            SourceType::Profile => context.source.fetch_posts_page(&request.uid, page).await?,
            SourceType::Favorites => {
                context.source.fetch_favorites_page(page, starttime, endtime).await?
            }
        };
        if walk.api_total <= 0 && api_total > 0 {
            walk.api_total = api_total;
        }

        if raw_posts.is_empty() {
            walk.empty_pages += 1;
            if walk.empty_pages >= MAX_EMPTY_PAGES {
                return Ok(PageStep::Finished(None));
            }
            return Ok(PageStep::Continue);
        }
        walk.empty_pages = 0;

        // A page made entirely of posts older than the start date means everything below it is out of range.
        if let Some(start) = starttime.filter(|_| matches!(request.source_type, SourceType::Profile)) {
            let all_older = raw_posts.iter().all(|post| {
                matches!(parse_weibo_date_to_timestamp(&post.created_at), ts if ts > 0 && ts < start)
            });
            if all_older {
                return Ok(PageStep::Finished(None));
            }
        }

        for post in &raw_posts {
            if !post.created_at.is_empty() && parse_weibo_date_to_timestamp(&post.created_at) == 0 {
                context.report_note(
                    walk,
                    &format!("警告：无法解析日期 \"{}\"，已保留该微博", post.created_at),
                );
            }
        }

        let in_range: Vec<RawPost> = raw_posts
            .into_iter()
            .filter(|post| in_date_range(&post.created_at, starttime, endtime))
            .collect();
        let in_range_count = in_range.len();
        let first_time: Vec<RawPost> = in_range
            .into_iter()
            .filter(|post| !walk.existing_ids.contains(&post.mblogid))
            .collect();

        // Pages that are entirely out of range are not a sign of trouble; pages that only repeat
        // what is already saved are.
        if in_range_count > 0 && first_time.is_empty() {
            walk.repeated_pages += 1;
            if walk.repeated_pages >= MAX_REPEATED_PAGES {
                return Ok(PageStep::Finished(Some(format!(
                    "微博接口连续 {MAX_REPEATED_PAGES} 页只返回已保存过的微博，已提前结束"
                ))));
            }
            return Ok(PageStep::Continue);
        }
        walk.repeated_pages = 0;

        walk.total_fetched += first_time.len();
        let fresh: Vec<RawPost> = first_time
            .into_iter()
            .filter(|post| !(request.ignore_deleted && post.user.is_none()))
            .collect();
        self.process_posts(context, walk, fresh, page).await?;

        // The page is fully handled, so a checkpoint may resume after it.
        walk.last_page = page as usize;
        walk.dirty = true;
        context.report_page(walk, page, &format!("已获取 {} 条微博", walk.total_fetched));

        if walk.posts_since_checkpoint >= CHECKPOINT_EVERY_POSTS
            || walk.last_checkpoint_at.elapsed() >= CHECKPOINT_EVERY
        {
            // A failed periodic write is retried on a later page; the final one reports the error.
            if walk.save_checkpoint(context).await.is_ok() {
                walk.last_checkpoint_at = Instant::now();
                walk.posts_since_checkpoint = 0;
                walk.dirty = false;
            }
        }

        Ok(PageStep::Continue)
    }

    /// Expands long posts, converts and filters one page of posts, appending what is kept to `walk.posts`.
    async fn process_posts<S: WeiboSource>(
        &self,
        context: &RunContext<'_, S>,
        walk: &mut WalkState,
        raw_posts: Vec<RawPost>,
        page: i64,
    ) -> Result<(), AppError> {
        let request = context.request;

        // Reposts are dropped by the "original only" filter before any request is made for them.
        let candidates: Vec<RawPost> = raw_posts
            .into_iter()
            .filter(|raw| !(matches!(request.filter, PostFilter::Original) && raw.retweeted_status.is_some()))
            .collect();
        let mut long_texts_total = 0;
        for raw in &candidates {
            if needs_long_text(raw) {
                long_texts_total += 1;
            }
            if raw.retweeted_status.as_deref().is_some_and(needs_long_text) {
                long_texts_total += 1;
            }
        }

        let mut long_texts_done = 0;
        for raw in candidates {
            let top_level_long_text = if needs_long_text(&raw) {
                let text = fetch_long_text(context, walk, &raw.mblogid).await?;
                long_texts_done += 1;
                context.report_page(
                    walk,
                    page,
                    &format!("正在获取长微博全文 {long_texts_done}/{long_texts_total}"),
                );
                text
            } else {
                None
            };

            let retweeted_long_text = match raw.retweeted_status.as_deref() {
                Some(retweeted) if needs_long_text(retweeted) => {
                    let text = fetch_long_text(context, walk, &retweeted.mblogid).await?;
                    long_texts_done += 1;
                    context.report_page(
                        walk,
                        page,
                        &format!("正在获取长微博全文 {long_texts_done}/{long_texts_total}"),
                    );
                    text
                }
                _ => None,
            };

            let uid_for_normalize = match request.source_type {
                SourceType::Profile => request.uid.as_str(),
                SourceType::Favorites => "",
            };
            let normalized = normalize_raw_post_for_export(raw, top_level_long_text, retweeted_long_text);
            let post = to_weibo_post(&normalized, uid_for_normalize);

            // Only the profile mode has the length filter, and only after long posts are expanded.
            if matches!(request.source_type, SourceType::Profile)
                && request.min_text_length > 0
                && count_visible_chars(&post.text) < request.min_text_length
            {
                continue;
            }
            if !walk.existing_ids.insert(post.mblogid.clone()) {
                continue;
            }
            walk.posts.push(post);
            walk.posts_since_checkpoint += 1;
            walk.dirty = true;
            context.state.posts_exported.store(walk.posts.len(), Ordering::Relaxed);
        }

        Ok(())
    }

    /// Downloads the images of every saved post. A failed image is counted, not fatal: exports fall
    /// back to the remote URL for it.
    async fn download_images<S: WeiboSource>(
        &self,
        context: &RunContext<'_, S>,
        posts: &mut [WeiboPost],
        image_failed: &mut usize,
    ) -> Result<(), AppError> {
        let request = context.request;
        let state = context.state;
        let sink = context.sink;

        sink.emit(ProgressPhase::DownloadingImages, 0, 0, "正在下载图片...");

        let image_dir = PathBuf::from(&request.output_dir).join("images");
        tokio::fs::create_dir_all(&image_dir).await?;

        let total_images: usize = posts.iter().map(|post| post.images.len()).sum();
        if total_images == 0 {
            sink.emit(ProgressPhase::DownloadingImages, 0, 0, "没有图片需要下载");
            return Ok(());
        }

        // Collect every unique (url, filename) job across posts.
        let mut download_jobs: Vec<(String, String)> = Vec::with_capacity(total_images);
        let mut seen_urls: HashSet<String> = HashSet::with_capacity(total_images);
        for post in posts.iter() {
            let filenames = dedupe_filenames(
                &post
                    .images
                    .iter()
                    .map(|image| {
                        post_filename(&post.created_at, &post.mblogid, &image_extension(&image.original_url))
                    })
                    .collect::<Vec<_>>(),
            );

            for (image, filename) in post.images.iter().zip(filenames) {
                if seen_urls.insert(image.original_url.clone()) {
                    download_jobs.push((image.original_url.clone(), filename));
                }
            }
        }

        let total_unique = download_jobs.len();
        let client = state.get_client();
        let mut completed = 0;

        let stored_paths: HashMap<String, String> = ImageStoreService::new()
            .download_images_concurrent(&client, download_jobs, &image_dir, state, |ok| {
                completed += 1;
                if !ok {
                    *image_failed += 1;
                }
                // Throttle UI progress events slightly for large galleries.
                if completed == total_unique || completed % 2 == 0 || total_unique <= 20 {
                    let succeeded = completed - *image_failed;
                    sink.emit(
                        ProgressPhase::DownloadingImages,
                        completed,
                        total_unique,
                        &format!("已下载 {succeeded}/{total_unique} 张图片..."),
                    );
                }
            })
            .await?
            .into_iter()
            .collect();

        for post in posts.iter_mut() {
            for image in post.images.iter_mut() {
                image.local_path = stored_paths
                    .get(&image.original_url)
                    .map(|filename| format!("images/{filename}"));
            }
        }

        Ok(())
    }
}

/// What the walk should do after one page.
enum PageStep {
    Continue,
    /// The end of the timeline was reached, or the walk had to stop early.
    Finished(Option<String>),
}

/// Everything the page handlers need.
struct RunContext<'a, S: WeiboSource> {
    request: &'a DownloadRequest,
    state: &'a AppState,
    source: &'a S,
    sink: &'a dyn ProgressSink,
    export_context: &'a ExportContext,
}

impl<S: WeiboSource> RunContext<'_, S> {
    /// `(current, total)` of the walk. With a date range Weibo's total says nothing about how many
    /// posts are in range, so no total is reported and the UI shows page and post counts instead.
    fn progress(&self, walk: &WalkState) -> (usize, usize) {
        let ranged = self.request.date_range.start_timestamp.is_some()
            || self.request.date_range.end_timestamp.is_some();
        self.state.posts_exported.store(walk.posts.len(), Ordering::Relaxed);
        if ranged {
            (walk.total_fetched, 0)
        } else {
            let total = walk.api_total.max(0) as usize;
            (walk.total_fetched, total.max(walk.total_fetched))
        }
    }

    fn report_page(&self, walk: &WalkState, page: i64, activity: &str) {
        let (current, total) = self.progress(walk);
        self.sink.emit(
            ProgressPhase::FetchingPostList,
            current,
            total,
            &format!("第 {page} 页 · {activity}"),
        );
    }

    /// A warning that leaves the progress bar where it is.
    fn report_note(&self, walk: &WalkState, note: &str) {
        let (current, total) = self.progress(walk);
        self.sink.emit(ProgressPhase::FetchingPostList, current, total, note);
    }
}

/// Progress of one walk through the timeline.
struct WalkState {
    /// Posts kept so far, in the order they were fetched.
    posts: Vec<WeiboPost>,
    existing_ids: HashSet<String>,
    /// Every page up to this one is fully handled, so a checkpoint may resume after it.
    last_page: usize,
    /// Posts seen so far that were in range and new (progress display only).
    total_fetched: usize,
    /// Post count reported by Weibo, when it reports one.
    api_total: i64,
    long_text_failed: usize,
    resumed_from_page: Option<usize>,
    empty_pages: usize,
    repeated_pages: usize,
    /// Something changed since the last checkpoint was written.
    dirty: bool,
    last_checkpoint_at: Instant,
    posts_since_checkpoint: usize,
}

impl WalkState {
    fn new() -> Self {
        Self {
            posts: Vec::new(),
            existing_ids: HashSet::new(),
            last_page: 0,
            total_fetched: 0,
            api_total: 0,
            long_text_failed: 0,
            resumed_from_page: None,
            empty_pages: 0,
            repeated_pages: 0,
            dirty: false,
            last_checkpoint_at: Instant::now(),
            posts_since_checkpoint: 0,
        }
    }

    /// Continues an interrupted download when its checkpoint was written for exactly these settings.
    async fn resume<S: WeiboSource>(context: &RunContext<'_, S>) -> Self {
        let request = context.request;
        let Ok(bundle) = cache::load_cache_bundle(&request.output_dir).await else {
            return Self::new();
        };
        let Some(checkpoint) = bundle.checkpoint else {
            return Self::new();
        };
        if checkpoint.schema != CHECKPOINT_SCHEMA || !checkpoint_matches(&checkpoint, request) {
            return Self::new();
        }

        context.sink.emit(
            ProgressPhase::Resuming,
            0,
            checkpoint.total_fetched,
            &format!(
                "从第 {} 页继续下载（已保存 {} 条）...",
                checkpoint.last_page + 1,
                bundle.posts.len()
            ),
        );

        Self {
            existing_ids: bundle.posts.iter().map(|post| post.mblogid.clone()).collect(),
            last_page: checkpoint.last_page,
            total_fetched: checkpoint.total_fetched,
            api_total: checkpoint.total_posts,
            long_text_failed: checkpoint.long_text_failed,
            resumed_from_page: Some(checkpoint.last_page + 1),
            posts: bundle.posts,
            ..Self::new()
        }
    }

    async fn save_checkpoint<S: WeiboSource>(&self, context: &RunContext<'_, S>) -> Result<(), AppError> {
        let request = context.request;
        cache::save_cache_checkpoint(
            &self.posts,
            &request.output_dir,
            CheckpointMeta {
                uid: request.uid.clone(),
                source_type: Some(request.source_type.clone()),
                last_page: self.last_page,
                total_fetched: self.total_fetched,
                total_posts: self.api_total,
                filter: Some(request.filter.clone()),
                start_timestamp: request.date_range.start_timestamp,
                end_timestamp: request.date_range.end_timestamp,
                schema: CHECKPOINT_SCHEMA,
                min_text_length: request.min_text_length,
                ignore_deleted: request.ignore_deleted,
                long_text_failed: self.long_text_failed,
            },
            context.export_context,
        )
        .await
    }
}

fn checkpoint_matches(checkpoint: &CheckpointMeta, request: &DownloadRequest) -> bool {
    checkpoint.uid == request.uid
        && checkpoint.source_type.as_ref().is_none_or(|source| source == &request.source_type)
        && checkpoint.filter.as_ref().is_none_or(|filter| filter == &request.filter)
        && checkpoint.start_timestamp == request.date_range.start_timestamp
        && checkpoint.end_timestamp == request.date_range.end_timestamp
        && checkpoint.min_text_length == request.min_text_length
        && checkpoint.ignore_deleted == request.ignore_deleted
}

/// Posts whose date cannot be parsed are kept rather than dropped silently.
fn in_date_range(created_at: &str, starttime: Option<i64>, endtime: Option<i64>) -> bool {
    let timestamp = parse_weibo_date_to_timestamp(created_at);
    if timestamp == 0 {
        return true;
    }
    starttime.is_none_or(|start| timestamp >= start) && endtime.is_none_or(|end| timestamp <= end)
}

/// Fetches the full text of a long post. Losing the session or the user's patience must stop the
/// run; anything else leaves this post truncated, is counted, and is reported at the end.
async fn fetch_long_text<S: WeiboSource>(
    context: &RunContext<'_, S>,
    walk: &mut WalkState,
    mblogid: &str,
) -> Result<Option<String>, AppError> {
    match context.source.get_long_text(mblogid).await {
        Ok(content) => Ok(Some(content)),
        Err(error @ (AppError::Cancelled | AppError::InvalidCookie | AppError::NotLoggedIn)) => Err(error),
        Err(error) => {
            if walk.long_text_failed < MAX_LOGGED_LONG_TEXT_FAILURES {
                let reason: String = error.to_string().chars().take(60).collect();
                context.report_note(
                    walk,
                    &format!("长微博 {mblogid} 全文获取失败，已保留截断内容：{reason}"),
                );
            }
            walk.long_text_failed += 1;
            Ok(None)
        }
    }
}

fn report_cancelled(sink: &dyn ProgressSink, walk: &WalkState) {
    let saved_anything = !walk.posts.is_empty() || walk.last_page > 0;
    let message = if saved_anything {
        format!(
            "进度已保存（已获取 {} 条微博），点击「继续下载」会从第 {} 页接着下载",
            walk.posts.len(),
            walk.last_page + 1
        )
    } else {
        "已取消".to_string()
    };
    sink.emit(ProgressPhase::Cancelled, walk.posts.len(), 0, &message);
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;
    use std::path::{Path, PathBuf};
    use std::sync::{Arc, Mutex};

    use super::*;
    use crate::models::{DateRange, RawPostUser};
    use chrono::TimeZone;

    const UID: &str = "1234567890";

    /// How the fake API fails.
    #[derive(Clone, Copy, Debug)]
    enum Failure {
        Transient,
        SessionExpired,
        LoggedOut,
        Cancelled,
    }

    fn fake_error(failure: Failure) -> AppError {
        match failure {
            Failure::Transient => AppError::ApiError("模拟的临时错误".to_string()),
            Failure::SessionExpired => AppError::InvalidCookie,
            Failure::LoggedOut => AppError::NotLoggedIn,
            Failure::Cancelled => AppError::Cancelled,
        }
    }

    /// A scripted stand-in for the Weibo API: one entry per page, plus per-post long text outcomes.
    struct FakeSource {
        pages: Vec<Vec<RawPost>>,
        long_text: HashMap<String, Result<String, Failure>>,
        fail_page_with: Option<(i64, Failure)>,
        requested_pages: Mutex<Vec<i64>>,
    }

    impl FakeSource {
        fn new(pages: Vec<Vec<RawPost>>) -> Self {
            Self {
                pages,
                long_text: HashMap::new(),
                fail_page_with: None,
                requested_pages: Mutex::new(Vec::new()),
            }
        }

        fn requested_pages(&self) -> Vec<i64> {
            self.requested_pages.lock().unwrap().clone()
        }
    }

    impl WeiboSource for FakeSource {
        async fn get_user_info(&self, _uid: &str) -> Result<UserProfile, AppError> {
            Ok(UserProfile {
                uid: UID.to_string(),
                screen_name: "测试博主".to_string(),
            })
        }

        async fn fetch_posts_page(&self, _uid: &str, page: i64) -> Result<(Vec<RawPost>, i64), AppError> {
            self.requested_pages.lock().unwrap().push(page);
            if let Some((at, failure)) = self.fail_page_with {
                if at == page {
                    return Err(fake_error(failure));
                }
            }
            match self.pages.get(page as usize - 1) {
                Some(posts) => Ok((posts.clone(), (self.pages.len() * 3) as i64)),
                None => Ok((Vec::new(), 0)),
            }
        }

        async fn fetch_favorites_page(
            &self,
            page: i64,
            _starttime: Option<i64>,
            _endtime: Option<i64>,
        ) -> Result<(Vec<RawPost>, i64), AppError> {
            self.fetch_posts_page(UID, page).await
        }

        async fn get_long_text(&self, mblogid: &str) -> Result<String, AppError> {
            match self.long_text.get(mblogid) {
                Some(Ok(text)) => Ok(text.clone()),
                Some(Err(failure)) => Err(fake_error(*failure)),
                None => Err(AppError::ApiError("没有长文".to_string())),
            }
        }
    }

    /// Posts one day apart, newest first, so date ranges and early stops can be exercised.
    fn post(index: usize) -> RawPost {
        let date = chrono::NaiveDate::from_ymd_opt(2024, 6, 1).unwrap() - chrono::Duration::days(index as i64);
        RawPost {
            mblogid: format!("p{index}"),
            created_at: format!("{date} 12:00:00"),
            text: "正文内容".to_string(),
            is_long_text: Some(false),
            user: Some(RawPostUser {
                id: 1,
                screen_name: "测试博主".to_string(),
            }),
            ..Default::default()
        }
    }

    fn timeline(pages: usize, per_page: usize) -> Vec<Vec<RawPost>> {
        (0..pages)
            .map(|page| (0..per_page).map(|i| post(page * per_page + i)).collect())
            .collect()
    }

    fn long_post(index: usize) -> RawPost {
        RawPost {
            text: "开头展开全文".to_string(),
            is_long_text: Some(true),
            ..post(index)
        }
    }

    fn with_image(mut raw: RawPost) -> RawPost {
        raw.pic_infos = Some(serde_json::json!({
            "a": { "large": { "url": "http://127.0.0.1:9/a.jpg", "width": 100, "height": 100 } }
        }));
        raw
    }

    fn request(dir: &Path) -> DownloadRequest {
        DownloadRequest {
            uid: UID.to_string(),
            source_type: SourceType::Profile,
            cookie: String::new(),
            filter: PostFilter::All,
            include_images: false,
            date_range: DateRange {
                start_timestamp: None,
                end_timestamp: None,
            },
            output_dir: dir.to_string_lossy().to_string(),
            ignore_deleted: false,
            min_text_length: 0,
            limit: 0,
        }
    }

    fn beijing_timestamp(year: i32, month: u32, day: u32, hour: u32, minute: u32, second: u32) -> i64 {
        chrono::FixedOffset::east_opt(8 * 3600)
            .unwrap()
            .with_ymd_and_hms(year, month, day, hour, minute, second)
            .single()
            .unwrap()
            .timestamp()
    }

    fn temp_output_dir(name: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("wecun-pipeline-{name}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }

    fn cache_in(dir: &Path) -> crate::models::CachedPosts {
        cache::load_cache_bundle_sync(&dir.to_string_lossy()).expect("a cache file")
    }

    fn saved_ids(dir: &Path) -> Vec<String> {
        cache_in(dir).posts.into_iter().map(|post| post.mblogid).collect()
    }

    #[derive(Default)]
    struct VecSink(Mutex<Vec<ProgressEvent>>);

    impl ProgressSink for VecSink {
        fn emit(&self, phase: ProgressPhase, current: usize, total: usize, message: &str) {
            self.0
                .lock()
                .unwrap()
                .push(ProgressEvent::new(phase, current, total, message));
        }
    }

    async fn run(
        request: &DownloadRequest,
        source: &FakeSource,
    ) -> (Result<DownloadOutcome, AppError>, Arc<AppState>, Vec<ProgressEvent>) {
        let state = Arc::new(AppState::default());
        let sink = VecSink::default();
        let result = DownloadService.run_with(request, &state, source, &sink).await;
        let events = sink.0.lock().unwrap().clone();
        (result, state, events)
    }

    fn event(events: &[ProgressEvent], phase: ProgressPhase) -> Option<&ProgressEvent> {
        events.iter().find(|event| event.phase == phase)
    }

    #[test]
    fn summary_mentions_every_shortfall() {
        let clean = DownloadOutcome {
            saved: 8,
            ..DownloadOutcome::default()
        };
        assert_eq!(clean.summary(), "下载完成！共 8 条微博");
        assert!(clean.is_complete());

        let partial = DownloadOutcome {
            saved: 8,
            long_text_failed: 2,
            image_failed: 5,
            stopped_early: Some("接口异常".to_string()),
            ..DownloadOutcome::default()
        };
        let summary = partial.summary();
        assert!(
            summary.contains("2 条长微博") && summary.contains("5 张图片") && summary.contains("接口异常"),
            "{summary}"
        );
        assert!(!partial.is_complete());
    }

    #[tokio::test]
    async fn resuming_after_a_stop_saves_every_post() {
        let dir = temp_output_dir("resume");
        let pages = timeline(4, 3);

        let stopped = FakeSource {
            fail_page_with: Some((3, Failure::Cancelled)),
            ..FakeSource::new(pages.clone())
        };
        let (result, _, first_events) = run(&request(&dir), &stopped).await;
        assert!(matches!(result, Err(AppError::Cancelled)));

        // Pages 1 and 2 were finished before the stop, so that is where the next run must continue.
        let checkpoint = cache_in(&dir).checkpoint.expect("a stopped download leaves a checkpoint");
        assert_eq!((checkpoint.schema, checkpoint.last_page), (CHECKPOINT_SCHEMA, 2));
        assert_eq!(saved_ids(&dir), ["p0", "p1", "p2", "p3", "p4", "p5"]);
        let cancelled = event(&first_events, ProgressPhase::Cancelled).expect("the stop is reported");
        assert_eq!(cancelled.current, 6, "the UI offers to continue only when something was saved");
        assert!(cancelled.message.contains("第 3 页"), "{}", cancelled.message);

        let (result, _, events) = run(&request(&dir), &FakeSource::new(pages)).await;
        let outcome = result.unwrap();

        assert_eq!(outcome.saved, 12, "no post may be skipped between the two runs");
        assert_eq!(outcome.resumed_from_page, Some(3));
        assert!(event(&events, ProgressPhase::Resuming).is_some());
        assert!(cache_in(&dir).checkpoint.is_none(), "a finished download is not resumable");
        assert!(
            events.iter().any(|event| matches!(event.phase, ProgressPhase::FetchingPostList) && event.total == 12 && event.current > 0),
            "the list phase must report a real total"
        );
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[tokio::test]
    async fn losing_the_session_stops_the_run_instead_of_saving_truncated_posts() {
        let cases = [(Failure::SessionExpired, "session"), (Failure::LoggedOut, "logged-out")];

        for (failure, name) in cases {
            let dir = temp_output_dir(name);
            let mut pages = timeline(3, 3);
            pages[1][0] = long_post(3);

            let broken = FakeSource {
                long_text: HashMap::from([("p3".to_string(), Err(failure))]),
                ..FakeSource::new(pages.clone())
            };
            let (result, _, _) = run(&request(&dir), &broken).await;

            match failure {
                Failure::SessionExpired => assert!(matches!(result, Err(AppError::InvalidCookie)), "{name}"),
                Failure::LoggedOut => assert!(matches!(result, Err(AppError::NotLoggedIn)), "{name}"),
                other => panic!("{other:?} is not an auth failure"),
            }
            assert_eq!(saved_ids(&dir), ["p0", "p1", "p2"], "{name}: only the finished page may be saved");

            let restored = FakeSource {
                long_text: HashMap::from([("p3".to_string(), Ok("完整的长微博全文".to_string()))]),
                ..FakeSource::new(pages)
            };
            let (result, _, _) = run(&request(&dir), &restored).await;
            let outcome = result.unwrap();

            assert_eq!(outcome.saved, 9, "{name}");
            assert_eq!(outcome.resumed_from_page, Some(2), "{name}");
            assert_eq!(cache_in(&dir).posts[3].text, "完整的长微博全文", "{name}");
            let _ = std::fs::remove_dir_all(&dir);
        }
    }

    #[tokio::test]
    async fn long_texts_that_fail_are_kept_truncated_and_counted() {
        let dir = temp_output_dir("long-text");
        let mut pages = timeline(1, 3);
        pages[0][0] = long_post(0);
        pages[0][2] = long_post(2);
        let source = FakeSource {
            long_text: HashMap::from([
                ("p0".to_string(), Ok("完整的长微博全文".to_string())),
                ("p2".to_string(), Err(Failure::Transient)),
            ]),
            ..FakeSource::new(pages)
        };

        let (result, _, events) = run(&request(&dir), &source).await;
        let outcome = result.unwrap();

        assert_eq!(outcome.saved, 3);
        assert_eq!(outcome.long_text_failed, 1);
        assert!(!outcome.is_complete());
        assert_eq!(cache_in(&dir).posts[0].text, "完整的长微博全文");
        assert_eq!(cache_in(&dir).posts[2].text, "开头展开全文", "a failed fetch must not lose the post");
        assert!(
            events.iter().any(|event| event.message.contains("p2") && event.message.contains("全文获取失败")),
            "the failed long text is named in the log"
        );
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[tokio::test]
    async fn an_old_date_range_reaches_its_posts_however_many_newer_pages_come_first() {
        let dir = temp_output_dir("range");
        // Posts run back from 2024-06-01, so November 2023 sits behind 61 pages of newer posts.
        let source = FakeSource::new(timeline(80, 3));
        let mut scoped = request(&dir);
        scoped.date_range = DateRange {
            start_timestamp: Some(beijing_timestamp(2023, 11, 1, 0, 0, 0)),
            end_timestamp: Some(beijing_timestamp(2023, 11, 30, 23, 59, 59)),
        };

        let (result, _, events) = run(&scoped, &source).await;
        let outcome = result.unwrap();

        assert_eq!(outcome.saved, 30, "every post from November 2023");
        assert_eq!(outcome.stopped_early, None);
        assert_eq!(source.requested_pages().len(), 73, "stops on the first page entirely older than the range");
        assert!(
            events.iter().all(|event| event.total == 0 || !matches!(event.phase, ProgressPhase::FetchingPostList)),
            "with a date range the API total says nothing about how many posts are in range"
        );
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[tokio::test]
    async fn pagination_that_only_repeats_saved_posts_stops_and_says_so() {
        let dir = temp_output_dir("stuck");
        let page = timeline(1, 3).remove(0);
        let source = FakeSource::new(vec![page; 10]);

        let (result, _, _) = run(&request(&dir), &source).await;
        let outcome = result.unwrap();

        assert_eq!(outcome.saved, 3);
        assert_eq!(source.requested_pages().len(), MAX_REPEATED_PAGES + 1);
        let reason = outcome.stopped_early.expect("the walk did not reach the end of the timeline");
        assert!(reason.contains("提前结束"), "{reason}");
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[tokio::test]
    async fn a_checkpoint_for_other_settings_is_not_resumed() {
        let dir = temp_output_dir("settings");
        let pages = timeline(4, 3);
        let stopped = FakeSource {
            fail_page_with: Some((3, Failure::Cancelled)),
            ..FakeSource::new(pages.clone())
        };
        assert!(run(&request(&dir), &stopped).await.0.is_err());

        let mut stricter = request(&dir);
        stricter.min_text_length = 50;
        let (result, _, _) = run(&stricter, &FakeSource::new(pages)).await;
        let outcome = result.unwrap();

        assert_eq!(outcome.resumed_from_page, None, "posts saved under other filters must not be mixed in");
        assert_eq!(outcome.saved, 0, "no post is long enough for the new filter");
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[tokio::test]
    async fn checkpoints_from_older_versions_are_not_resumed() {
        let dir = temp_output_dir("legacy");
        // Before pages were processed one by one, `last_page` meant the last page *fetched*.
        let legacy = CheckpointMeta {
            uid: UID.to_string(),
            source_type: Some(SourceType::Profile),
            last_page: 2,
            total_fetched: 6,
            total_posts: 12,
            filter: Some(PostFilter::All),
            start_timestamp: None,
            end_timestamp: None,
            schema: 0,
            min_text_length: 0,
            ignore_deleted: false,
            long_text_failed: 0,
        };
        let earlier: Vec<WeiboPost> = timeline(2, 3)
            .into_iter()
            .flatten()
            .map(|raw| to_weibo_post(&raw, UID))
            .collect();
        cache::save_cache_checkpoint(&earlier, &dir.to_string_lossy(), legacy, &ExportContext::default())
            .await
            .unwrap();

        let (result, _, _) = run(&request(&dir), &FakeSource::new(timeline(4, 3))).await;
        let outcome = result.unwrap();

        assert_eq!(outcome.resumed_from_page, None);
        assert_eq!(outcome.saved, 12);
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[tokio::test]
    async fn images_that_fail_to_download_are_counted() {
        let dir = temp_output_dir("images");
        let mut pages = timeline(1, 1);
        pages[0][0] = with_image(post(0));
        let mut with_pictures = request(&dir);
        with_pictures.include_images = true;

        let (result, _, _) = run(&with_pictures, &FakeSource::new(pages)).await;
        let outcome = result.unwrap();

        assert_eq!(outcome.image_failed, 1);
        assert_eq!(cache_in(&dir).posts[0].images[0].local_path, None, "exports fall back to the remote URL");
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[tokio::test]
    async fn stopping_before_anything_is_saved_leaves_no_checkpoint() {
        let dir = temp_output_dir("nothing-saved");
        let source = FakeSource {
            fail_page_with: Some((1, Failure::Cancelled)),
            ..FakeSource::new(timeline(2, 3))
        };

        let (result, _, events) = run(&request(&dir), &source).await;

        assert!(matches!(result, Err(AppError::Cancelled)));
        assert!(cache::load_cache_bundle_sync(&dir.to_string_lossy()).is_err());
        let cancelled = event(&events, ProgressPhase::Cancelled).expect("the stop is reported");
        assert_eq!(cancelled.current, 0);
        let _ = std::fs::remove_dir_all(&dir);
    }
}
