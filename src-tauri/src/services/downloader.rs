use std::path::PathBuf;
use std::sync::atomic::Ordering;
use std::{collections::HashMap, path::Path};

use tauri::{AppHandle, Emitter};

use crate::error::AppError;
use crate::models::{
    DownloadRequest, ExportFormat, PostFilter, ProgressEvent, ProgressPhase, WeiboPost,
};
use crate::services::{
    export_markdown::MarkdownExportService,
    export_pdf::PdfExportService,
    file_naming::{dedupe_filenames, post_filename},
    image_store::ImageStoreService,
    weibo_api::WeiboApiClient,
};
use crate::state::AppState;

#[derive(Debug, Default)]
pub struct DownloadService;

impl DownloadService {
    pub fn new() -> Self {
        Self
    }

    pub async fn run(
        &self,
        request: &DownloadRequest,
        state: &AppState,
        app: &AppHandle,
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

    async fn run_inner(
        &self,
        request: &DownloadRequest,
        state: &AppState,
        app: &AppHandle,
    ) -> Result<(), AppError> {
        self.emit(app, ProgressPhase::FetchingUserInfo, 0, 1, "正在获取用户信息...");

        let client = WeiboApiClient::new(app.clone());
        let user = client.get_user_info(&request.uid).await?;

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

        let mut all_raw_posts = Vec::new();
        let mut total_posts = 0_i64;
        let mut page = 1_i64;

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

            all_raw_posts.extend(posts);
            state
                .posts_fetched
                .store(all_raw_posts.len(), Ordering::Relaxed);
            state.current_page.store(page as usize, Ordering::Relaxed);

            self.emit(
                app,
                ProgressPhase::FetchingPostList,
                all_raw_posts.len(),
                total_posts.max(all_raw_posts.len() as i64) as usize,
                &format!("已获取 {} 条微博...", all_raw_posts.len()),
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

        let mut normalized_posts = Vec::with_capacity(all_raw_posts.len());
        for (index, raw) in all_raw_posts.iter().enumerate() {
            self.ensure_not_cancelled(state, app)?;

            let text = if raw.is_long_text == Some(true) {
                match client.get_long_text(&raw.mblogid).await {
                    Ok(content) => content,
                    Err(_) => raw.text.clone(),
                }
            } else {
                raw.text.clone()
            };

            let mut post = client.normalize_post(raw, &request.uid);
            post.text = text;

            match request.filter {
                PostFilter::Original if post.is_repost => continue,
                PostFilter::Original | PostFilter::All => {}
            }

            if post.text.chars().count() < request.min_text_length {
                continue;
            }

            normalized_posts.push(post);

            self.emit(
                app,
                ProgressPhase::FetchingLongText,
                index + 1,
                all_raw_posts.len(),
                &format!("已处理 {}/{} 条...", index + 1, all_raw_posts.len()),
            );
        }

        if request.include_images {
            self.download_images(request, &mut normalized_posts, state, app)
                .await?;
        }

        self.emit(app, ProgressPhase::Exporting, 0, 1, "正在导出文件...");
        std::fs::create_dir_all(&request.output_dir)?;

        match request.export_format {
            ExportFormat::Pdf => {
                PdfExportService::new()
                    .export(&normalized_posts, &request.output_dir)
                    .await?;
            }
            ExportFormat::MarkdownSingle | ExportFormat::MarkdownPerPost => {
                MarkdownExportService::new()
                    .export(&normalized_posts, &request.output_dir)
                    .await?;
            }
        }

        self.emit(app, ProgressPhase::Complete, 1, 1, "下载完成！");
        Ok(())
    }

    async fn download_images(
        &self,
        request: &DownloadRequest,
        posts: &mut [WeiboPost],
        state: &AppState,
        app: &AppHandle,
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

            let stored_paths = match image_store.download_images(&urls, &image_dir).await {
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

    fn ensure_not_cancelled(&self, state: &AppState, app: &AppHandle) -> Result<(), AppError> {
        if state.is_cancelled() {
            self.emit(app, ProgressPhase::Cancelled, 0, 0, "已取消");
            return Err(AppError::Cancelled);
        }

        Ok(())
    }

    fn emit(
        &self,
        app: &AppHandle,
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
