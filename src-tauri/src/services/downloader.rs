use std::path::PathBuf;
use std::sync::atomic::Ordering;
use std::{collections::{HashMap, HashSet}, path::Path};

use chrono::{TimeZone, Utc};
use tauri::{AppHandle, Emitter, Runtime};

use crate::error::AppError;
use crate::models::{
    CheckpointMeta, DownloadRequest, ExportContext, PostFilter, ProgressEvent, ProgressPhase,
    RawPost, WeiboPost,
};
use crate::services::{
    file_naming::{dedupe_filenames, post_filename},
    image_store::ImageStoreService,
    weibo_api::WeiboApiClient,
};
use crate::state::AppState;
use crate::utils::html::count_plain_text_chars;

#[derive(Debug, Default)]
pub struct DownloadService;

fn needs_long_text(raw: &RawPost) -> bool {
    raw.is_long_text == Some(true) || raw.text.contains("展开") || raw.text.contains("全文")
}

pub fn normalize_raw_post_for_export(
    mut raw: RawPost,
    top_level_long_text: Option<String>,
    retweeted_long_text: Option<String>,
) -> RawPost {
    if let Some(text) = top_level_long_text.filter(|text| !text.is_empty()) {
        raw.text = text;
    }

    if let Some(retweeted) = raw.retweeted_status.as_mut() {
        if let Some(text) = retweeted_long_text.filter(|text| !text.is_empty()) {
            retweeted.text = text;
        }
    }

    raw
}

impl DownloadService {
    pub fn new() -> Self {
        Self
    }

    pub async fn run<R: Runtime>(
        &self,
        request: &DownloadRequest,
        state: &AppState,
        app: &AppHandle<R>,
    ) -> Result<(), AppError> {
        state.reset();

        let result = self.run_inner(request, state, app).await;

        if let Err(error) = &result {
            if !matches!(error, AppError::Cancelled) {
                self.emit(
                    app,
                    ProgressPhase::Error,
                    0,
                    0,
                    &format!("下载失败: {error}"),
                );
            }
        }

        result
    }

    async fn run_inner<R: Runtime>(
        &self,
        request: &DownloadRequest,
        state: &AppState,
        app: &AppHandle<R>,
    ) -> Result<(), AppError> {
        self.emit(app, ProgressPhase::FetchingUserInfo, 0, 1, "正在获取用户信息...");

        let client = WeiboApiClient::new(app.clone());
        let user = client.get_user_info(&request.uid).await?;

        let mut resumed_posts: Vec<WeiboPost> = Vec::new();
        let mut resumed_total_fetched = 0_usize;
        let start_page = match crate::services::cache::load_cache_bundle_sync(&request.output_dir) {
            Ok(bundle) if bundle.checkpoint.as_ref().is_some_and(|c| c.uid == request.uid) => {
                let cp = bundle.checkpoint.expect("checkpoint should exist");
                resumed_posts = bundle.posts;
                resumed_total_fetched = cp.total_fetched;
                state.posts_fetched.store(cp.total_fetched, Ordering::Relaxed);
                state.posts_exported
                    .store(resumed_posts.len(), Ordering::Relaxed);
                state.total_posts.store(cp.total_posts.max(0) as usize, Ordering::Relaxed);
                state.current_page.store(cp.last_page, Ordering::Relaxed);
                self.emit(
                    app,
                    ProgressPhase::Resuming,
                    0,
                    cp.total_fetched,
                    &format!(
                        "从第 {} 页恢复下载（已有 {} 条）...",
                        cp.last_page + 1,
                        resumed_posts.len()
                    ),
                );
                cp.last_page + 1
            }
            _ => 1,
        };

        self.emit(
            app,
            ProgressPhase::FetchingUserInfo,
            1,
            1,
            &format!("用户: {}", user.screen_name),
        );

        self.ensure_not_cancelled(state, app)?;

        self.emit(app, ProgressPhase::FetchingPostList, 0, 0, "正在获取微博列表...");

        let starttime = request.date_range.start_timestamp;
        let endtime = request.date_range.end_timestamp;

        let existing_ids: HashSet<String> = resumed_posts.iter().map(|p| p.mblogid.clone()).collect();
        let mut all_raw_posts: Vec<RawPost> = Vec::new();
        let mut total_posts = 0_i64;
        let mut page = start_page as i64;
        let mut last_fetched_page = start_page.saturating_sub(1);

        loop {
            self.ensure_not_cancelled(state, app)?;

            let (posts, total) = client
                .fetch_posts_page(&request.uid, page, starttime, endtime)
                .await?;

            if total_posts == 0 {
                total_posts = total;
                state.total_posts.store(total.max(0) as usize, Ordering::Relaxed);

                let page_size = posts.len().max(1);
                let total_pages = if total <= 0 {
                    0
                } else {
                    ((total as usize) + page_size - 1) / page_size
                };
                state.total_pages.store(total_pages, Ordering::Relaxed);
            }

            if posts.is_empty() {
                break;
            }

            let new_posts: Vec<RawPost> = posts
                .into_iter()
                .filter(|post| !existing_ids.contains(&post.mblogid))
                .collect();

            all_raw_posts.extend(new_posts);
            last_fetched_page = page as usize;
            state
                .posts_fetched
                .store(resumed_total_fetched + all_raw_posts.len(), Ordering::Relaxed);
            state.current_page.store(page as usize, Ordering::Relaxed);

            self.emit(
                app,
                ProgressPhase::FetchingPostList,
                resumed_total_fetched + all_raw_posts.len(),
                total_posts.max((resumed_total_fetched + all_raw_posts.len()) as i64) as usize,
                &format!("已获取 {} 条微博...", resumed_total_fetched + all_raw_posts.len()),
            );

            page += 1;
        }

        self.emit(
            app,
            ProgressPhase::FetchingLongText,
            0,
            all_raw_posts.len(),
            "正在获取长文内容...",
        );

        let export_context = ExportContext {
            date_range_label: format_date_range_label(
                request.date_range.start_timestamp,
                request.date_range.end_timestamp,
            ),
        };

        let mut normalized_posts = resumed_posts.clone();
        normalized_posts.reserve(all_raw_posts.len());
        for (index, raw) in all_raw_posts.iter().enumerate() {
            self.ensure_not_cancelled(state, app)?;

            let top_level_long_text = if needs_long_text(raw) {
                match client.get_long_text(&raw.mblogid).await {
                    Ok(content) => Some(content),
                    Err(e) => {
                        eprintln!("[longtext] 顶层长文获取失败 mblogid={}: {}", raw.mblogid, e);
                        None
                    }
                }
            } else {
                None
            };

            let retweeted_long_text = match raw.retweeted_status.as_deref() {
                Some(retweeted) if needs_long_text(retweeted) => match client.get_long_text(&retweeted.mblogid).await {
                    Ok(content) => Some(content),
                    Err(e) => {
                        eprintln!("[longtext] 转发长文获取失败 mblogid={}: {}", retweeted.mblogid, e);
                        None
                    }
                },
                _ => None,
            };

            let normalized_raw = normalize_raw_post_for_export(
                raw.clone(),
                top_level_long_text,
                retweeted_long_text,
            );

            let post = WeiboApiClient::<R>::normalize_post(&normalized_raw, &request.uid);

            match request.filter {
                PostFilter::Original if post.is_repost => continue,
                PostFilter::Original | PostFilter::All => {}
            }

            if count_plain_text_chars(&post.text) < request.min_text_length {
                continue;
            }

            normalized_posts.push(post);

            state.posts_exported.store(normalized_posts.len(), Ordering::Relaxed);

            if normalized_posts.len() % 10 == 0 {
                let _ = crate::services::cache::save_cache_checkpoint(
                    &normalized_posts,
                    &request.output_dir,
                    CheckpointMeta {
                        uid: request.uid.clone(),
                        last_page: last_fetched_page,
                        total_fetched: resumed_total_fetched + all_raw_posts.len(),
                        total_posts,
                    },
                    &export_context,
                )
                .await;
            }

            self.emit(app, ProgressPhase::FetchingLongText, 0, 1, "正在处理长文内容...");
            self.emit(
                app,
                ProgressPhase::FetchingLongText,
                index + 1,
                all_raw_posts.len(),
                &format!("已处理 {}/{} 条...", index + 1, all_raw_posts.len()),
            );
        }

        let _ = crate::services::cache::save_cache_checkpoint(
            &normalized_posts,
            &request.output_dir,
            CheckpointMeta {
                uid: request.uid.clone(),
                last_page: last_fetched_page,
                total_fetched: resumed_total_fetched + all_raw_posts.len(),
                total_posts,
            },
            &export_context,
        )
        .await;
        let mut seen = HashSet::new();
        normalized_posts.retain(|p| seen.insert(p.mblogid.clone()));
        state.posts_exported.store(normalized_posts.len(), Ordering::Relaxed);

        if request.include_images {
            self.download_images(request, &mut normalized_posts, state, app)
                .await?;
        }

        crate::services::cache::save_cache_bundle(
            &normalized_posts,
            &request.output_dir,
            export_context,
        )
        .await?;

        Ok(())
    }

    async fn download_images<R: Runtime>(
        &self,
        request: &DownloadRequest,
        posts: &mut [WeiboPost],
        state: &AppState,
        app: &AppHandle<R>,
    ) -> Result<(), AppError> {
        self.emit(app, ProgressPhase::DownloadingImages, 0, 0, "正在下载图片...");

        let image_dir = PathBuf::from(&request.output_dir).join("images");
        std::fs::create_dir_all(&image_dir)?;

        let total_images: usize = posts.iter().map(|post| post.images.len()).sum();
        if total_images == 0 {
            self.emit(app, ProgressPhase::DownloadingImages, 0, 0, "没有图片需要下载");
            return Ok(());
        }

        let image_store = ImageStoreService::new();
        let client = &state.http_client;
        let mut downloaded = 0_usize;

        for post in posts.iter_mut() {
            self.ensure_not_cancelled(state, app)?;

            let desired_filenames = dedupe_filenames(
                &post
                    .images
                    .iter()
                    .map(|image| post_filename(&post.created_at, &post.mblogid, &image_extension(&image.original_url)))
                    .collect::<Vec<_>>(),
            );

            let urls: Vec<(String, String)> = post
                .images
                .iter()
                .zip(desired_filenames.iter())
                .map(|(image, filename)| (image.original_url.clone(), filename.clone()))
                .collect();

            let stored_paths = match image_store.download_images(client, &urls, &image_dir).await {
                Ok(paths) => paths,
                Err(error) => {
                    eprintln!("Failed to download images for post {}: {}", post.mblogid, error);
                    Vec::new()
                }
            };

            let stored_paths: HashMap<String, String> = stored_paths.into_iter().collect();

            for image in post.images.iter_mut() {
                image.local_path = stored_paths
                    .get(&image.original_url)
                    .map(|filename| format!("images/{filename}"));

                if stored_paths.contains_key(&image.original_url) {
                    downloaded += 1;
                }

                self.emit(
                    app,
                    ProgressPhase::DownloadingImages,
                    downloaded,
                    total_images,
                    &format!("已下载 {}/{} 张图片...", downloaded, total_images),
                );
            }
        }

        Ok(())
    }

    fn ensure_not_cancelled<R: Runtime>(&self, state: &AppState, app: &AppHandle<R>) -> Result<(), AppError> {
        if state.is_cancelled() {
            self.emit(app, ProgressPhase::Cancelled, 0, 0, "已取消");
            return Err(AppError::Cancelled);
        }

        Ok(())
    }

    fn emit<R: Runtime>(
        &self,
        app: &AppHandle<R>,
        phase: ProgressPhase,
        current: usize,
        total: usize,
        message: &str,
    ) {
        let _ = app.emit(
            "download-progress",
            ProgressEvent::new(phase, current, total, message),
        );
    }
}

fn image_extension(url: &str) -> String {
    let cleaned = url.split(['?', '#']).next().unwrap_or(url);

    Path::new(cleaned)
        .extension()
        .and_then(|ext| ext.to_str())
        .filter(|ext| !ext.is_empty())
        .map(|ext| ext.to_ascii_lowercase())
        .unwrap_or_else(|| "jpg".to_string())
}

fn format_date_range_label(start_timestamp: Option<i64>, end_timestamp: Option<i64>) -> String {
    match (
        start_timestamp.and_then(timestamp_to_local_date),
        end_timestamp.and_then(timestamp_to_local_date),
    ) {
        (Some(start), Some(end)) => format!("{start}至{end}"),
        _ => "全部时间".to_string(),
    }
}

fn timestamp_to_local_date(timestamp: i64) -> Option<String> {
    let offset = chrono::FixedOffset::east_opt(8 * 3600)?;
    Some(
        Utc.timestamp_opt(timestamp, 0)
            .single()?
            .with_timezone(&offset)
            .format("%Y-%m-%d")
            .to_string(),
    )
}
