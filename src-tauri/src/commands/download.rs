use std::sync::atomic::Ordering;

use tauri::State;

use crate::error::AppError;
use crate::models::DownloadRequest;
use crate::services::downloader::DownloadService;
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
    if request.uid.trim().is_empty() {
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
        Ok(()) => Ok(format!("下载完成！共 {} 条微博", state.posts_exported.load(Ordering::Relaxed))),
        Err(AppError::Cancelled) => Err("已取消下载".to_string()),
        Err(e) => Err(format!("下载失败: {e}")),
    }
}

#[tauri::command]
pub fn cancel_download(state: State<'_, AppState>) {
    state.cancel_flag.store(true, Ordering::Relaxed);
}

#[tauri::command]
pub fn has_saved_cookie(app: tauri::AppHandle) -> bool {
    load_saved_cookie(&app)
        .map(|cookie| !cookie.trim().is_empty())
        .unwrap_or(false)
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
