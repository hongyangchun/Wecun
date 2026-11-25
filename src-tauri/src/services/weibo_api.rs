use std::fs;
use std::future::Future;
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};

use reqwest::StatusCode;
use tauri::{AppHandle, Emitter, Manager, Runtime, Url, WebviewUrl, WebviewWindowBuilder, WindowEvent};
use tokio::sync::Mutex;

use crate::error::AppError;
use crate::models::{
    ProgressEvent, ProgressPhase, RawHistoryMap, RawLongText, RawPost, RawSearchProfile, RawFavProfile,
    RawUserInfo, UserProfile, WeiboImage, WeiboPost,
};
use crate::state::AppState;

const LOGIN_WINDOW_LABEL: &str = "weibo-login";
const WEIBO_BASE: &str = "https://weibo.com";
const MIN_REQUEST_INTERVAL: Duration = Duration::from_millis(1000);
const COOKIE_FILE_NAME: &str = "cookie.json";

#[derive(Debug, Clone)]
struct RetryConfig {
    max_retries: u32,
    initial_delay: Duration,
    max_delay: Duration,
}

impl Default for RetryConfig {
    fn default() -> Self {
        Self {
            max_retries: 3,
            initial_delay: Duration::from_secs(2),
            max_delay: Duration::from_secs(30),
        }
    }
}

pub struct WeiboApiClient<R: Runtime> {
    app: AppHandle<R>,
    last_fetch: Mutex<Option<Instant>>,
}

impl<R: Runtime> WeiboApiClient<R> {
    pub fn new(app: AppHandle<R>) -> Self {
        Self {
            app,
            last_fetch: Mutex::new(None),
        }
    }

    async fn throttle(&self) -> Result<(), AppError> {
        let sleep_duration = {
            let mut guard = self.last_fetch.lock().await;
            let wait = guard.and_then(|last| remaining_throttle_delay(last.elapsed(), MIN_REQUEST_INTERVAL));
            if wait.is_none() {
                *guard = Some(Instant::now());
            }
            wait
        };

        if let Some(duration) = sleep_duration {
            let state = self.app.state::<AppState>();
            if state.sleep_cancellable(duration).await {
                return Err(AppError::Cancelled);
            }
            let mut guard = self.last_fetch.lock().await;
            *guard = Some(Instant::now());
        }

        Ok(())
    }

    fn build_url(path: &str, query: &[(&str, String)]) -> String {
        let mut url = format!("{WEIBO_BASE}{path}");
        if !query.is_empty() {
            let params: Vec<String> = query
                .iter()
                .map(|(k, v)| format!("{}={}", urlencoding::encode(k), urlencoding::encode(v)))
                .collect();
            url.push('?');
            url.push_str(&params.join("&"));
        }
        url
    }

    async fn get_json<T: serde::de::DeserializeOwned>(
        &self,
        path: &str,
        query: &[(&str, String)],
    ) -> Result<T, AppError> {
        self.throttle().await?;

        let state = self.app.state::<AppState>();
        let cookie = state.get_cookie();
        if cookie.is_empty() {
            return Err(AppError::ApiError("请先登录".to_string()));
        }

        let client = state.get_client();
        let url = Self::build_url(path, query);

        let (status, text) = self
            .request_with_retry(path, progress_phase_for_path(path), || {
                let client = client.clone();
                let url = url.clone();
                let cookie = cookie.clone();
                async move { self.send_request(client, url, cookie).await }
            })
            .await?;

        if is_auth_invalid_response(status, &text) {
            // Check if cookie was recently set (grace period to avoid race conditions during account switching)
            let last_set = state.get_last_cookie_set_time();
            let elapsed = last_set.map(|t| t.elapsed());
            let grace_period = Duration::from_secs(10);
            let should_suppress_error = elapsed.is_some_and(|e| e < grace_period);

            if should_suppress_error {
                // Cookie was just set, likely due to account switching. Return error but don't clear state.
                let remaining = grace_period.saturating_sub(elapsed.unwrap_or_default());
                return Err(AppError::ApiError(format!(
                    "正在切换账号，请等待 {:.0} 秒后重试",
                    remaining.as_secs_f64()
                )));
            }

            #[cfg(debug_assertions)]
            eprintln!(
                "[DEBUG] Auth failure - Status: {}, Path: {}, Body preview: {}",
                status,
                path,
                preview_str(&text, 200)
            );

            state.set_cookie(String::new());
            clear_saved_cookie(&self.app);
            let _ = self.app.emit("login-invalid", "登录已失效，请重新登录");
            return Err(AppError::InvalidCookie);
        }

        if !status.is_success() {
            return Err(AppError::ApiError(format!(
                "HTTP {status} from {path}: {}",
                preview_str(&text, 300)
            )));
        }

        serde_json::from_str(&text)
            .map_err(|e| AppError::Parse(format!("解析 {path} 失败: {e}\nBody: {}", preview_str(&text, 300))))
    }

    /// Send GET request with a specific cookie (doesn't use state cookie).
    async fn get_json_with_cookie<T: serde::de::DeserializeOwned>(
        &self,
        path: &str,
        query: &[(&str, String)],
        cookie: &str,
    ) -> Result<T, AppError> {
        self.throttle().await?;

        if cookie.is_empty() {
            return Err(AppError::ApiError("请先登录".to_string()));
        }

        let state = self.app.state::<AppState>();
        let client = state.get_client();
        let url = Self::build_url(path, query);

        let cookie_owned = cookie.to_string();
        let (status, text) = self
            .request_with_retry(path, ProgressPhase::FetchingUserInfo, || {
                let client = client.clone();
                let url = url.clone();
                let cookie = cookie_owned.clone();
                async move { self.send_request(client, url, cookie).await }
            })
            .await?;

        // Don't check auth invalid here since we're validating a cookie
        if !status.is_success() {
            return Err(AppError::ApiError(format!(
                "HTTP {status} from {path}: {}",
                preview_str(&text, 300)
            )));
        }

        serde_json::from_str(&text)
            .map_err(|e| AppError::Parse(format!("解析 {path} 失败: {e}\nBody: {}", preview_str(&text, 300))))
    }

    async fn request_with_retry<F, Fut>(
        &self,
        path: &str,
        phase: ProgressPhase,
        request_fn: F,
    ) -> Result<(StatusCode, String), AppError>
    where
        F: Fn() -> Fut,
        Fut: Future<Output = Result<(StatusCode, String), AppError>>,
    {
        let retry_config = RetryConfig::default();
        let state = self.app.state::<AppState>();
        let mut attempt = 0;

        loop {
            if state.is_cancelled() {
                return Err(AppError::Cancelled);
            }

            match request_fn().await {
                Ok((status, text)) => {
                    if is_auth_invalid_response(status, &text) || !is_retryable_status(status) {
                        return Ok((status, text));
                    }

                    attempt += 1;
                    if attempt > retry_config.max_retries {
                        return Ok((status, text));
                    }

                    self.emit_retry_progress(
                        phase.clone(),
                        attempt,
                        retry_config.max_retries,
                        &format!(
                            "请求 {path} 失败({status}: {})，正在重试({}/{})...",
                            text.chars().take(50).collect::<String>(),
                            attempt,
                            retry_config.max_retries
                        ),
                    );

                    if state.sleep_cancellable(retry_delay(&retry_config, attempt)).await {
                        return Err(AppError::Cancelled);
                    }
                }
                Err(error) => {
                    if !is_network_error(&error) {
                        return Err(error);
                    }

                    attempt += 1;
                    if attempt > retry_config.max_retries {
                        return Err(error);
                    }

                    self.emit_retry_progress(
                        phase.clone(),
                        attempt,
                        retry_config.max_retries,
                        &format!(
                            "请求 {path} 网络异常，正在重试({}/{})...",
                            attempt,
                            retry_config.max_retries
                        ),
                    );

                    if state.sleep_cancellable(retry_delay(&retry_config, attempt)).await {
                        return Err(AppError::Cancelled);
                    }
                }
            }
        }
    }

    async fn send_request(
        &self,
        client: reqwest::Client,
        url: String,
        cookie: String,
    ) -> Result<(StatusCode, String), AppError> {
        let resp = client
            .get(&url)
            .header("Cookie", cookie)
            .header("Referer", "https://weibo.com/")
            .send()
            .await
            .map_err(AppError::Network)?;

        let status = resp.status();
        let text = resp.text().await.map_err(AppError::Network)?;
        Ok((status, text))
    }

    fn emit_retry_progress(
        &self,
        phase: ProgressPhase,
        current: u32,
        total: u32,
        message: &str,
    ) {
        let _ = self.app.emit(
            "download-progress",
            ProgressEvent::new(phase, current as usize, total as usize, message),
        );
    }

    pub async fn get_user_info(&self, uid: &str) -> Result<UserProfile, AppError> {
        let raw: RawUserInfo = self
            .get_json("/ajax/profile/info", &[("uid", uid.to_string())])
            .await?;
        Ok(UserProfile {
            uid: raw.data.user.id.to_string(),
            screen_name: raw.data.user.screen_name,
        })
    }

    pub async fn get_user_info_with_cookie(&self, uid: &str, cookie: &str) -> Result<UserProfile, AppError> {
        let raw: RawUserInfo = self
            .get_json_with_cookie("/ajax/profile/info", &[("uid", uid.to_string())], cookie)
            .await?;
        Ok(UserProfile {
            uid: raw.data.user.id.to_string(),
            screen_name: raw.data.user.screen_name,
        })
    }

    /// Get current logged-in user info using state cookie.
    pub async fn get_current_user_info(&self) -> Result<UserProfile, AppError> {
        let state = self.app.state::<AppState>();
        let cookie = state.get_cookie();
        self.get_current_user_info_with_cookie(&cookie).await
    }

    /// Get current logged-in user info with a specific cookie.
    /// Uses multiple reliable strategies:
    /// 1. `https://m.weibo.cn/api/config` -> extract `data.uid` -> `/ajax/profile/info?uid={uid}`
    /// 2. `https://weibo.com/` HTML -> extract embedded `$CONFIG` uid -> `/ajax/profile/info?uid={uid}`
    /// 3. Legacy `/ajax/config/info` fallback
    pub async fn get_current_user_info_with_cookie(&self, cookie: &str) -> Result<UserProfile, AppError> {
        if cookie.trim().is_empty() {
            return Err(AppError::ApiError("请先登录".to_string()));
        }

        // Strategy 1: m.weibo.cn/api/config reliably returns logged-in uid
        if let Some(uid) = self.fetch_uid_from_mobile_config(cookie).await {
            if let Ok(profile) = self.get_user_info_with_cookie(&uid, cookie).await {
                if !profile.screen_name.trim().is_empty() {
                    return Ok(profile);
                }
            }
            return Ok(UserProfile {
                uid: uid.clone(),
                screen_name: format!("用户{uid}"),
            });
        }

        // Strategy 2: parse uid from weibo.com home HTML
        if let Some(uid) = self.fetch_uid_from_weibo_home(cookie).await {
            if let Ok(profile) = self.get_user_info_with_cookie(&uid, cookie).await {
                if !profile.screen_name.trim().is_empty() {
                    return Ok(profile);
                }
            }
            return Ok(UserProfile {
                uid: uid.clone(),
                screen_name: format!("用户{uid}"),
            });
        }

        // Strategy 3: legacy /ajax/config/info endpoint
        #[derive(serde::Deserialize)]
        struct ConfigResponse {
            data: ConfigData,
        }

        #[derive(serde::Deserialize)]
        struct ConfigData {
            #[serde(default)]
            uid: Option<String>,
            #[serde(default)]
            screen_name: Option<String>,
            #[serde(default)]
            user: Option<ConfigUser>,
        }

        #[derive(serde::Deserialize)]
        struct ConfigUser {
            id: Option<i64>,
            screen_name: Option<String>,
        }

        if let Ok(resp) = self.get_json_with_cookie::<ConfigResponse>("/ajax/config/info", &[], cookie).await {
            let uid = resp
                .data
                .uid
                .or_else(|| resp.data.user.as_ref().and_then(|u| u.id.map(|id| id.to_string())))
                .unwrap_or_default();
            let screen_name = resp
                .data
                .screen_name
                .or_else(|| resp.data.user.as_ref().and_then(|u| u.screen_name.clone()))
                .unwrap_or_default();

            if !uid.is_empty() && !screen_name.is_empty() {
                return Ok(UserProfile { uid, screen_name });
            }
            if !uid.is_empty() {
                if let Ok(profile) = self.get_user_info_with_cookie(&uid, cookie).await {
                    return Ok(profile);
                }
            }
        }

        Err(AppError::ApiError("无法获取当前登录用户信息".to_string()))
    }

    async fn fetch_uid_from_mobile_config(&self, cookie: &str) -> Option<String> {
        #[derive(serde::Deserialize)]
        struct MobileConfigResp {
            data: Option<MobileConfigData>,
        }
        #[derive(serde::Deserialize)]
        struct MobileConfigData {
            #[serde(default)]
            login: Option<bool>,
            #[serde(default)]
            uid: Option<serde_json::Value>,
        }

        let state = self.app.state::<AppState>();
        let client = state.get_client();
        let resp = client
            .get("https://m.weibo.cn/api/config")
            .header("Cookie", cookie)
            .header("Referer", "https://m.weibo.cn/")
            .header("MWeibo-Pwa", "1")
            .header("X-Requested-With", "XMLHttpRequest")
            .send()
            .await
            .ok()?;

        if !resp.status().is_success() {
            return None;
        }
        let body = resp.text().await.ok()?;
        let parsed: MobileConfigResp = serde_json::from_str(&body).ok()?;
        let data = parsed.data?;
        if data.login == Some(false) {
            return None;
        }
        match data.uid? {
            serde_json::Value::String(s) if !s.trim().is_empty() && s.chars().all(|c| c.is_ascii_digit()) => Some(s),
            serde_json::Value::Number(n) => Some(n.to_string()),
            _ => None,
        }
    }

    async fn fetch_uid_from_weibo_home(&self, cookie: &str) -> Option<String> {
        let state = self.app.state::<AppState>();
        let client = state.get_client();
        let resp = client
            .get("https://weibo.com/")
            .header("Cookie", cookie)
            .header("Referer", "https://weibo.com/")
            .send()
            .await
            .ok()?;

        if !resp.status().is_success() {
            return None;
        }
        let html = resp.text().await.ok()?;
        extract_uid_from_weibo_html(&html)
    }

    pub async fn get_history_map(&self, uid: &str) -> Result<RawHistoryMap, AppError> {
        self.get_json("/ajax/profile/mbloghistory", &[("uid", uid.to_string())])
            .await
    }

    pub async fn fetch_posts_page(
        &self,
        uid: &str,
        page: i64,
        starttime: Option<i64>,
        endtime: Option<i64>,
    ) -> Result<(Vec<RawPost>, i64), AppError> {
        let mut query = vec![
            ("uid", uid.to_string()),
            ("page", page.to_string()),
        ];
        if let Some(ts) = starttime {
            query.push(("starttime", ts.to_string()));
        }
        if let Some(ts) = endtime {
            query.push(("endtime", ts.to_string()));
        }

        let raw: RawSearchProfile = self.get_json("/ajax/statuses/searchProfile", &query).await?;

        let list = raw.data.list.unwrap_or_default();
        let total = raw.data.total.unwrap_or(0);

        #[cfg(debug_assertions)]
        eprintln!("[DEBUG] fetch_posts_page: page={}, returned {} posts, total={}", page, list.len(), total);

        Ok((list, total))
    }

    pub async fn fetch_favorites_page(
        &self,
        page: i64,
        starttime: Option<i64>,
        endtime: Option<i64>,
    ) -> Result<(Vec<RawPost>, i64), AppError> {
        let mut query = vec![("page", page.to_string())];
        if let Some(ts) = starttime {
            query.push(("starttime", ts.to_string()));
        }
        if let Some(ts) = endtime {
            query.push(("endtime", ts.to_string()));
        }

        let raw: RawFavProfile = self.get_json("/ajax/favorites/all_fav", &query).await?;

        let list = raw.data.unwrap_or_default();
        let total = if list.is_empty() { 0 } else { -1 };
        Ok((list, total))
    }

    pub async fn get_long_text(&self, mblogid: &str) -> Result<String, AppError> {
        let raw: RawLongText = self
            .get_json("/ajax/statuses/longtext", &[("id", mblogid.to_string())])
            .await?;

        if !raw.data.long_text_content.is_empty() {
            return Ok(raw.data.long_text_content);
        }
        if let Some(raw_text) = raw.data.raw_text {
            if !raw_text.is_empty() {
                return Ok(raw_text);
            }
        }

        Err(AppError::ApiError(format!("长微博内容为空: {mblogid}")))
    }

    pub fn normalize_post(raw: &RawPost, uid: &str) -> WeiboPost {
        let author = raw
            .user
            .as_ref()
            .map(|u| u.screen_name.clone())
            .unwrap_or_default();

        let is_repost = raw.retweeted_status.is_some();
        let repost_user = raw
            .retweeted_status
            .as_ref()
            .and_then(|r| r.user.as_ref())
            .map(|u| u.screen_name.clone());
        let repost_text = raw
            .retweeted_status
            .as_ref()
            .map(|r| r.text.clone())
            .filter(|t| !t.trim().is_empty());

        let region = raw.region_name.clone();
        let images = Self::parse_images(&raw.pic_infos);
        let uid_for_url = if uid.is_empty() {
            raw.user.as_ref().map(|u| u.id.to_string()).unwrap_or_else(|| "unknown".to_string())
        } else {
            uid.to_string()
        };
        let source_url = format!("https://weibo.com/{}/{}", uid_for_url, raw.mblogid);
        let tags = Self::extract_tags(raw);

        WeiboPost {
            mblogid: raw.mblogid.clone(),
            created_at: raw.created_at.clone(),
            text: raw.text.clone(),
            images,
            is_repost,
            repost_user,
            repost_text,
            region,
            source_url,
            author,
            tags,
        }
    }

    fn extract_tags(raw: &RawPost) -> Vec<String> {
        let mut tags = Vec::new();

        if let Some(tag_struct) = &raw.tag_struct {
            for item in tag_struct {
                if let Some(name) = item.get("tagName").and_then(|v| v.as_str()) {
                    tags.push(name.to_string());
                }
            }
        }

        if let Some(topic_struct) = &raw.topic_struct {
            if let Some(arr) = topic_struct.as_array() {
                for item in arr {
                    if let Some(name) = item
                        .get("title")
                        .or_else(|| item.get("topic_title"))
                        .and_then(|v| v.as_str())
                    {
                        tags.push(name.to_string());
                    }
                }
            }
        }

        tags.extend(extract_hashtags_from_text(&raw.text));

        tags.sort();
        tags.dedup();
        tags
    }

    fn parse_images(pic_infos: &Option<serde_json::Value>) -> Vec<WeiboImage> {
        let Some(obj) = pic_infos else {
            return vec![];
        };
        let Some(map) = obj.as_object() else {
            return vec![];
        };

        map.values()
            .filter_map(|v| {
                let large = v.get("large")?;
                let url = large.get("url")?.as_str()?;
                let width = large.get("width")?.as_u64()? as u32;
                let height = large.get("height")?.as_u64()? as u32;
                Some(WeiboImage {
                    original_url: if url.starts_with("http") {
                        url.to_string()
                    } else {
                        format!("https:{url}")
                    },
                    local_path: None,
                    width,
                    height,
                })
            })
            .collect()
    }

    pub async fn fetch_all_posts<F>(
        &self,
        uid: &str,
        starttime: Option<i64>,
        endtime: Option<i64>,
        mut on_page: F,
    ) -> Result<Vec<RawPost>, AppError>
    where
        F: FnMut(usize, usize) -> bool,
    {
        let mut all_posts = Vec::new();
        let mut page: i64 = 1;
        let mut total = 0i64;

        loop {
            let (posts, total_count) = self.fetch_posts_page(uid, page, starttime, endtime).await?;

            if total == 0 {
                total = total_count;
            }

            let total_pages = if total > 0 {
                (total as f64 / posts.len().max(1) as f64).ceil() as usize
            } else {
                1
            };

            if posts.is_empty() {
                break;
            }

            all_posts.extend(posts);

            if !on_page(page as usize, total_pages.max(all_posts.len())) {
                break;
            }

            page += 1;
        }

        Ok(all_posts)
    }
}

fn extract_hashtags_from_text(text: &str) -> Vec<String> {
    let mut result = Vec::new();
    let chars: Vec<char> = text.chars().collect();
    let len = chars.len();
    let mut i = 0;

    while i < len {
        if chars[i] == '#' {
            let start = i + 1;
            let mut end = start;
            while end < len && chars[end] != '#' {
                end += 1;
            }
            if end < len && end > start {
                let tag: String = chars[start..end].iter().collect();
                // Skip HTML tags that got captured (contains < or >)
                if !tag.contains('<') && !tag.contains('>') {
                    result.push(tag);
                }
                i = end + 1;
                continue;
            }
        }
        i += 1;
    }

    result
}

fn progress_phase_for_path(path: &str) -> ProgressPhase {
    match path {
        "/ajax/profile/info" => ProgressPhase::FetchingUserInfo,
        "/ajax/statuses/longtext" => ProgressPhase::FetchingLongText,
        "/ajax/profile/mbloghistory" | "/ajax/statuses/searchProfile" => {
            ProgressPhase::FetchingPostList
        }
        _ => ProgressPhase::FetchingPostList,
    }
}

fn is_retryable_status(status: StatusCode) -> bool {
    status == StatusCode::TOO_MANY_REQUESTS || status.is_server_error()
}

fn is_network_error(error: &AppError) -> bool {
    matches!(error, AppError::Network(_))
}

fn remaining_throttle_delay(elapsed: Duration, min_interval: Duration) -> Option<Duration> {
    min_interval.checked_sub(elapsed).filter(|d| !d.is_zero())
}

fn preview_str(text: &str, max_chars: usize) -> String {
    text.chars().take(max_chars).collect()
}

fn retry_delay(config: &RetryConfig, attempt: u32) -> Duration {
    let multiplier = 2u32.saturating_pow(attempt.saturating_sub(1));
    config
        .initial_delay
        .saturating_mul(multiplier)
        .min(config.max_delay)
}

fn cookie_file_path<R: Runtime>(app: &AppHandle<R>) -> PathBuf {
    app.path()
        .app_data_dir()
        .unwrap_or_default()
        .join(COOKIE_FILE_NAME)
}

#[derive(serde::Serialize, serde::Deserialize, Clone, Default)]
pub struct SavedSession {
    pub cookie: String,
    #[serde(default)]
    pub uid: Option<String>,
    #[serde(default)]
    pub screen_name: Option<String>,
}

pub fn load_saved_session<R: Runtime>(app: &AppHandle<R>) -> Option<SavedSession> {
    let path = cookie_file_path(app);
    if !path.exists() {
        return None;
    }

    let content = fs::read_to_string(path).ok()?;
    let data: SavedSession = serde_json::from_str(&content).ok()?;
    if data.cookie.trim().is_empty() {
        return None;
    }
    Some(data)
}

pub fn load_saved_cookie<R: Runtime>(app: &AppHandle<R>) -> Option<String> {
    load_saved_session(app).map(|s| s.cookie)
}

pub fn extract_uid_from_weibo_html(html: &str) -> Option<String> {
    let patterns = [
        "$CONFIG['uid']='",
        "$CONFIG[\"uid\"]=\"",
        "\"uid\":\"",
        "\"uid\": \"",
        "\"idstr\":\"",
    ];
    for pattern in patterns {
        if let Some(pos) = html.find(pattern) {
            let rest = &html[pos + pattern.len()..];
            let digits: String = rest.chars().take_while(|c| c.is_ascii_digit()).collect();
            if digits.len() >= 5 && digits.len() <= 20 {
                return Some(digits);
            }
        }
    }
    None
}

pub fn is_login_redirect_success(url_str: &str) -> bool {
    let lower = url_str.to_ascii_lowercase();
    let is_weibo_home = lower.starts_with("https://weibo.com")
        || lower.starts_with("https://www.weibo.com")
        || lower.starts_with("https://m.weibo.cn");
    is_weibo_home
        && !lower.contains("passport")
        && !lower.contains("signin")
        && !lower.contains("/login")
}

pub fn apply_saved_cookie_to_state(state: &AppState, cookie: Option<String>) -> Option<String> {
    let restored = cookie.and_then(|value| {
        let trimmed = value.trim();
        if trimmed.is_empty() {
            None
        } else {
            Some(trimmed.to_string())
        }
    });

    match restored {
        Some(ref value) => state.set_cookie(value.clone()),
        None => state.set_cookie(String::new()),
    }

    restored
}

pub fn restore_saved_cookie<R: Runtime>(app: &AppHandle<R>, state: &AppState) -> Option<String> {
    let restored = load_saved_cookie(app);
    apply_saved_cookie_to_state(state, restored)
}

pub fn is_auth_invalid_response(status: reqwest::StatusCode, body: &str) -> bool {
    let body_lower = body.to_ascii_lowercase();

    // Check for HTTP status codes that indicate auth failure
    if status == reqwest::StatusCode::UNAUTHORIZED || status == reqwest::StatusCode::FORBIDDEN {
        return true;
    }

    // Check for Weibo-specific error codes
    if body_lower.contains("retcode=6102") {
        return true;
    }

    // Check if the response is an HTML page (indicating a redirect to login page)
    // This happens when the session cookie is invalid
    if body_lower.contains("<!doctype html")
        && body_lower.contains("window.wbbotdetector")
        && body_lower.contains("weibo.com/favicon.ico") {
        return true;
    }

    // Check for passport.weibo.com in the body, but be more specific
    // Only trigger if it's clearly a login/redirect page, not just a reference
    if body_lower.contains("passport.weibo.com") && body_lower.contains("<!doctype html") {
        return true;
    }

    false
}

fn write_saved_session<R: Runtime>(app: &AppHandle<R>, session: &SavedSession) {
    let path = cookie_file_path(app);
    if let Some(parent) = path.parent() {
        let _ = fs::create_dir_all(parent);
    }
    if let Ok(content) = serde_json::to_string_pretty(session) {
        let tmp_path = path.with_extension("json.tmp");
        if fs::write(&tmp_path, &content).is_ok() {
            #[cfg(unix)]
            {
                use std::os::unix::fs::PermissionsExt;
                let _ = fs::set_permissions(&tmp_path, fs::Permissions::from_mode(0o600));
            }
            if fs::rename(&tmp_path, &path).is_err() {
                let _ = fs::remove_file(&path);
                let _ = fs::rename(&tmp_path, &path);
            }
        }
    }
}

pub fn save_cookie<R: Runtime>(app: &AppHandle<R>, cookie: &str) {
    let existing = load_saved_session(app).filter(|s| s.cookie == cookie);
    let session = SavedSession {
        cookie: cookie.to_string(),
        uid: existing.as_ref().and_then(|s| s.uid.clone()),
        screen_name: existing.and_then(|s| s.screen_name),
    };
    write_saved_session(app, &session);
}

pub fn save_session_user<R: Runtime>(app: &AppHandle<R>, uid: &str, screen_name: &str) {
    let Some(mut session) = load_saved_session(app) else {
        return;
    };
    session.uid = Some(uid.to_string());
    session.screen_name = Some(screen_name.to_string());
    write_saved_session(app, &session);
}

pub fn clear_saved_cookie<R: Runtime>(app: &AppHandle<R>) {
    let _ = fs::remove_file(cookie_file_path(app));
}

#[cfg(test)]
mod tests {
    use super::{is_retryable_status, preview_str, remaining_throttle_delay, retry_delay, RetryConfig};
    use std::time::Duration;

    #[test]
    fn remaining_throttle_delay_never_underflows() {
        let min = Duration::from_millis(1000);
        assert_eq!(
            remaining_throttle_delay(Duration::from_millis(300), min),
            Some(Duration::from_millis(700))
        );
        assert_eq!(remaining_throttle_delay(Duration::from_millis(1000), min), None);
        assert_eq!(remaining_throttle_delay(Duration::from_millis(2500), min), None);
    }

    #[test]
    fn preview_str_handles_multibyte_utf8_safely() {
        let chinese = "中文错误响应内容".repeat(50);
        assert_eq!(preview_str(&chinese, 300).chars().count(), 300);
    }

    #[test]
    fn extract_uid_from_weibo_html_supports_common_formats() {
        assert_eq!(
            super::extract_uid_from_weibo_html(r#"$CONFIG['uid']='2166767661';"#).as_deref(),
            Some("2166767661")
        );
        assert_eq!(
            super::extract_uid_from_weibo_html(r#"{"uid":"1738498871","ok":1}"#).as_deref(),
            Some("1738498871")
        );
        assert_eq!(super::extract_uid_from_weibo_html("<html>no uid</html>"), None);
    }

    #[test]
    fn is_login_redirect_success_matches_desktop_and_mobile_weibo() {
        assert!(super::is_login_redirect_success("https://weibo.com/u/123456"));
        assert!(super::is_login_redirect_success("https://www.weibo.com/"));
        assert!(super::is_login_redirect_success("https://m.weibo.cn/"));
        assert!(!super::is_login_redirect_success("https://passport.weibo.com/sso/signin?entry=miniblog"));
        assert!(!super::is_login_redirect_success("https://m.weibo.cn/login"));
    }

    #[test]
    fn retry_config_defaults_match_expected_backoff() {
        let config = RetryConfig::default();
        assert_eq!(config.max_retries, 3);
        assert_eq!(config.initial_delay, Duration::from_secs(2));
        assert_eq!(config.max_delay, Duration::from_secs(30));
    }

    #[test]
    fn retries_only_transient_http_statuses() {
        assert!(is_retryable_status(reqwest::StatusCode::TOO_MANY_REQUESTS));
        assert!(is_retryable_status(reqwest::StatusCode::BAD_GATEWAY));
        assert!(is_retryable_status(reqwest::StatusCode::SERVICE_UNAVAILABLE));
        assert!(!is_retryable_status(reqwest::StatusCode::UNAUTHORIZED));
        assert!(!is_retryable_status(reqwest::StatusCode::FORBIDDEN));
        assert!(!is_retryable_status(reqwest::StatusCode::BAD_REQUEST));
    }

    #[test]
    fn retry_delay_uses_exponential_backoff_with_cap() {
        let config = RetryConfig::default();
        assert_eq!(retry_delay(&config, 1), Duration::from_secs(2));
        assert_eq!(retry_delay(&config, 2), Duration::from_secs(4));
        assert_eq!(retry_delay(&config, 3), Duration::from_secs(8));

        let capped = RetryConfig {
            max_retries: 5,
            initial_delay: Duration::from_secs(20),
            max_delay: Duration::from_secs(30),
        };
        assert_eq!(retry_delay(&capped, 2), Duration::from_secs(30));
        assert_eq!(retry_delay(&capped, 5), Duration::from_secs(30));
    }
}

const WEIBO_QR_ONLY_SCRIPT: &str = r#"
(function () {
  if (
    window.location.hostname !== 'passport.weibo.com' ||
    !window.location.pathname.includes('/sso/signin')
  ) {
    return;
  }

  const STYLE_ID = '__wecun_qr_only_style';
  let qrOpened = false;

  function ensureStyle() {
    if (document.getElementById(STYLE_ID)) return;
    const style = document.createElement('style');
    style.id = STYLE_ID;
    style.textContent = `
      html.__wecun_qr_loading #app { opacity: 0 !important; pointer-events: none !important; }
      div:has(> img[src*="h5_fanhui"]) { display: none !important; }
    `;
    (document.head || document.documentElement).appendChild(style);
  }

  document.documentElement.classList.add('__wecun_qr_loading');

  function reveal() {
    document.documentElement.classList.remove('__wecun_qr_loading');
  }

  function hideBackButtons() {
    document.querySelectorAll('img[src*="h5_fanhui"]').forEach((img) => {
      if (img.parentElement) img.parentElement.style.display = 'none';
    });
    document.querySelectorAll('div, span, button, a').forEach((el) => {
      if (el.children.length <= 1 && (el.textContent || '').trim() === '返回') {
        el.style.display = 'none';
      }
    });
  }

  function isQrVisible() {
    const bodyText = document.body ? document.body.textContent || '' : '';
    return (
      bodyText.includes('打开微博手机APP') ||
      !!document.querySelector('[class*="z-9999"]')
    );
  }

  function injectOfficialAppTip() {
    if (document.getElementById('__wecun_official_app_tip')) return;
    const all = document.querySelectorAll('div');
    for (const el of all) {
      if (el.children.length === 0 && (el.textContent || '').includes('打开微博手机APP')) {
        const tip = document.createElement('div');
        tip.id = '__wecun_official_app_tip';
        tip.textContent = '请使用微博官方版 App 扫码（暂不支持轻享版）';
        tip.style.cssText =
          'margin-top:8px;font-size:12px;color:#b45309;background:#fef3c7;border-radius:6px;padding:4px 10px;display:inline-block;';
        el.insertAdjacentElement('afterend', tip);
        break;
      }
    }
  }

  function tryOpenQr() {
    ensureStyle();
    hideBackButtons();

    if (isQrVisible()) {
      qrOpened = true;
      injectOfficialAppTip();
      reveal();
      return true;
    }

    const scanImg = document.querySelector('img[src*="scan.png"]');
    if (scanImg && scanImg.parentElement) {
      scanImg.parentElement.click();
      return false;
    }

    const candidates = document.querySelectorAll('span, button, a, div');
    for (const el of candidates) {
      if (el.children.length <= 1 && (el.textContent || '').trim() === '扫码登录') {
        el.click();
        return false;
      }
    }
    return false;
  }

  function start() {
    ensureStyle();
    tryOpenQr();
    const observer = new MutationObserver(() => {
      tryOpenQr();
    });
    if (document.documentElement) {
      observer.observe(document.documentElement, { childList: true, subtree: true });
    }
    const timer = setInterval(() => {
      tryOpenQr();
      if (qrOpened) {
        clearInterval(timer);
      }
    }, 100);
    setTimeout(() => {
      reveal();
      clearInterval(timer);
    }, 5000);
  }

  if (document.readyState === 'loading') {
    document.addEventListener('DOMContentLoaded', start, { once: true });
  } else {
    start();
  }
})();
"#;

pub async fn open_login_window(app: AppHandle) -> Result<(), String> {
    // Close any existing login window first to avoid conflicts when switching accounts
    if let Some(existing_win) = app.get_webview_window(LOGIN_WINDOW_LABEL) {
        let _ = existing_win.close();
        // Give it a moment to close properly
        tokio::time::sleep(Duration::from_millis(120)).await;
    }

    let app_clone = app.clone();
    let login_completed = Arc::new(AtomicBool::new(false));
    let login_completed_nav = Arc::clone(&login_completed);
    let login_completed_close = Arc::clone(&login_completed);
    let app_for_close = app.clone();

    let window = WebviewWindowBuilder::new(
        &app,
        LOGIN_WINDOW_LABEL,
        WebviewUrl::External("https://passport.weibo.com/sso/signin?entry=miniblog".parse().unwrap()),
    )
    .title("微博扫码登录")
    .inner_size(420.0, 520.0)
    .resizable(false)
    .center()
    .initialization_script(WEIBO_QR_ONLY_SCRIPT)
    .on_navigation(move |url: &Url| {
        let url_str = url.as_str();
        // When redirected to weibo.com / www.weibo.com / m.weibo.cn after QR scan
        if is_login_redirect_success(url_str) && !login_completed_nav.swap(true, Ordering::SeqCst) {
            let app = app_clone.clone();

            // Hide window immediately to prevent visible redirect
            if let Some(win) = app.get_webview_window(LOGIN_WINDOW_LABEL) {
                let _ = win.hide();
            }

            tauri::async_runtime::spawn(async move {
                // Wait briefly to ensure all cookies are properly written by WebView
                tokio::time::sleep(Duration::from_millis(800)).await;

                if let Some(win) = app.get_webview_window(LOGIN_WINDOW_LABEL) {
                    if let Ok(cookies) = win.cookies() {
                        let cookie_string: Vec<String> = cookies
                            .iter()
                            .filter(|c| {
                                let domain = c.domain().unwrap_or("");
                                domain.contains("weibo") || domain.contains(".sina.cn")
                            })
                            .map(|c| format!("{}={}", c.name(), c.value()))
                            .collect();

                        if !cookie_string.is_empty() {
                            let full_cookie = cookie_string.join("; ");

                            #[cfg(debug_assertions)]
                            eprintln!("[DEBUG] Captured {} cookies", cookie_string.len());

                            let state = app.state::<AppState>();
                            state.set_cookie(full_cookie.clone());
                            save_cookie(&app, &full_cookie);
                            let _ = app.emit("cookie-received", full_cookie);
                        } else {
                            #[cfg(debug_assertions)]
                            eprintln!("[DEBUG] No cookies captured after login!");
                        }
                    }

                    let _ = win.close();
                    let _ = app.emit("login-success", "");
                }
            });
        }
        true
    })
    .build()
    .map_err(|e| e.to_string())?;

    window.on_window_event(move |event| {
        if matches!(event, WindowEvent::Destroyed) && !login_completed_close.load(Ordering::SeqCst) {
            let _ = app_for_close.emit("login-window-closed", "");
        }
    });

    Ok(())
}
