use std::path::PathBuf;
use std::sync::atomic::Ordering;
use std::{collections::{HashMap, HashSet}, path::Path};

use chrono::{Datelike, TimeZone, Utc};
use tauri::{AppHandle, Emitter, Runtime};

use crate::error::AppError;
use crate::models::{
    CheckpointMeta, DownloadRequest, ExportContext, PostFilter, ProgressEvent, ProgressPhase,
    RawPost, SourceType, UserProfile, WeiboPost,
};
use crate::services::{
    file_naming::{dedupe_filenames, post_filename},
    image_store::ImageStoreService,
    weibo_api::WeiboApiClient,
};
use crate::state::AppState;

#[derive(Debug, Default)]
pub struct DownloadService;

fn needs_long_text(raw: &RawPost) -> bool {
    raw.is_long_text == Some(true) || raw.text.contains("展开") || raw.text.contains("全文")
}

/// Count visible characters in text, excluding HTML tags
/// This is used for the min_text_length filter
fn count_visible_chars(html: &str) -> usize {
    let mut in_tag = false;
    let mut count = 0;

    for c in html.chars() {
        match c {
            '<' => in_tag = true,
            '>' => in_tag = false,
            c if !in_tag => {
                // Only count non-whitespace visible characters
                if !c.is_whitespace() {
                    count += 1;
                }
            }
            _ => {}
        }
    }

    count
}

fn parse_weibo_date_to_timestamp(created_at: &str) -> i64 {
    let created_at = created_at.trim();
    if created_at.is_empty() {
        return 0;
    }

    let offset = chrono::FixedOffset::east_opt(8 * 3600).unwrap();

    // Standard Weibo format: "Wed Mar 19 09:53:32 +0800 2014"
    if let Ok(dt) = chrono::DateTime::parse_from_str(created_at, "%a %b %d %H:%M:%S %z %Y") {
        return dt.timestamp();
    }

    // Format: "2024-03-19 09:53:32"
    if let Ok(dt) = chrono::NaiveDateTime::parse_from_str(created_at, "%Y-%m-%d %H:%M:%S") {
        // Assume +0800 if no offset
        if let Some(dt_with_tz) = offset.from_local_datetime(&dt).single() {
            return dt_with_tz.timestamp();
        }
    }

    // Relative formats (often used in some API views)
    let now_utc = Utc::now();
    let now_ts = now_utc.timestamp();
    
    if created_at.contains("秒前") {
        if let Some(n) = created_at.split("秒").next().and_then(|s| s.parse::<i64>().ok()) {
            return now_ts - n;
        }
    }
    if created_at.contains("分钟前") {
        if let Some(n) = created_at.split("分").next().and_then(|s| s.parse::<i64>().ok()) {
            return now_ts - n * 60;
        }
    }
    if created_at.contains("小时前") {
        if let Some(n) = created_at.split("小").next().and_then(|s| s.parse::<i64>().ok()) {
            return now_ts - n * 3600;
        }
    }
    if created_at.contains("今天") {
        // Format: "今天 10:00"
        if let Some(hm) = created_at.split_whitespace().nth(1) {
            let mut parts = hm.split(':');
            let h = parts.next().and_then(|s| s.parse::<u32>().ok()).unwrap_or(0);
            let m = parts.next().and_then(|s| s.parse::<u32>().ok()).unwrap_or(0);
            
            let bj_now = now_utc.with_timezone(&offset);
            if let Some(dt) = bj_now.date_naive().and_hms_opt(h, m, 0) {
                if let Some(dt_with_tz) = offset.from_local_datetime(&dt).single() {
                    return dt_with_tz.timestamp();
                }
            }
        }
    }
    if created_at.contains("昨天") {
        // Format: "昨天 10:00"
        if let Some(hm) = created_at.split_whitespace().nth(1) {
            let mut parts = hm.split(':');
            let h = parts.next().and_then(|s| s.parse::<u32>().ok()).unwrap_or(0);
            let m = parts.next().and_then(|s| s.parse::<u32>().ok()).unwrap_or(0);
            
            let bj_yesterday = (now_utc - chrono::Duration::days(1)).with_timezone(&offset);
            if let Some(dt) = bj_yesterday.date_naive().and_hms_opt(h, m, 0) {
                if let Some(dt_with_tz) = offset.from_local_datetime(&dt).single() {
                    return dt_with_tz.timestamp();
                }
            }
        }
    }

    // Format: "04-07 10:00" (Current year)
    if created_at.len() == 11 && created_at.chars().nth(2) == Some('-') && created_at.chars().nth(5) == Some(' ') {
        let bj_now = now_utc.with_timezone(&offset);
        let year = bj_now.year();
        let full_date = format!("{}-{}", year, created_at);
        if let Ok(dt) = chrono::NaiveDateTime::parse_from_str(&full_date, "%Y-%m-%d %H:%M") {
            if let Some(dt_with_tz) = offset.from_local_datetime(&dt).single() {
                return dt_with_tz.timestamp();
            }
        }
    }

    // Fallback for YYYY-MM-DD
    if created_at.len() >= 10 && created_at.contains('-') {
        if let Ok(date) = chrono::NaiveDate::parse_from_str(&created_at[..10], "%Y-%m-%d") {
            if let Some(dt) = date.and_hms_opt(0, 0, 0) {
                if let Some(dt_with_tz) = offset.from_local_datetime(&dt).single() {
                    return dt_with_tz.timestamp();
                }
            }
        }
    }

    // Last resort: try to parse any date-like string
    0
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
    ) -> Result<UserProfile, AppError> {
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
    ) -> Result<UserProfile, AppError> {
        self.emit(app, ProgressPhase::FetchingUserInfo, 0, 1, "正在获取用户信息...");

        let client = WeiboApiClient::new(app.clone());
        let user = match request.source_type {
            SourceType::Profile => client.get_user_info(&request.uid).await?,
            SourceType::Favorites => UserProfile {
                uid: "favorites".to_string(),
                screen_name: "我的收藏".to_string(),
            },
        };

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
        let mut empty_pages_count = 0;

        loop {
            self.ensure_not_cancelled(state, app)?;

            let (posts, total) = match request.source_type {
                SourceType::Profile => {
                    // Don't pass starttime/endtime to API - it causes 50 post limit
                    // We'll filter on the client side instead
                    eprintln!("[DEBUG] Fetching page {} - date filter will be applied client-side", page);
                    let (raw_posts, api_total) = client.fetch_posts_page(&request.uid, page, None, None).await?;

                    // Check if all posts on this page are older than start time
                    // If so, we've gone past the date range and can stop
                    if let Some(start) = starttime {
                        let all_older = raw_posts.iter().all(|p| {
                            let ts = parse_weibo_date_to_timestamp(&p.created_at);
                            ts > 0 && ts < start
                        });
                        if !raw_posts.is_empty() && all_older {
                            eprintln!("[DEBUG] All posts on page {} are older than start date, stopping", page);
                            break;
                        }
                    }

                    // Filter by date range on the client side
                    let filtered: Vec<RawPost> = raw_posts.into_iter().filter(|p| {
                        let ts = parse_weibo_date_to_timestamp(&p.created_at);
                        if ts == 0 && !p.created_at.is_empty() {
                            // Warn but don't filter out posts with unparseable dates
                            self.emit(
                                app,
                                ProgressPhase::FetchingPostList,
                                0,
                                0,
                                &format!("⚠️ 警告：无法解析日期格式 \"{}\"", p.created_at),
                            );
                        }

                        // Check date range
                        if let Some(start) = starttime {
                            if ts < start {
                                return false;
                            }
                        }
                        if let Some(end) = endtime {
                            if ts > end {
                                return false;
                            }
                        }
                        true
                    }).collect();

                    (filtered, api_total)
                }
                SourceType::Favorites => {
                    let (all_fav_posts, _) = client.fetch_favorites_page(page, starttime, endtime).await?;
                    let mut filtered = Vec::new();
                    for p in all_fav_posts {
                        let ts = parse_weibo_date_to_timestamp(&p.created_at);

                        if ts == 0 && !p.created_at.is_empty() {
                            self.emit(
                                app,
                                ProgressPhase::FetchingPostList,
                                0,
                                0,
                                &format!("⚠️ 警告：无法解析日期格式 \"{}\"，该微博可能会被过滤掉", p.created_at),
                            );
                        }

                        #[cfg(debug_assertions)]
                        println!("[debug] fav post: mblogid={}, created_at={}, ts={}, start={:?}, end={:?}", p.mblogid, p.created_at, ts, starttime, endtime);

                        if let Some(start) = starttime {
                            if ts < start {
                                continue;
                            }
                        }
                        if let Some(end) = endtime {
                            if ts > end {
                                continue;
                            }
                        }
                        filtered.push(p);
                    }
                    if filtered.is_empty() && page - start_page as i64 > 50 {
                         // We might want to stop if we've gone too far, but let's see.
                    }
                    (filtered, -1)
                }
            };

            if total_posts == 0 && total > 0 {
                total_posts = total;
                eprintln!("[DEBUG] API returned total={} posts, page_size={}", total, posts.len());
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
                empty_pages_count += 1;
                // Allow more empty pages when date range is specified (API may have gaps)
                let max_empty_pages = if starttime.is_some() || endtime.is_some() {
                    50  // More tolerance for date-filtered queries - we might skip many pages
                } else {
                    3   // Fewer empty pages for normal queries
                };
                if empty_pages_count > max_empty_pages {
                    break;
                }
                page += 1;
                continue;
            }
            empty_pages_count = 0;

            let new_posts: Vec<RawPost> = posts
                .into_iter()
                .filter(|post| !existing_ids.contains(&post.mblogid))
                // Filter out deleted posts if ignore_deleted is true
                .filter(|post| !(request.ignore_deleted && post.user.is_none()))
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

        let type_label = match request.source_type {
            SourceType::Profile => "微博备份",
            SourceType::Favorites => "收藏微博",
        }
        .to_string();

        let export_context = ExportContext {
            date_range_label: format_date_range_label(
                request.date_range.start_timestamp,
                request.date_range.end_timestamp,
            ),
            type_label,
        };

        let mut normalized_posts = resumed_posts.clone();
        normalized_posts.reserve(all_raw_posts.len());
        for (index, raw) in all_raw_posts.iter().enumerate() {
            self.ensure_not_cancelled(state, app)?;

            if request.ignore_deleted && raw.user.is_none() {
                continue;
            }

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

            let uid_for_normalize = match request.source_type {
                SourceType::Profile => &request.uid,
                SourceType::Favorites => "",
            };
            let post = WeiboApiClient::<R>::normalize_post(&normalized_raw, uid_for_normalize);

            match request.filter {
                PostFilter::Original if post.is_repost => continue,
                PostFilter::Original | PostFilter::All => {}
            }

            // Filter by text length (only for profile mode, not favorites)
            if matches!(request.source_type, SourceType::Profile) && request.min_text_length > 0 {
                // Count visible characters (excluding HTML tags and URLs)
                let text_len = count_visible_chars(&post.text);
                if text_len < request.min_text_length {
                    continue;
                }
            }

            normalized_posts.push(post);

            state.posts_exported.store(normalized_posts.len(), Ordering::Relaxed);

            if normalized_posts.len() % 10 == 0 {
                let _ = crate::services::cache::save_cache_checkpoint(
                    &normalized_posts,
                    &request.output_dir,
                    CheckpointMeta {
                        uid: request.uid.clone(),
                        source_type: Some(request.source_type.clone()),
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
                source_type: Some(request.source_type.clone()),
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

        Ok(user)
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
        let client = state.get_client();
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

            let stored_paths = match image_store.download_images(&client, &urls, &image_dir).await {
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_date_parsing() {
        let ts = parse_weibo_date_to_timestamp("Wed Mar 19 09:53:32 +0800 2014");
        assert!(ts > 0, "Should parse correctly");
        assert_eq!(ts, 1395194012);
        
        let ts2 = parse_weibo_date_to_timestamp("2024-03-19 09:53:32");
        assert!(ts2 > 0, "Should parse ISO-like");
        // 2024-03-19 09:53:32 +0800 -> 1710813212
        assert_eq!(ts2, 1710813212);

        let ts3 = parse_weibo_date_to_timestamp("2024-03-19");
        assert!(ts3 > 0, "Should parse YYYY-MM-DD");
        // 2024-03-19 00:00:00 +0800 -> 1710777600
        assert_eq!(ts3, 1710777600);
    }
}
