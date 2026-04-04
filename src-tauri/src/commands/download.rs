use std::sync::atomic::Ordering;
use std::time::Duration;

use tauri::{Emitter, Manager, State, Url, WebviewUrl, WebviewWindowBuilder};

use crate::error::AppError;
use crate::models::DownloadRequest;
use crate::services::downloader::DownloadService;
use crate::state::AppState;

const LOGIN_WINDOW_LABEL: &str = "weibo-login";
const WEIBO_HOME_URL: &str = "https://weibo.com";
const WEIBO_LOGIN_URL: &str = "https://passport.weibo.com/sso/signin";

fn is_logged_in_url(url: &Url) -> bool {
    let url_str = url.as_str();
    url_str.starts_with(WEIBO_HOME_URL)
        && !url_str.contains("sso/signin")
        && !url_str.contains("passport.weibo.com")
}

#[tauri::command]
pub async fn open_login_window(app: tauri::AppHandle) -> Result<(), String> {
    if let Some(existing) = app.get_webview_window(LOGIN_WINDOW_LABEL) {
        existing.show().map_err(|e| e.to_string())?;
        existing.set_focus().map_err(|e| e.to_string())?;
        return Ok(());
    }

    let app_clone = app.clone();
    let state = app.state::<AppState>();
    state
        .api_window_ready
        .store(false, std::sync::atomic::Ordering::Relaxed);
    state.clear_all_api_results();

    let _ = WebviewWindowBuilder::new(
        &app,
        LOGIN_WINDOW_LABEL,
        WebviewUrl::External(WEIBO_LOGIN_URL.parse().unwrap()),
    )
    .title("登录微博")
    .inner_size(480.0, 560.0)
    .resizable(false)
    .center()
    .visible(true)
    .on_navigation(move |url: &Url| {
        if is_logged_in_url(url) {
            let app = app_clone.clone();
            tauri::async_runtime::spawn(async move {
                tokio::time::sleep(Duration::from_secs(1)).await;

                if let Some(win) = app.get_webview_window(LOGIN_WINDOW_LABEL) {
                    let all_cookies = win.cookies().ok();

                    if let Some(cookies) = all_cookies {
                        let cookie_string: Vec<String> = cookies
                            .iter()
                            .map(|c| format!("{}={}", c.name(), c.value()))
                            .collect();

                        if !cookie_string.is_empty() {
                            let _ = app.emit("cookie-received", cookie_string.join("; "));
                        }
                    }

                    let state = app.state::<AppState>();
                    state
                        .api_window_ready
                        .store(true, std::sync::atomic::Ordering::Relaxed);

                    if let Ok(current_url) = win.url() {
                        if current_url.as_str() != WEIBO_HOME_URL {
                            let _ = win.navigate(WEIBO_HOME_URL.parse().unwrap());
                            tokio::time::sleep(Duration::from_millis(500)).await;
                        }
                    }

                    let _ = win.hide();
                    let _ = app.emit("login-success", "");
                }
            });
        }
        true
    })
    .build()
    .map_err(|e| e.to_string())?;

    Ok(())
}

#[tauri::command]
pub async fn test_api(_cookie: String, uid: String) -> Result<String, String> {
    let app = crate::state::app_handle().ok_or_else(|| "应用未初始化".to_string())?;
    let body = crate::services::weibo_api::weibo_api_call(
        &app,
        "/ajax/profile/info",
        &[("uid", uid)],
    )
    .await
    .map_err(|e| e.to_string())?;

    Ok(format!("Body: {}", &body[..body.len().min(500)]))
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
    if request.cookie.trim().is_empty() {
        return Err("Cookie不能为空".to_string());
    }
    if request.output_dir.is_empty() {
        return Err("请选择保存目录".to_string());
    }

    state.reset();

    let service = DownloadService::new();
    match service.run(&request, &state, &app).await {
        Ok(()) => Ok(format!("下载完成！共 {} 条微博", state.posts_fetched.load(Ordering::Relaxed))),
        Err(AppError::Cancelled) => Err("已取消下载".to_string()),
        Err(e) => Err(format!("下载失败: {e}")),
    }
}

#[tauri::command]
pub fn cancel_download(state: State<'_, AppState>) {
    state.cancel_flag.store(true, Ordering::Relaxed);
}
