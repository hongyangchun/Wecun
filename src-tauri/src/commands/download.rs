use std::sync::atomic::Ordering;

use tauri::{Emitter, State};

use crate::error::AppError;
use crate::models::{DownloadRequest, ExportFormat, ExportRequest, ProgressEvent, ProgressPhase};
use crate::services::downloader::DownloadService;
use crate::services::export_html::HtmlExportService;
use crate::services::export_markdown::MarkdownExportService;
use crate::services::export_profile::ProfileExportService;
use crate::services::profile::models::ProfileResult;
use crate::services::ProfileService;
use crate::services::weibo_api::{
    clear_saved_cookie, load_saved_cookie, open_login_window as weibo_open_login, restore_saved_cookie,
};
use crate::state::AppState;
use tauri::Manager;

#[tauri::command]
pub async fn open_login_window(app: tauri::AppHandle) -> Result<(), String> {
    weibo_open_login(app).await
}

#[tauri::command]
pub async fn start_download(
    request: DownloadRequest,
    state: State<'_, AppState>,
    app: tauri::AppHandle,
) -> Result<String, String> {
    if request.source_type == crate::models::SourceType::Profile && request.uid.trim().is_empty() {
        return Err("用户ID不能为空".to_string());
    }
    if state.get_cookie().is_empty() {
        return Err("请先登录微博".to_string());
    }
    if request.output_dir.is_empty() {
        return Err("请选择保存目录".to_string());
    }

    state.reset();

    let service = DownloadService::new();
    match service.run(&request, &state, &app).await {
        Ok(_user) => {
            let count = state.posts_exported.load(Ordering::Relaxed);
            let _ = app.emit(
                "download-progress",
                ProgressEvent::new(
                    ProgressPhase::Complete,
                    count,
                    count,
                    &format!("下载完成！共 {} 条微博", count),
                ),
            );
            Ok(format!("下载完成！共 {} 条微博", count))
        }
        Err(AppError::Cancelled) => Err("已取消下载".to_string()),
        Err(e) => Err(format!("下载失败: {e}")),
    }
}

#[tauri::command]
pub fn cancel_download(state: State<'_, AppState>) {
    state.cancel_flag.store(true, Ordering::Relaxed);
}

#[tauri::command]
pub async fn export_posts(
    request: ExportRequest,
    app: tauri::AppHandle,
) -> Result<String, String> {
    let bundle = crate::services::cache::load_cache_bundle_sync(&request.output_dir)
        .map_err(|e| {
            if matches!(e, AppError::Io(ref io_err) if io_err.kind() == std::io::ErrorKind::NotFound) {
                "找不到缓存文件，请先完成下载".to_string()
            } else if matches!(e, AppError::Parse(_)) {
                "缓存文件已损坏，请重新下载".to_string()
            } else {
                format!("读取缓存失败: {e}")
            }
        })?;
    let export_context = bundle.export_context;
    let posts = bundle.posts;

    if posts.is_empty() {
        return Err("没有可导出的微博数据".to_string());
    }

    let _ = app.emit(
        "download-progress",
        ProgressEvent::new(ProgressPhase::Exporting, 1, 1, &format!("正在导出 {}...", format_label(&request.export_format))),
    );

    let posts_count = posts.len();

    match request.export_format {
        ExportFormat::MarkdownSingle => MarkdownExportService::new()
            .export(&posts, &request.output_dir, &export_context, true)
            .await
            .map_err(|e| format!("导出Markdown失败: {e}"))?,
        ExportFormat::MarkdownObsidian => MarkdownExportService::new()
            .export(&posts, &request.output_dir, &export_context, false)
            .await
            .map_err(|e| format!("导出Markdown(Obsidian)失败: {e}"))?,
        ExportFormat::Html => HtmlExportService::new()
            .export(&posts, &request.output_dir, &export_context)
            .await
            .map_err(|e| format!("导出HTML失败: {e}"))?,
    }

    let _ = app.emit(
        "download-progress",
        ProgressEvent::new(ProgressPhase::Complete, posts_count, posts_count, "导出完成！"),
    );

    Ok(format!("导出完成！已生成 {}", format_label(&request.export_format)))
}

#[tauri::command]
pub async fn analyze_profile(
    output_dir: String,
    _app: tauri::AppHandle,
) -> Result<ProfileResult, String> {
    let service = ProfileService::new();
    service
        .analyze(&output_dir)
        .await
        .map_err(|error| format!("分析失败: {error}"))
}

#[tauri::command]
pub async fn export_profile(
    output_dir: String,
    profile: ProfileResult,
) -> Result<String, String> {
    ProfileExportService::new()
        .export(&output_dir, &profile)
        .await
        .map_err(|error| format!("导出画像分析失败: {error}"))
}

fn format_label(fmt: &ExportFormat) -> &'static str {
    match fmt {
        ExportFormat::MarkdownSingle => "Markdown",
        ExportFormat::MarkdownObsidian => "Markdown (Obsidian兼容)",
        ExportFormat::Html => "HTML",
    }
}

#[tauri::command]
pub fn has_saved_cookie(app: tauri::AppHandle) -> bool {
    load_saved_cookie(&app)
        .map(|cookie| !cookie.trim().is_empty())
        .unwrap_or(false)
}

/// Returns the saved cookie if exists, empty string otherwise.
/// This is more efficient than calling has_saved_cookie + load_saved_cookie_cmd.
#[tauri::command]
pub fn get_saved_cookie(app: tauri::AppHandle) -> String {
    load_saved_cookie(&app)
        .filter(|cookie| !cookie.trim().is_empty())
        .unwrap_or_else(String::new)
}

/// Get current logged-in user info.
/// Returns empty string if not logged in or on error.
#[tauri::command]
pub async fn get_current_user_info(app: tauri::AppHandle) -> String {
    use crate::services::weibo_api::WeiboApiClient;

    let client = WeiboApiClient::new(app.clone());
    let state = app.state::<AppState>();

    if state.get_cookie().is_empty() {
        return String::new();
    }

    // Try to get current user info by calling profile info without uid
    // Weibo API should return current user's info when no uid is provided
    match client.get_current_user_info().await {
        Ok(user) => user.screen_name,
        Err(_) => String::new(),
    }
}

#[tauri::command]
pub fn load_saved_cookie_cmd(app: tauri::AppHandle) -> Result<String, String> {
    let state = app.state::<AppState>();
    restore_saved_cookie(&app, &state).ok_or_else(|| "没有保存的登录信息".to_string())
}

#[tauri::command]
pub fn clear_saved_cookie_cmd(app: tauri::AppHandle) {
    let state = app.state::<AppState>();
    state.set_cookie(String::new());
    clear_saved_cookie(&app);
}
