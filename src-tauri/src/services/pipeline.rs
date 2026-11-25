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
    BackupMeta, CheckpointMeta, DownloadRequest, ExportContext, PostFilter, ProgressEvent, ProgressPhase, RawPost,
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
    /// Posts this run added to an earlier backup, when it was an update. `None` for a full download.
    pub added: Option<usize>,
    /// Posts of the earlier backup that an update kept as they were.
    pub unchanged: usize,
    /// Set when the directory held a backup that could not be updated, saying why the run started over.
    pub started_over: Option<String>,
}

impl DownloadOutcome {
    /// Whether the run got everything it asked for.
    pub fn is_complete(&self) -> bool {
        self.long_text_failed == 0 && self.image_failed == 0 && self.stopped_early.is_none()
    }

    /// One line for the log and the completion event.
    pub fn summary(&self) -> String {
        let mut summary = match self.added {
            Some(added) => format!("更新完成！新增 {added} 条微博，共 {} 条", self.saved),
            None => format!("下载完成！共 {} 条微博", self.saved),
        };
        if self.long_text_failed > 0 {
            summary.push_str(&format!("，其中 {} 条长微博未能获取全文", self.long_text_failed));
        }
        if self.image_failed > 0 {
            summary.push_str(&format!("，{} 张图片未能下载", self.image_failed));
        }
        if let Some(reason) = &self.stopped_early {
            summary.push_str(&format!("；{}", reason));
        }
        if let Some(reason) = &self.started_over {
            summary.push_str(&format!("（{reason}）"));
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

        let (mut walk, first_page, started_over) = WalkState::start(&context).await;
        let mut page = first_page as i64;

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
                walk.checkpoint_on_disk = Some((walk.last_page, walk.posts.len()));
                reason
            }
            Err(error) => {
                if walk.dirty && walk.save_checkpoint(&context).await.is_ok() {
                    walk.checkpoint_on_disk = Some((walk.last_page, walk.posts.len()));
                    walk.dirty = false;
                }
                log::error!("download stopped: {error}");
                // A failing write must not hide the error that ended the walk, and "your progress is
                // saved" may only be said when a checkpoint really is on disk.
                if matches!(error, AppError::Cancelled) {
                    report_cancelled(sink, &walk);
                } else {
                    report_progress_saved(sink, &walk);
                }
                return Err(error);
            }
        };

        // An update appends what it finds after the posts it started from, and a resumed walk reads
        // one page again and appends what it finds there, so newer posts can arrive after older
        // ones. They belong in front of the old ones; a profile timeline is then put back in
        // post-time order on top of that.
        if walk.base_len > 0 {
            let mut new_posts = walk.posts.split_off(walk.base_len);
            // In the order the walk first saw them: a resumed update finds posts above the ones an
            // earlier attempt saved, and those are newer.
            new_posts.sort_by_key(|post| walk.sighting_index.get(&post.mblogid).copied().unwrap_or(usize::MAX));
            walk.posts.splice(0..0, new_posts);
        }
        if matches!(request.source_type, SourceType::Profile) {
            walk.posts = in_timeline_order(std::mem::take(&mut walk.posts));
        }

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
                Err(error) => {
                    report_progress_saved(sink, &walk);
                    return Err(error);
                }
                Ok(()) => {}
            }
        }

        let backup = BackupMeta {
            uid: request.uid.clone(),
            source_type: request.source_type.clone(),
            post_count: walk.posts.len(),
            author: match request.source_type {
                SourceType::Profile => user.screen_name.clone(),
                SourceType::Favorites => "我的收藏".to_string(),
            },
            filter: request.filter.clone(),
            min_text_length: request.min_text_length,
            ignore_deleted: request.ignore_deleted,
            start_timestamp: request.date_range.start_timestamp,
            end_timestamp: request.date_range.end_timestamp,
        };
        if let Err(error) =
            cache::save_finished_backup(&walk.posts, &request.output_dir, export_context, &backup, walk.sorted_skipped())
                .await
        {
            report_progress_saved(sink, &walk);
            return Err(error);
        }
        state.posts_exported.store(walk.posts.len(), Ordering::Relaxed);

        let outcome = DownloadOutcome {
            saved: walk.posts.len(),
            long_text_failed: walk.long_text_failed,
            image_failed,
            resumed_from_page: walk.resumed_from_page,
            stopped_early,
            added: walk.updating.then(|| walk.posts.len() - walk.base_len),
            unchanged: if walk.updating { walk.base_len } else { 0 },
            started_over,
        };
        log::info!("download finished: {}", outcome.summary());
        if let Some(page) = outcome.resumed_from_page {
            log::info!("this run continued from page {page}");
        }
        Ok(outcome)
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
        let base_handled = |post: &RawPost| {
            walk.base_ids.contains(&post.mblogid) || walk.base_skipped.contains(&post.mblogid)
        };
        // Whether the backup being updated went through this whole page. Only that ends an update:
        // pages only get older from here, and that backup went through all of them. What this run
        // itself went through (pages it read before an interruption, new posts it left out) does
        // not: there may be more below.
        let all_base_handled = in_range.iter().all(&base_handled);
        if walk.updating {
            for post in &in_range {
                if !base_handled(post) {
                    let next = walk.sighting_index.len();
                    walk.sighting_index.entry(post.mblogid.clone()).or_insert(next);
                }
            }
        }
        let first_time: Vec<RawPost> = in_range
            .into_iter()
            .filter(|post| {
                !walk.existing_ids.contains(&post.mblogid)
                    && !walk.base_skipped.contains(&post.mblogid)
                    && !walk.run_skipped.contains(&post.mblogid)
            })
            .collect();

        // Pages that are entirely out of range are not a sign of trouble.
        if in_range_count > 0 && first_time.is_empty() {
            if walk.updating {
                if all_base_handled {
                    return Ok(PageStep::Finished(None));
                }
                return Ok(PageStep::Continue);
            }
            walk.repeated_pages += 1;
            if walk.repeated_pages >= MAX_REPEATED_PAGES {
                return Ok(PageStep::Finished(Some(format!(
                    "微博接口连续 {MAX_REPEATED_PAGES} 页只返回已保存过的微博，已提前结束"
                ))));
            }
            return Ok(PageStep::Continue);
        }
        walk.repeated_pages = 0;

        let new_posts = first_time.len();
        walk.total_fetched += new_posts;
        let mut fresh = Vec::with_capacity(first_time.len());
        for post in first_time {
            if request.ignore_deleted && post.user.is_none() {
                // Left out on purpose. Remember it, or every update comes back to it.
                walk.run_skipped.insert(post.mblogid);
                continue;
            }
            fresh.push(post);
        }
        self.process_posts(context, walk, fresh, page).await?;

        // The page is fully handled, so a checkpoint may resume after it.
        walk.last_page = page as usize;
        walk.dirty = true;
        let progress_note = if walk.updating {
            format!("已发现 {} 条新微博", walk.posts.len() - walk.base_len)
        } else {
            format!("已获取 {} 条微博", walk.total_fetched)
        };
        context.report_page(walk, page, &progress_note);
        log::debug!("page {page} handled: {new_posts} new posts, {} saved in total", walk.posts.len());

        if walk.posts_since_checkpoint >= CHECKPOINT_EVERY_POSTS
            || walk.last_checkpoint_at.elapsed() >= CHECKPOINT_EVERY
        {
            // A failed periodic write is retried on a later page; the final one reports the error.
            if walk.save_checkpoint(context).await.is_ok() {
                walk.last_checkpoint_at = Instant::now();
                walk.posts_since_checkpoint = 0;
                walk.dirty = false;
                walk.checkpoint_on_disk = Some((walk.last_page, walk.posts.len()));
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
        let original_only = matches!(request.filter, PostFilter::Original);
        let candidates: Vec<RawPost> = raw_posts
            .into_iter()
            .filter(|raw| {
                let left_out = original_only && raw.retweeted_status.is_some();
                if left_out {
                    // Reposts are dropped before any request is made for them, and an update must not
                    // take them for something new.
                    walk.run_skipped.insert(raw.mblogid.clone());
                }
                !left_out
            })
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
            let failures_before = walk.long_text_failed;
            let top_level_long_text = if needs_long_text(&raw) {
                let says_it_is_long = raw.is_long_text == Some(true);
                let text = fetch_long_text(context, walk, &raw.mblogid, says_it_is_long).await?;
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
                    let says_it_is_long = retweeted.is_long_text == Some(true);
                    let text = fetch_long_text(context, walk, &retweeted.mblogid, says_it_is_long).await?;
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
                // Nothing of this post is kept, so nothing of it was kept truncated. An update must
                // not take it for something new either, unless the length was judged on an incomplete
                // text because the full one could not be fetched: a later update gets another chance.
                let judged_on_complete_text = walk.long_text_failed == failures_before;
                walk.long_text_failed = failures_before;
                if judged_on_complete_text {
                    walk.run_skipped.insert(post.mblogid);
                }
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
    /// `(current, total)` of the walk. No total is reported, so the UI shows page and post counts
    /// instead of a percentage, when a date range makes Weibo's total meaningless or when Weibo
    /// reports none (favorites).
    fn progress(&self, walk: &WalkState) -> (usize, usize) {
        let ranged = self.request.date_range.start_timestamp.is_some()
            || self.request.date_range.end_timestamp.is_some();
        self.state.posts_exported.store(walk.posts.len(), Ordering::Relaxed);
        // How far an update has got says nothing about how much is left: it ends at the first page
        // it already has.
        if walk.updating {
            return (walk.posts.len() - walk.base_len, 0);
        }
        if ranged || walk.api_total <= 0 {
            (walk.total_fetched, 0)
        } else {
            let total = walk.api_total as usize;
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
    /// Posts the walk started from, when it is adding to an earlier backup.
    base_len: usize,
    /// This run adds to a finished backup instead of downloading from scratch.
    updating: bool,
    /// The posts of the backup being updated, and those it left out because of the rules. Together
    /// they are what that backup went through: an update stops where they fill a page.
    base_ids: HashSet<String>,
    base_skipped: HashSet<String>,
    /// Posts this run left out because of the rules, on evidence that is complete.
    run_skipped: HashSet<String>,
    /// Where in the walk each new post was first seen, for an update. A resumed update reads pages
    /// an earlier attempt already saved from, and what it finds above them is newer than they are.
    sighting_index: HashMap<String, usize>,
    empty_pages: usize,
    repeated_pages: usize,
    /// Something changed since the last checkpoint was written.
    dirty: bool,
    last_checkpoint_at: Instant,
    posts_since_checkpoint: usize,
    /// `(last_page, post count)` of the checkpoint that is really on disk, when there is one.
    checkpoint_on_disk: Option<(usize, usize)>,
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
            base_len: 0,
            updating: false,
            base_ids: HashSet::new(),
            base_skipped: HashSet::new(),
            run_skipped: HashSet::new(),
            sighting_index: HashMap::new(),
            empty_pages: 0,
            repeated_pages: 0,
            dirty: false,
            last_checkpoint_at: Instant::now(),
            posts_since_checkpoint: 0,
            checkpoint_on_disk: None,
        }
    }

    /// Works out how this run relates to what the output directory already holds: a fresh download,
    /// the continuation of an interrupted one, or an update of a finished backup.
    ///
    /// Also returns the first page to fetch and, when a finished backup was there but could not be
    /// updated, why the run starts over instead.
    async fn start<S: WeiboSource>(context: &RunContext<'_, S>) -> (Self, usize, Option<String>) {
        let request = context.request;
        let dir = &request.output_dir;
        let over = |reason: Option<String>| (Self::new(), 1, reason);

        let Ok(bundle) = cache::load_cache_bundle(dir).await else {
            return over(None);
        };

        if let Some(checkpoint) = bundle.checkpoint {
            if !request.fresh && checkpoint.schema == CHECKPOINT_SCHEMA && checkpoint_matches(&checkpoint, request)
            {
                let base_len = checkpoint.base_posts;
                let last_page = checkpoint.last_page;
                let updating = base_len > 0;
                let message = if updating {
                    format!("继续上次中断的更新（已保存 {} 条）...", bundle.posts.len())
                } else {
                    format!("从第 {} 页继续下载（已保存 {} 条）...", last_page + 1, bundle.posts.len())
                };
                context.sink.emit(ProgressPhase::Resuming, 0, checkpoint.total_fetched, &message);
                let walk = Self {
                    existing_ids: bundle.posts.iter().map(|post| post.mblogid.clone()).collect(),
                    base_ids: bundle.posts.iter().take(base_len).map(|post| post.mblogid.clone()).collect(),
                    base_skipped: checkpoint.skipped_ids.iter().cloned().collect(),
                    last_page,
                    total_fetched: checkpoint.total_fetched,
                    api_total: checkpoint.total_posts,
                    long_text_failed: checkpoint.long_text_failed,
                    resumed_from_page: Some(if updating { 1 } else { last_page + 1 }),
                    base_len,
                    updating,
                    checkpoint_on_disk: Some((last_page, bundle.posts.len())),
                    posts: bundle.posts,
                    ..Self::new()
                };
                if updating {
                    // An interrupted update starts again from the first page. The pages it already
                    // went through hold nothing left to do, while anything published since then is
                    // above them.
                    return (walk, 1, None);
                }
                // Start one page before the first unprocessed one. A post deleted on Weibo during a
                // pause moves every later post up a slot, so the neighbour of the last saved post
                // may have slipped onto the page before; reading that page again costs one request
                // and catches it.
                return (walk, last_page.max(1), None);
            }

            // A checkpoint for other settings, or one the user asked to skip, must not be picked up
            // by a later run either.
            cache::clear(dir);
            return over(None);
        }

        let Some(backup) = bundle
            .backup
            .filter(|backup| backup.uid == request.uid && backup.source_type == request.source_type)
        else {
            return over(None);
        };

        if request.fresh {
            cache::clear(dir);
            return over(None);
        }
        if let Err(reason) = update_allowed(&backup, request) {
            cache::clear(dir);
            return over(Some(reason));
        }

        context.sink.emit(
            ProgressPhase::FetchingPostList,
            0,
            0,
            &format!("已有 {} 条，正在检查新微博...", bundle.posts.len()),
        );
        let base_len = bundle.posts.len();
        let base_ids: HashSet<String> = bundle.posts.iter().map(|post| post.mblogid.clone()).collect();
        let walk = Self {
            existing_ids: base_ids.clone(),
            base_ids,
            base_skipped: bundle.skipped_ids.iter().cloned().collect(),
            base_len,
            updating: true,
            posts: bundle.posts,
            ..Self::new()
        };
        (walk, 1, None)
    }

    /// The posts the backup being updated left out, in a fixed order so that files do not change
    /// for no reason. A checkpoint keeps them apart from this run's own: an update that is resumed
    /// has to know what the backup went through, not what the interrupted run did.
    fn sorted_base_skipped(&self) -> Vec<String> {
        sorted_ids(&self.base_skipped)
    }

    /// Everything left out so far, for the finished backup.
    fn sorted_skipped(&self) -> Vec<String> {
        let mut ids = sorted_ids(&self.base_skipped);
        ids.extend(self.run_skipped.iter().cloned());
        ids.sort();
        ids
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
                base_posts: self.base_len,
                skipped_ids: self.sorted_base_skipped(),
            },
            context.export_context,
        )
        .await
    }
}

/// An update may only add to a backup made the same way, otherwise one file would hold two sets of
/// rules. The window may shrink or move on to newer posts, but not reach further back: posts older
/// than anything saved so far sit below the first page the walk already has, where it stops.
fn update_allowed(backup: &BackupMeta, request: &DownloadRequest) -> Result<(), String> {
    let oldest = |timestamp: Option<i64>| timestamp.unwrap_or(i64::MIN);
    if backup.filter != request.filter
        || backup.min_text_length != request.min_text_length
        || backup.ignore_deleted != request.ignore_deleted
    {
        return Err("筛选条件变了，已重新完整下载".to_string());
    }
    if oldest(request.date_range.start_timestamp) < oldest(backup.start_timestamp) {
        return Err("时间范围向前扩大了，已重新完整下载".to_string());
    }
    Ok(())
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
/// run; a failed request leaves this post truncated, is counted, and is reported at the end.
///
/// Weibo also answers "no long text" for posts that were only asked because their text mentions
/// "展开" or "全文". That is not a failure; it only counts when the post itself says it is long.
async fn fetch_long_text<S: WeiboSource>(
    context: &RunContext<'_, S>,
    walk: &mut WalkState,
    mblogid: &str,
    says_it_is_long: bool,
) -> Result<Option<String>, AppError> {
    match context.source.get_long_text(mblogid).await {
        Ok(content @ Some(_)) => Ok(content),
        Ok(None) if !says_it_is_long => Ok(None),
        Ok(None) => {
            record_long_text_failure(context, walk, mblogid, "微博没有返回全文");
            Ok(None)
        }
        Err(error @ (AppError::Cancelled | AppError::InvalidCookie | AppError::NotLoggedIn | AppError::Blocked(_))) => {
            Err(error)
        }
        Err(error) => {
            record_long_text_failure(context, walk, mblogid, &error.to_string());
            Ok(None)
        }
    }
}

fn record_long_text_failure<S: WeiboSource>(
    context: &RunContext<'_, S>,
    walk: &mut WalkState,
    mblogid: &str,
    reason: &str,
) {
    log::warn!("long text of {mblogid} could not be fetched: {reason}");
    // Every failure is counted; only the first few are reported one by one.
    if walk.long_text_failed < MAX_LOGGED_LONG_TEXT_FAILURES {
        let reason: String = reason.chars().take(60).collect();
        context.report_note(
            walk,
            &format!("长微博 {mblogid} 全文获取失败，已保留截断内容：{reason}"),
        );
    }
    walk.long_text_failed += 1;
}

/// For an error screen: says that the walk's progress is on disk, when it is.
fn report_progress_saved(sink: &dyn ProgressSink, walk: &WalkState) {
    if let Some(message) = saved_progress_message(walk) {
        log::info!("{message}");
        sink.emit(ProgressPhase::FetchingPostList, walk.posts.len(), 0, &message);
    }
}

fn saved_progress_message(walk: &WalkState) -> Option<String> {
    let (last_page, saved) = walk.checkpoint_on_disk?;
    // An interrupted update starts again from the first page, so a page number would be wrong.
    let how_far = if walk.dirty {
        if walk.updating {
            format!("（已获取 {saved} 条微博，之后处理的几条没能写入磁盘）")
        } else {
            format!(
                "到第 {} 页（已获取 {saved} 条微博，之后处理的几条没能写入磁盘）",
                last_page + 1
            )
        }
    } else {
        format!("（已获取 {saved} 条微博）")
    };
    let how = if walk.updating {
        "会接着检查新微博".to_string()
    } else {
        format!("会从第 {} 页接着下载", last_page + 1)
    };
    Some(format!("进度已保存{how_far}，再次点击「开始下载」{how}"))
}

fn report_cancelled(sink: &dyn ProgressSink, walk: &WalkState) {
    // `total` is 1 when a checkpoint is on disk. That is how the UI knows to offer "继续下载",
    // even when pages were walked but no post kept.
    let (message, resumable) = match saved_progress_message(walk) {
        Some(message) => (message, 1),
        // Nothing ever reached the disk. Say so when something was processed, rather than
        // implying the stop was clean.
        None if walk.dirty => ("已取消，但进度没能写入磁盘，下次需要重新下载".to_string(), 0),
        None => ("已取消".to_string(), 0),
    };
    let saved = walk.checkpoint_on_disk.map_or(0, |(_, saved)| saved);
    log::info!("{message}");
    sink.emit(ProgressPhase::Cancelled, saved, resumable, &message);
}

fn sorted_ids(ids: &HashSet<String>) -> Vec<String> {
    let mut ids: Vec<String> = ids.iter().cloned().collect();
    ids.sort();
    ids
}

fn post_time(post: &WeiboPost) -> i64 {
    parse_weibo_date_to_timestamp(&post.created_at)
}

/// Puts a profile timeline back in post-time order, newest first. A resumed walk reads one page
/// again and appends what it finds, so posts that moved onto that page would otherwise land after
/// older ones. Posts whose date cannot be parsed keep the place of their predecessor.
fn in_timeline_order(posts: Vec<WeiboPost>) -> Vec<WeiboPost> {
    let mut carried = i64::MAX;
    let mut keyed: Vec<(i64, WeiboPost)> = posts
        .into_iter()
        .map(|post| {
            let time = post_time(&post);
            if time > 0 {
                carried = time;
            }
            (carried, post)
        })
        .collect();
    keyed.sort_by_key(|(time, _)| std::cmp::Reverse(*time));
    keyed.into_iter().map(|(_, post)| post).collect()
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;
    use std::path::{Path, PathBuf};
    use std::sync::{Arc, Mutex};

    use chrono::TimeZone;

    use super::*;
    use crate::models::{DateRange, RawPostUser};

    const UID: &str = "1234567890";

    /// How the fake API fails.
    #[derive(Clone, Copy, Debug)]
    enum Failure {
        Transient,
        SessionExpired,
        LoggedOut,
        Blocked,
        Cancelled,
    }

    fn fake_error(failure: Failure) -> AppError {
        match failure {
            Failure::Transient => AppError::ApiError("模拟的临时错误".to_string()),
            Failure::SessionExpired => AppError::InvalidCookie,
            Failure::LoggedOut => AppError::NotLoggedIn,
            Failure::Blocked => AppError::Blocked("模拟的风控拒绝".to_string()),
            Failure::Cancelled => AppError::Cancelled,
        }
    }

    /// A scripted stand-in for the Weibo API: one entry per page, plus per-post long text outcomes.
    struct FakeSource {
        pages: Vec<Vec<RawPost>>,
        long_text: HashMap<String, Result<Option<String>, Failure>>,
        fail_page_with: Option<(i64, Failure)>,
        /// Presses the app's stop button just before answering this page.
        cancel_on_page: Option<(i64, Arc<AppState>)>,
        requested_pages: Mutex<Vec<i64>>,
    }

    impl FakeSource {
        fn new(pages: Vec<Vec<RawPost>>) -> Self {
            Self {
                pages,
                long_text: HashMap::new(),
                fail_page_with: None,
                cancel_on_page: None,
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
            if let Some((at, state)) = &self.cancel_on_page {
                if *at == page {
                    state.cancel();
                }
            }
            match self.pages.get(page as usize - 1) {
                Some(posts) => {
                    let total = self.pages.iter().map(Vec::len).sum::<usize>() as i64;
                    Ok((posts.clone(), total))
                }
                None => Ok((Vec::new(), 0)),
            }
        }

        async fn fetch_favorites_page(
            &self,
            page: i64,
            _starttime: Option<i64>,
            _endtime: Option<i64>,
        ) -> Result<(Vec<RawPost>, i64), AppError> {
            // Like the real client: Weibo reports no total for favorites.
            let (posts, _) = self.fetch_posts_page(UID, page).await?;
            Ok((posts, -1))
        }

        async fn get_long_text(&self, mblogid: &str) -> Result<Option<String>, AppError> {
            match self.long_text.get(mblogid) {
                Some(Ok(text)) => Ok(text.clone()),
                Some(Err(failure)) => Err(fake_error(*failure)),
                // What Weibo answers for a post that merely mentions "展开" or "全文".
                None => Ok(None),
            }
        }
    }

    /// A post from `days_before` days before 2024-06-01.
    fn post_dated(id: &str, days_before: i64) -> RawPost {
        let date =
            chrono::NaiveDate::from_ymd_opt(2024, 6, 1).unwrap() - chrono::Duration::days(days_before);
        RawPost {
            mblogid: id.to_string(),
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

    /// Posts one day apart, newest first, so date ranges and early stops can be exercised.
    fn post(index: usize) -> RawPost {
        post_dated(&format!("p{index}"), index as i64)
    }

    fn undated(id: &str) -> RawPost {
        RawPost {
            created_at: " sometime ".to_string(),
            ..post_dated(id, 0)
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
            fresh: false,
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

    async fn run_on(
        request: &DownloadRequest,
        state: &Arc<AppState>,
        source: &FakeSource,
    ) -> (Result<DownloadOutcome, AppError>, Vec<ProgressEvent>) {
        state.reset();
        let sink = VecSink::default();
        let result = DownloadService.run_with(request, state, source, &sink).await;
        let events = sink.0.lock().unwrap().clone();
        (result, events)
    }

    async fn run(
        request: &DownloadRequest,
        source: &FakeSource,
    ) -> (Result<DownloadOutcome, AppError>, Arc<AppState>, Vec<ProgressEvent>) {
        let state = Arc::new(AppState::default());
        let (result, events) = run_on(request, &state, source).await;
        (result, state, events)
    }

    fn event(events: &[ProgressEvent], phase: ProgressPhase) -> Option<&ProgressEvent> {
        events.iter().find(|event| event.phase == phase)
    }

    fn list_progress(events: &[ProgressEvent]) -> Vec<(usize, usize)> {
        events
            .iter()
            .filter(|event| matches!(event.phase, ProgressPhase::FetchingPostList))
            .map(|event| (event.current, event.total))
            .collect()
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

    #[test]
    fn timeline_order_is_newest_first_and_keeps_undated_posts_after_their_predecessor() {
        let shuffled = vec![
            undated("u"),
            post_dated("p5", 5),
            post_dated("p1", 1),
            undated("v"),
            post_dated("p3", 3),
        ]
        .into_iter()
        .map(|raw| to_weibo_post(&raw, UID))
        .collect();

        let ordered = in_timeline_order(shuffled);

        let ids: Vec<&str> = ordered.iter().map(|post| post.mblogid.as_str()).collect();
        assert_eq!(ids, ["u", "p1", "v", "p3", "p5"]);
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
        assert_eq!(
            (cancelled.current, cancelled.total),
            (6, 1),
            "saved posts, and 1 because a checkpoint is on disk and the UI may offer to continue"
        );
        assert!(cancelled.message.contains("第 3 页"), "{}", cancelled.message);

        let (result, _, events) = run(&request(&dir), &FakeSource::new(pages)).await;
        let outcome = result.unwrap();

        assert_eq!(outcome.saved, 12, "no post may be skipped between the two runs");
        assert_eq!(outcome.resumed_from_page, Some(3));
        assert!(event(&events, ProgressPhase::Resuming).is_some());
        assert!(cache_in(&dir).checkpoint.is_none(), "a finished download is not resumable");
        assert!(
            list_progress(&events).iter().any(|(current, total)| *total == 12 && *current > 0),
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
            let (result, _, events) = run(&request(&dir), &broken).await;

            match failure {
                Failure::SessionExpired => assert!(matches!(result, Err(AppError::InvalidCookie)), "{name}"),
                Failure::LoggedOut => assert!(matches!(result, Err(AppError::NotLoggedIn)), "{name}"),
                other => panic!("{other:?} is not an auth failure"),
            }
            assert_eq!(saved_ids(&dir), ["p0", "p1", "p2"], "{name}: only the finished page may be saved");
            // The error screen must tell the user that signing in again and restarting continues the download.
            assert!(
                events.iter().any(|event| event.message.contains("第 2 页接着下载")),
                "{name}: the saved progress is announced"
            );

            let restored = FakeSource {
                long_text: HashMap::from([("p3".to_string(), Ok(Some("完整的长微博全文".to_string())))]),
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
                ("p0".to_string(), Ok(Some("完整的长微博全文".to_string()))),
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
    async fn a_post_that_only_mentions_the_expand_keyword_is_not_a_failed_long_post() {
        let dir = temp_output_dir("keyword-only");
        let mut pages = timeline(1, 2);
        pages[0][0] = RawPost {
            text: "想象由此展开".to_string(),
            ..post(0)
        };
        pages[0][1] = long_post(1);
        // Weibo has no long text for either post.
        let source = FakeSource::new(pages);

        let (result, _, _) = run(&request(&dir), &source).await;
        let outcome = result.unwrap();

        assert_eq!(outcome.saved, 2);
        assert_eq!(outcome.long_text_failed, 1, "only the post that says it is long is missing something");
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
            list_progress(&events).iter().all(|(_, total)| *total == 0),
            "with a date range the API total says nothing about how many posts are in range"
        );
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[tokio::test]
    async fn favorites_show_no_percentage_because_weibo_reports_no_total() {
        let dir = temp_output_dir("favorites");
        let mut favorites = request(&dir);
        favorites.source_type = SourceType::Favorites;

        let (result, _, events) = run(&favorites, &FakeSource::new(timeline(3, 3))).await;

        assert_eq!(result.unwrap().saved, 9);
        assert!(cache_in(&dir).checkpoint.is_none());
        assert!(
            list_progress(&events).iter().all(|(_, total)| *total == 0),
            "no total means no percentage: the first page must not read as 100%"
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
            base_posts: 0,
            skipped_ids: Vec::new(),
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
        assert_eq!((cancelled.current, cancelled.total), (0, 0));
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[tokio::test]
    async fn no_resume_is_offered_when_the_checkpoint_cannot_be_written() {
        let parent = temp_output_dir("unwritable");
        // Nothing can be created below a regular file.
        std::fs::write(parent.join("not-a-directory"), "").unwrap();
        let dir = parent.join("not-a-directory").join("backup");
        let source = FakeSource {
            fail_page_with: Some((2, Failure::Cancelled)),
            ..FakeSource::new(timeline(3, 3))
        };

        let (result, _, events) = run(&request(&dir), &source).await;

        assert!(matches!(result, Err(AppError::Cancelled)));
        let cancelled = event(&events, ProgressPhase::Cancelled).expect("the stop is reported");
        assert_eq!(
            (cancelled.current, cancelled.total),
            (0, 0),
            "nothing reached the disk, so there is nothing to continue from"
        );
        assert!(cancelled.message.contains("没能写入磁盘"), "{}", cancelled.message);
        let _ = std::fs::remove_dir_all(&parent);
    }

    #[tokio::test]
    async fn a_stop_in_the_middle_of_a_page_keeps_the_posts_before_it() {
        let dir = temp_output_dir("mid-page");
        let mut pages = timeline(3, 3);
        pages[1][1] = long_post(4);
        pages[1][2] = long_post(5);
        let broken = FakeSource {
            long_text: HashMap::from([
                ("p4".to_string(), Ok(Some("p4 的全文".to_string()))),
                ("p5".to_string(), Err(Failure::SessionExpired)),
            ]),
            ..FakeSource::new(pages.clone())
        };

        let (result, _, _) = run(&request(&dir), &broken).await;
        assert!(matches!(result, Err(AppError::InvalidCookie)));
        let checkpoint = cache_in(&dir).checkpoint.expect("a checkpoint");
        assert_eq!(checkpoint.last_page, 1, "page 2 was not finished, so it does not count as handled");
        assert_eq!(saved_ids(&dir), ["p0", "p1", "p2", "p3", "p4"]);

        let restored = FakeSource {
            long_text: HashMap::from([
                ("p4".to_string(), Ok(Some("p4 的全文".to_string()))),
                ("p5".to_string(), Ok(Some("p5 的全文".to_string()))),
            ]),
            ..FakeSource::new(pages)
        };
        let (result, _, _) = run(&request(&dir), &restored).await;

        assert_eq!(result.unwrap().saved, 9);
        assert_eq!(
            saved_ids(&dir),
            ["p0", "p1", "p2", "p3", "p4", "p5", "p6", "p7", "p8"],
            "the interrupted page is walked again without duplicates"
        );
        assert_eq!(cache_in(&dir).posts[5].text, "p5 的全文");
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[tokio::test]
    async fn stopping_while_images_download_resumes_without_walking_the_timeline_again() {
        let dir = temp_output_dir("images-resume");
        let state = Arc::new(AppState::default());
        let mut pages = timeline(4, 3);
        pages[3][2] = with_image(post(11));
        let mut with_pictures = request(&dir);
        with_pictures.include_images = true;
        // The timeline is saved once the second empty page is seen, so stopping right then only
        // interrupts the images.
        let stopping = FakeSource {
            cancel_on_page: Some((6, Arc::clone(&state))),
            ..FakeSource::new(pages.clone())
        };

        let (result, events) = run_on(&with_pictures, &state, &stopping).await;

        assert!(matches!(result, Err(AppError::Cancelled)));
        let checkpoint = cache_in(&dir).checkpoint.expect("a checkpoint");
        assert_eq!((checkpoint.last_page, checkpoint.schema), (4, CHECKPOINT_SCHEMA));
        assert_eq!(saved_ids(&dir).len(), 12);
        let cancelled = event(&events, ProgressPhase::Cancelled).expect("the stop is reported");
        assert_eq!(cancelled.total, 1);

        let (result, _, events) = run(&with_pictures, &FakeSource::new(pages)).await;
        let outcome = result.unwrap();

        assert_eq!(outcome.resumed_from_page, Some(5));
        assert!(event(&events, ProgressPhase::Resuming).is_some());
        assert_eq!(outcome.image_failed, 1, "the image that could not be downloaded is still reported");
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[tokio::test]
    async fn a_post_deleted_upstream_during_a_pause_does_not_hide_its_neighbour() {
        let dir = temp_output_dir("deleted-upstream");
        let stopped = FakeSource {
            fail_page_with: Some((3, Failure::Cancelled)),
            ..FakeSource::new(timeline(4, 3))
        };
        assert!(run(&request(&dir), &stopped).await.0.is_err());

        // p0 is gone from Weibo, so every later post moved up one slot: p6 now sits on page 2.
        let remaining: Vec<RawPost> = timeline(4, 3).into_iter().flatten().skip(1).collect();
        let pages: Vec<Vec<RawPost>> = remaining.chunks(3).map(|chunk| chunk.to_vec()).collect();

        let (result, _, _) = run(&request(&dir), &FakeSource::new(pages)).await;

        assert_eq!(
            saved_ids(&dir),
            ["p0", "p1", "p2", "p3", "p4", "p5", "p6", "p7", "p8", "p9", "p10", "p11"],
            "p6 moved onto the last saved page and must not be skipped"
        );
        assert_eq!(result.unwrap().saved, 12);
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[tokio::test]
    async fn posts_published_during_a_pause_that_reach_the_overlap_stay_in_timeline_order() {
        let dir = temp_output_dir("new-upstream");
        let stopped = FakeSource {
            fail_page_with: Some((3, Failure::Cancelled)),
            ..FakeSource::new(timeline(4, 3))
        };
        assert!(run(&request(&dir), &stopped).await.0.is_err());

        // Four newer posts push the timeline down, so one of them now sits on page 2 next to p0.
        let mut later: Vec<RawPost> = (0..4)
            .map(|i| post_dated(&format!("n{i}"), i as i64 - 4))
            .collect();
        later.extend(timeline(4, 3).into_iter().flatten());
        let pages: Vec<Vec<RawPost>> = later.chunks(3).map(|chunk| chunk.to_vec()).collect();

        let (result, _, _) = run(&request(&dir), &FakeSource::new(pages)).await;

        assert_eq!(result.unwrap().saved, 13);
        assert_eq!(
            saved_ids(&dir),
            ["n3", "p0", "p1", "p2", "p3", "p4", "p5", "p6", "p7", "p8", "p9", "p10", "p11"],
            "the newer post is found on the overlap page and sorted to where it belongs"
        );
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[tokio::test]
    async fn a_long_post_dropped_by_the_length_filter_is_not_reported_as_truncated() {
        let dir = temp_output_dir("dropped-long-post");
        let mut pages = timeline(1, 1);
        pages[0][0] = long_post(0);
        let mut strict = request(&dir);
        strict.min_text_length = 50;
        let source = FakeSource {
            long_text: HashMap::from([("p0".to_string(), Err(Failure::Transient))]),
            ..FakeSource::new(pages)
        };

        let (result, _, _) = run(&strict, &source).await;
        let outcome = result.unwrap();

        assert_eq!(outcome.saved, 0);
        assert_eq!(outcome.long_text_failed, 0, "nothing of this post was kept, so nothing was kept truncated");
    }

    #[tokio::test]
    async fn a_refused_request_stops_the_walk_and_keeps_what_was_saved() {
        let dir = temp_output_dir("blocked");
        let source = FakeSource {
            fail_page_with: Some((2, Failure::Blocked)),
            ..FakeSource::new(timeline(3, 3))
        };

        let (result, _, events) = run(&request(&dir), &source).await;

        assert!(matches!(result, Err(AppError::Blocked(_))));
        let checkpoint = cache_in(&dir).checkpoint.expect("the finished page is kept for a later run");
        assert_eq!(checkpoint.last_page, 1);
        assert_eq!(saved_ids(&dir), ["p0", "p1", "p2"]);
        assert!(
            events.iter().any(|event| event.message.contains("第 2 页接着下载")),
            "the error screen learns that starting again continues the download"
        );
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[tokio::test]
    async fn a_refused_long_text_request_stops_the_walk_instead_of_leaving_the_post_truncated() {
        let dir = temp_output_dir("blocked-long-text");
        let mut pages = timeline(1, 1);
        pages[0][0] = long_post(0);
        let source = FakeSource {
            long_text: HashMap::from([("p0".to_string(), Err(Failure::Blocked))]),
            ..FakeSource::new(pages)
        };

        let (result, _, _) = run(&request(&dir), &source).await;

        assert!(matches!(result, Err(AppError::Blocked(_))));
        assert!(
            cache::load_cache_bundle_sync(&dir.to_string_lossy()).is_err(),
            "nothing was saved, so no post is left truncated"
        );
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[tokio::test]
    async fn an_error_after_the_timeline_is_saved_still_says_the_progress_is_on_disk() {
        let dir = temp_output_dir("images-dir-blocked");
        // A regular file where the images directory should go makes the image phase fail.
        std::fs::write(dir.join("images"), "").unwrap();
        let mut pages = timeline(1, 1);
        pages[0][0] = with_image(post(0));
        let mut with_pictures = request(&dir);
        with_pictures.include_images = true;

        let (result, _, events) = run(&with_pictures, &FakeSource::new(pages)).await;

        assert!(result.is_err());
        assert!(cache_in(&dir).checkpoint.is_some(), "the timeline itself was saved");
        assert!(
            events.iter().any(|event| event.message.contains("第 2 页接着下载")),
            "the error screen learns that starting again continues the download"
        );
        let _ = std::fs::remove_dir_all(&dir);
    }

    /// A post published after an earlier backup. `index` 0 is the newest.
    fn newer(index: usize, count: usize) -> RawPost {
        post_dated(&format!("n{index}"), index as i64 - count as i64)
    }

    fn pages_of(posts: Vec<RawPost>, per_page: usize) -> Vec<Vec<RawPost>> {
        posts.chunks(per_page).map(|chunk| chunk.to_vec()).collect()
    }

    fn finished_backup(dir: &Path) -> BackupMeta {
        cache_in(dir).backup.expect("the cache describes a finished backup")
    }

    #[test]
    fn an_update_may_only_add_to_a_backup_made_the_same_way() {
        let backup = BackupMeta {
            uid: UID.to_string(),
            source_type: SourceType::Profile,
            post_count: 0,
            author: String::new(),
            filter: PostFilter::All,
            min_text_length: 20,
            ignore_deleted: true,
            start_timestamp: Some(100),
            end_timestamp: Some(200),
        };
        let window = |start: Option<i64>, end: Option<i64>| DateRange {
            start_timestamp: start,
            end_timestamp: end,
        };

        // Same rules, and a window that moved on or shrank: the new posts are all newer than what is saved.
        let mut same = request(Path::new("/unused"));
        same.min_text_length = 20;
        same.ignore_deleted = true;
        same.date_range = window(Some(150), Some(300));
        assert_eq!(update_allowed(&backup, &same), Ok(()));
        same.date_range = window(Some(100), None);
        assert_eq!(update_allowed(&backup, &same), Ok(()));

        // Another set of content rules would leave two kinds of backup in one file.
        let mut other_rules = same.clone();
        other_rules.min_text_length = 0;
        assert_eq!(update_allowed(&backup, &other_rules), Err("筛选条件变了，已重新完整下载".to_string()));
        other_rules = same.clone();
        other_rules.filter = PostFilter::Original;
        assert_eq!(update_allowed(&backup, &other_rules), Err("筛选条件变了，已重新完整下载".to_string()));

        // Posts older than anything saved so far sit below the first page the walk already has.
        let mut further_back = same.clone();
        further_back.date_range = window(None, Some(300));
        assert_eq!(update_allowed(&backup, &further_back), Err("时间范围向前扩大了，已重新完整下载".to_string()));
        further_back.date_range = window(Some(50), Some(300));
        assert_eq!(update_allowed(&backup, &further_back), Err("时间范围向前扩大了，已重新完整下载".to_string()));
    }

    #[tokio::test]
    async fn an_update_adds_new_posts_in_front_and_leaves_the_rest_alone() {
        let dir = temp_output_dir("update");
        let (result, _, _) = run(&request(&dir), &FakeSource::new(timeline(3, 2))).await;
        assert_eq!(result.unwrap().added, None, "a first download is not an update");

        // Two newer posts push everything else one page down.
        let mut later: Vec<RawPost> = (0..2).map(|i| newer(i, 2)).collect();
        later.extend(timeline(3, 2).into_iter().flatten());
        let source = FakeSource::new(pages_of(later, 2));

        let (result, _, _) = run(&request(&dir), &source).await;
        let outcome = result.unwrap();

        assert_eq!((outcome.added, outcome.unchanged, outcome.saved), (Some(2), 6, 8));
        assert_eq!(
            (outcome.resumed_from_page, outcome.stopped_early.as_deref(), outcome.started_over.as_deref()),
            (None, None, None)
        );
        assert_eq!(saved_ids(&dir), ["n0", "n1", "p0", "p1", "p2", "p3", "p4", "p5"]);
        assert_eq!(source.requested_pages(), [1, 2], "stops on the first page it already has");
        let bundle = cache_in(&dir);
        assert!(bundle.checkpoint.is_none());
        assert_eq!(bundle.backup.map(|backup| backup.post_count), Some(8));
        assert_eq!(
            cache::inspect_backup(&dir.to_string_lossy()).map(|backup| backup.post_count),
            Some(8)
        );
        assert_eq!(outcome.summary(), "更新完成！新增 2 条微博，共 8 条");
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[tokio::test]
    async fn an_update_with_nothing_new_reads_one_page_and_changes_nothing() {
        let dir = temp_output_dir("update-nothing");
        let pages = timeline(3, 2);
        run(&request(&dir), &FakeSource::new(pages.clone())).await.0.unwrap();

        let source = FakeSource::new(pages);
        let (result, _, _) = run(&request(&dir), &source).await;
        let outcome = result.unwrap();

        assert_eq!((outcome.added, outcome.saved), (Some(0), 6));
        assert_eq!(source.requested_pages(), [1]);
        assert_eq!(outcome.summary(), "更新完成！新增 0 条微博，共 6 条");
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[tokio::test]
    async fn changed_rules_start_over_instead_of_mixing_two_sets_of_posts() {
        let dir = temp_output_dir("update-rules");
        let pages = timeline(2, 2);
        run(&request(&dir), &FakeSource::new(pages.clone())).await.0.unwrap();

        let mut stricter = request(&dir);
        stricter.min_text_length = 50;
        let source = FakeSource::new(pages);
        let (result, _, _) = run(&stricter, &source).await;
        let outcome = result.unwrap();

        assert_eq!(outcome.added, None, "a fresh download, not an update");
        assert_eq!(outcome.started_over.as_deref(), Some("筛选条件变了，已重新完整下载"));
        assert_eq!(outcome.saved, 0, "no post is long enough for the new rule");
        assert_eq!(source.requested_pages(), [1, 2, 3, 4], "the whole timeline is walked again");
        assert_eq!(finished_backup(&dir).min_text_length, 50, "the backup now records the new rule");
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[tokio::test]
    async fn a_window_reaching_further_back_starts_over_but_a_narrower_one_updates() {
        let dir = temp_output_dir("update-window");
        // p0 was posted on 2024-06-01 and p5 five days earlier; this window keeps p0 to p3.
        let mut from_day_3 = request(&dir);
        from_day_3.date_range.start_timestamp = Some(beijing_timestamp(2024, 5, 29, 0, 0, 0));

        run(&from_day_3, &FakeSource::new(timeline(3, 2))).await.0.unwrap();
        let (result, _, _) = run(&from_day_3, &FakeSource::new(timeline(3, 2))).await;
        assert_eq!(result.unwrap().added, Some(0), "the same window updates");

        let (result, _, _) = run(&request(&dir), &FakeSource::new(timeline(3, 2))).await;
        let outcome = result.unwrap();
        assert_eq!(outcome.added, None);
        assert_eq!(outcome.started_over.as_deref(), Some("时间范围向前扩大了，已重新完整下载"));
        assert_eq!(outcome.saved, 6, "the two older posts are fetched as well");

        let (result, _, _) = run(&from_day_3, &FakeSource::new(timeline(3, 2))).await;
        let outcome = result.unwrap();
        assert_eq!((outcome.added, outcome.saved), (Some(0), 6), "a narrower window keeps what was saved before");
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[tokio::test]
    async fn asking_for_a_fresh_download_ignores_the_existing_backup() {
        let dir = temp_output_dir("update-fresh");
        run(&request(&dir), &FakeSource::new(timeline(3, 2))).await.0.unwrap();

        let mut fresh = request(&dir);
        fresh.fresh = true;
        let source = FakeSource::new(timeline(3, 2));
        let (result, _, _) = run(&fresh, &source).await;

        assert_eq!(result.unwrap().added, None);
        assert_eq!(source.requested_pages(), [1, 2, 3, 4, 5], "the whole timeline, then two empty pages");
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[tokio::test]
    async fn an_interrupted_update_resumes_as_an_update() {
        let dir = temp_output_dir("update-resume");
        run(&request(&dir), &FakeSource::new(timeline(3, 2))).await.0.unwrap();

        let mut later: Vec<RawPost> = (0..4).map(|i| newer(i, 4)).collect();
        later.extend(timeline(3, 2).into_iter().flatten());
        let pages = pages_of(later, 2);
        let stopping = FakeSource {
            fail_page_with: Some((3, Failure::Cancelled)),
            ..FakeSource::new(pages.clone())
        };
        let (result, _, _) = run(&request(&dir), &stopping).await;
        assert!(matches!(result, Err(AppError::Cancelled)));
        let checkpoint = cache_in(&dir).checkpoint.expect("a checkpoint");
        assert_eq!((checkpoint.last_page, checkpoint.base_posts), (2, 6));
        assert!(
            cache::inspect_backup(&dir.to_string_lossy()).is_none(),
            "an interrupted run is not a finished backup any more"
        );

        let (result, _, _) = run(&request(&dir), &FakeSource::new(pages)).await;
        let outcome = result.unwrap();

        assert_eq!((outcome.added, outcome.unchanged, outcome.saved), (Some(4), 6, 10));
        assert_eq!(outcome.resumed_from_page, Some(1), "an interrupted update starts again from the first page");
        assert_eq!(
            saved_ids(&dir),
            ["n0", "n1", "n2", "n3", "p0", "p1", "p2", "p3", "p4", "p5"],
            "the new posts end up in front of the ones the update started from"
        );
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[tokio::test]
    async fn a_backup_of_another_account_is_replaced_rather_than_updated() {
        let dir = temp_output_dir("update-other");
        run(&request(&dir), &FakeSource::new(timeline(2, 2))).await.0.unwrap();

        let mut other = request(&dir);
        other.uid = "9999999999".to_string();
        let (result, _, _) = run(&other, &FakeSource::new(timeline(3, 2))).await;
        let outcome = result.unwrap();

        assert_eq!((outcome.added, outcome.started_over.as_deref()), (None, None));
        assert_eq!(outcome.saved, 6);
        assert_eq!(finished_backup(&dir).uid, "9999999999");
        let _ = std::fs::remove_dir_all(&dir);
    }

    fn repost(id: &str, index: usize) -> RawPost {
        RawPost {
            retweeted_status: Some(Box::new(post_dated(id, index as i64 + 100))),
            ..post(index)
        }
    }

    #[tokio::test]
    async fn resuming_an_interrupted_update_still_reads_the_pages_it_never_got_to() {
        // Six new posts sit on three pages in front of the six that are already saved.
        let mut later: Vec<RawPost> = (0..6).map(|i| newer(i, 6)).collect();
        later.extend(timeline(3, 2).into_iter().flatten());
        let pages = pages_of(later, 2);
        let everyone = ["n0", "n1", "n2", "n3", "n4", "n5", "p0", "p1", "p2", "p3", "p4", "p5"];

        // Refused while fetching page 2, 3 or 4: the walk had got through 1, 2 or 3 pages of new posts.
        for refused_at in [2, 3, 4] {
            let expected_pages = vec![1, 2, 3, 4];
            let dir = temp_output_dir(&format!("update-resume-{refused_at}"));
            run(&request(&dir), &FakeSource::new(timeline(3, 2))).await.0.unwrap();

            let stopping = FakeSource {
                fail_page_with: Some((refused_at, Failure::Blocked)),
                ..FakeSource::new(pages.clone())
            };
            assert!(run(&request(&dir), &stopping).await.0.is_err(), "refused on page {refused_at}");

            let source = FakeSource::new(pages.clone());
            let (result, _, _) = run(&request(&dir), &source).await;
            let outcome = result.unwrap();

            assert_eq!(outcome.added, Some(6), "refused on page {refused_at}: no new post may be left unread");
            assert_eq!(outcome.saved, 12, "refused on page {refused_at}");
            assert_eq!(source.requested_pages(), expected_pages, "refused on page {refused_at}");
            assert_eq!(saved_ids(&dir), everyone, "refused on page {refused_at}");
            let _ = std::fs::remove_dir_all(&dir);
        }
    }

    #[tokio::test]
    async fn an_update_of_a_backup_made_with_a_filter_stops_at_posts_it_has_already_judged() {
        let dir = temp_output_dir("update-filtered");
        let original_only = DownloadRequest {
            filter: PostFilter::Original,
            ..request(&dir)
        };

        // Every other post is a repost, which the "original only" rule leaves out.
        let mut pages = timeline(3, 2);
        pages[0][1] = repost("r1", 1);
        pages[1][1] = repost("r3", 3);
        pages[2][1] = repost("r5", 5);
        let (result, _, _) = run(&original_only, &FakeSource::new(pages.clone())).await;
        assert_eq!(result.unwrap().saved, 3);

        // One new original post, in front of a repost that was skipped the first time.
        let mut later = vec![newer(0, 1)];
        later.extend(pages.into_iter().flatten());
        let source = FakeSource::new(pages_of(later, 2));
        let (result, _, _) = run(&original_only, &source).await;
        let outcome = result.unwrap();

        assert_eq!((outcome.added, outcome.saved), (Some(1), 4));
        assert_eq!(
            source.requested_pages(),
            [1, 2],
            "a repost the rule skipped last time is nothing new: the whole timeline need not be walked"
        );
        assert_eq!(saved_ids(&dir), ["n0", "p0", "p2", "p4"]);
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[tokio::test]
    async fn an_update_of_a_backup_that_was_allowed_to_keep_nothing_reads_one_page() {
        let dir = temp_output_dir("update-empty-rules");
        let strict = DownloadRequest {
            min_text_length: 50,
            ..request(&dir)
        };
        let pages = timeline(3, 2);
        assert_eq!(run(&strict, &FakeSource::new(pages.clone())).await.0.unwrap().saved, 0);

        let source = FakeSource::new(pages);
        let (result, _, _) = run(&strict, &source).await;

        assert_eq!(result.unwrap().added, Some(0));
        assert_eq!(source.requested_pages(), [1], "every post was judged too short the first time");
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[tokio::test]
    async fn a_resumed_update_picks_up_what_was_published_during_the_pause() {
        let dir = temp_output_dir("update-published-meanwhile");
        run(&request(&dir), &FakeSource::new(timeline(3, 2))).await.0.unwrap();

        let mut later: Vec<RawPost> = (0..4).map(|i| newer(i, 4)).collect();
        later.extend(timeline(3, 2).into_iter().flatten());
        let pages = pages_of(later.clone(), 2);
        let stopping = FakeSource {
            fail_page_with: Some((3, Failure::Blocked)),
            ..FakeSource::new(pages.clone())
        };
        assert!(run(&request(&dir), &stopping).await.0.is_err());

        // The blogger publishes once more before the user comes back.
        let mut meanwhile = vec![post_dated("m0", -5)];
        meanwhile.extend(later);
        let source = FakeSource::new(pages_of(meanwhile, 2));
        let (result, _, _) = run(&request(&dir), &source).await;
        let outcome = result.unwrap();

        assert_eq!(outcome.added, Some(5), "the post published during the pause is new as well");
        assert_eq!(outcome.saved, 11);
        assert_eq!(saved_ids(&dir)[0], "m0");
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[tokio::test]
    async fn a_resumed_update_remembers_what_the_backup_had_left_out() {
        let dir = temp_output_dir("update-resume-skipped");
        let original_only = DownloadRequest {
            filter: PostFilter::Original,
            ..request(&dir)
        };
        let mut base = timeline(3, 2);
        base[0][1] = repost("r1", 1);
        base[1][1] = repost("r3", 3);
        base[2][1] = repost("r5", 5);
        run(&original_only, &FakeSource::new(base.clone())).await.0.unwrap();

        let mut later: Vec<RawPost> = (0..4).map(|i| newer(i, 4)).collect();
        later.extend(base.into_iter().flatten());
        let pages = pages_of(later, 2);
        let stopping = FakeSource {
            fail_page_with: Some((3, Failure::Blocked)),
            ..FakeSource::new(pages.clone())
        };
        assert!(run(&original_only, &stopping).await.0.is_err());

        let source = FakeSource::new(pages);
        let (result, _, _) = run(&original_only, &source).await;
        let outcome = result.unwrap();

        assert_eq!((outcome.added, outcome.saved), (Some(4), 7));
        assert_eq!(
            source.requested_pages(),
            [1, 2, 3],
            "the repost the backup left out is still known once the update is resumed"
        );
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[tokio::test]
    async fn a_post_dropped_because_its_full_text_could_not_be_fetched_is_tried_again_by_a_later_update() {
        let dir = temp_output_dir("update-incomplete-evidence");
        let strict = DownloadRequest {
            min_text_length: 50,
            ..request(&dir)
        };
        let mut pages = timeline(1, 1);
        pages[0][0] = long_post(0);
        let first_try = FakeSource {
            long_text: HashMap::from([("p0".to_string(), Err(Failure::Transient))]),
            ..FakeSource::new(pages.clone())
        };
        let (result, _, _) = run(&strict, &first_try).await;
        assert_eq!(result.unwrap().saved, 0, "the truncated text is too short to keep");

        let full_text = "这是一段足够长的完整微博正文，用来确认更新时会把上次没取到全文的微博重新处理一次，而不是永远跳过它，这一点必须确认到位。";
        let second_try = FakeSource {
            long_text: HashMap::from([("p0".to_string(), Ok(Some(full_text.to_string())))]),
            ..FakeSource::new(pages)
        };
        let (result, _, _) = run(&strict, &second_try).await;

        assert_eq!(result.unwrap().added, Some(1), "nothing is final while the evidence was incomplete");
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[tokio::test]
    async fn a_stopped_update_does_not_promise_a_page_number_it_will_not_start_from() {
        let dir = temp_output_dir("update-stop-wording");
        run(&request(&dir), &FakeSource::new(timeline(3, 2))).await.0.unwrap();

        let mut later: Vec<RawPost> = (0..4).map(|i| newer(i, 4)).collect();
        later.extend(timeline(3, 2).into_iter().flatten());
        let stopping = FakeSource {
            fail_page_with: Some((2, Failure::Cancelled)),
            ..FakeSource::new(pages_of(later, 2))
        };

        let (result, _, events) = run(&request(&dir), &stopping).await;

        assert!(matches!(result, Err(AppError::Cancelled)));
        let cancelled = event(&events, ProgressPhase::Cancelled).expect("the stop is reported");
        assert!(cancelled.message.contains("接着检查新微博"), "{}", cancelled.message);
        assert!(!cancelled.message.contains("从第"), "the next run starts from the first page: {}", cancelled.message);
    }

    #[tokio::test]
    async fn a_resumed_update_keeps_the_order_in_which_new_posts_appear() {
        let dir = temp_output_dir("update-resume-order");
        let mut favourites = request(&dir);
        favourites.source_type = SourceType::Favorites;
        run(&favourites, &FakeSource::new(timeline(3, 2))).await.0.unwrap();

        let mut later: Vec<RawPost> = (0..4).map(|i| newer(i, 4)).collect();
        later.extend(timeline(3, 2).into_iter().flatten());
        let stopping = FakeSource {
            fail_page_with: Some((3, Failure::Blocked)),
            ..FakeSource::new(pages_of(later.clone(), 2))
        };
        assert!(run(&favourites, &stopping).await.0.is_err());

        // Something else is favoured before the user comes back. It is the newest of all, and it
        // has to end up in front of what the interrupted attempt saved.
        let mut meanwhile = vec![post_dated("m0", -5)];
        meanwhile.extend(later);
        let (result, _, _) = run(&favourites, &FakeSource::new(pages_of(meanwhile, 2))).await;

        assert_eq!(result.unwrap().saved, 11);
        assert_eq!(
            saved_ids(&dir),
            ["m0", "n0", "n1", "n2", "n3", "p0", "p1", "p2", "p3", "p4", "p5"],
            "new posts stay in the order the walk saw them, newest first"
        );
        let _ = std::fs::remove_dir_all(&dir);
    }
}
