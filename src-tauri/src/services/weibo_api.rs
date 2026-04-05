use std::fs;
use std::path::PathBuf;
use std::time::{Duration, Instant};

use tauri::{AppHandle, Emitter, Manager, Runtime, Url, WebviewUrl, WebviewWindowBuilder};
use tauri_plugin_stronghold::stronghold::Stronghold;
use tokio::sync::Mutex;

use crate::error::AppError;
use crate::models::{
    RawHistoryMap, RawLongText, RawPost, RawSearchProfile, RawUserInfo, UserProfile, WeiboImage,
    WeiboPost,
};
use crate::state::AppState;

const LOGIN_WINDOW_LABEL: &str = "weibo-login";
const WEIBO_BASE: &str = "https://weibo.com";
const MIN_REQUEST_INTERVAL: Duration = Duration::from_millis(1000);
const COOKIE_FILE_NAME: &str = "weibo_cookie.dat";
const STRONGHOLD_FILE_NAME: &str = "cookie_vault.tauri";
const STRONGHOLD_PASSWORD: &str = "weibo-downloader-vault-key";
const STRONGHOLD_CLIENT_NAME: &[u8] = b"weibo-cookie-client";
const COOKIE_STORE_KEY: &[u8] = b"weibo-session-cookie";

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

    async fn throttle(&self) {
        let mut guard = self.last_fetch.lock().await;
        if let Some(last) = *guard {
            let elapsed = last.elapsed();
            if elapsed < MIN_REQUEST_INTERVAL {
                tokio::time::sleep(MIN_REQUEST_INTERVAL - elapsed).await;
            }
        }
        *guard = Some(Instant::now());
    }

    async fn get_json<T: serde::de::DeserializeOwned>(
        &self,
        path: &str,
        query: &[(&str, String)],
    ) -> Result<T, AppError> {
        self.throttle().await;

        let state = self.app.state::<AppState>();
        let cookie = state.get_cookie();
        if cookie.is_empty() {
            return Err(AppError::ApiError("请先登录".to_string()));
        }

        let client = state.http_client.clone();

        let mut url = format!("{WEIBO_BASE}{path}");
        if !query.is_empty() {
            let params: Vec<String> = query
                .iter()
                .map(|(k, v)| format!("{}={}", urlencoding::encode(k), urlencoding::encode(v)))
                .collect();
            url.push('?');
            url.push_str(&params.join("&"));
        }

        let resp = client
            .get(&url)
            .header("Cookie", &cookie)
            .header("Referer", "https://weibo.com/")
            .send()
            .await
            .map_err(|e| AppError::ApiError(format!("网络请求失败: {e}")))?;

        let status = resp.status();
        let text = resp.text().await.map_err(AppError::Network)?;

        if is_auth_invalid_response(status, &text) {
            state.set_cookie(String::new());
            clear_saved_cookie(&self.app);
            let _ = self.app.emit("login-invalid", "登录已失效，请重新登录");
            return Err(AppError::InvalidCookie);
        }

        if !status.is_success() {
            return Err(AppError::ApiError(format!(
                "HTTP {status} from {path}: {}",
                &text[..text.len().min(300)]
            )));
        }

        serde_json::from_str(&text)
            .map_err(|e| AppError::Parse(format!("解析 {path} 失败: {e}\nBody: {}", &text[..text.len().min(300)])))
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

        let region = raw.region_name.clone();
        let images = Self::parse_images(&raw.pic_infos);
        let source_url = format!("https://weibo.com/{}/{}", uid, raw.mblogid);
        let tags = Self::extract_tags(raw);

        WeiboPost {
            mblogid: raw.mblogid.clone(),
            created_at: raw.created_at.clone(),
            text: raw.text.clone(),
            images,
            is_repost,
            repost_user,
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

fn cookie_dir<R: Runtime>(app: &AppHandle<R>) -> PathBuf {
    app.path()
        .app_data_dir()
        .unwrap_or_else(|_| std::env::temp_dir().join("weibo-downloader"))
}

fn cookie_file_path<R: Runtime>(app: &AppHandle<R>) -> PathBuf {
    cookie_dir(app).join(COOKIE_FILE_NAME)
}

fn stronghold_cookie_path<R: Runtime>(app: &AppHandle<R>) -> PathBuf {
    cookie_dir(app).join(STRONGHOLD_FILE_NAME)
}

pub(crate) fn stronghold_password_hash(password: &str) -> Vec<u8> {
    use std::collections::hash_map::DefaultHasher;
    use std::hash::{Hash, Hasher};

    let mut output = Vec::with_capacity(32);
    for index in 0..4u64 {
        let mut hasher = DefaultHasher::new();
        password.hash(&mut hasher);
        index.hash(&mut hasher);
        output.extend_from_slice(&hasher.finish().to_le_bytes());
    }
    output
}

fn stronghold_key() -> Vec<u8> {
    stronghold_password_hash(STRONGHOLD_PASSWORD)
}

fn open_stronghold_for_read<R: Runtime>(app: &AppHandle<R>) -> Option<Stronghold> {
    Stronghold::new(stronghold_cookie_path(app), stronghold_key()).ok()
}

fn open_stronghold_for_write<R: Runtime>(app: &AppHandle<R>) -> Option<Stronghold> {
    let path = stronghold_cookie_path(app);
    let parent = path.parent()?;
    fs::create_dir_all(parent).ok()?;

    match Stronghold::new(&path, stronghold_key()) {
        Ok(stronghold) => Some(stronghold),
        Err(_) => {
            let _ = fs::remove_file(&path);
            Stronghold::new(path, stronghold_key()).ok()
        }
    }
}

fn load_cookie_from_stronghold<R: Runtime>(app: &AppHandle<R>) -> Option<String> {
    let stronghold = open_stronghold_for_read(app)?;
    let client = stronghold.load_client(STRONGHOLD_CLIENT_NAME).ok()?;
    let value = client.store().get(COOKIE_STORE_KEY).ok().flatten()?;
    String::from_utf8(value).ok().map(|cookie| cookie.trim().to_string())
}

fn load_legacy_plaintext_cookie<R: Runtime>(app: &AppHandle<R>) -> Option<String> {
    let path = cookie_file_path(app);
    if !path.exists() {
        return None;
    }

    fs::read_to_string(path)
        .ok()
        .map(|cookie| cookie.trim().to_string())
}

fn persist_cookie_to_stronghold<R: Runtime>(app: &AppHandle<R>, cookie: &str) -> bool {
    let Some(stronghold) = open_stronghold_for_write(app) else {
        return false;
    };

    let client = stronghold
        .load_client(STRONGHOLD_CLIENT_NAME)
        .or_else(|_| stronghold.create_client(STRONGHOLD_CLIENT_NAME));

    let Ok(client) = client else {
        return false;
    };

    if client
        .store()
        .insert(COOKIE_STORE_KEY.to_vec(), cookie.as_bytes().to_vec(), None)
        .is_err()
    {
        return false;
    }

    stronghold.save().is_ok()
}

pub fn load_saved_cookie<R: Runtime>(app: &AppHandle<R>) -> Option<String> {
    load_cookie_from_stronghold(app)
        .filter(|cookie| !cookie.is_empty())
        .or_else(|| load_legacy_plaintext_cookie(app).filter(|cookie| !cookie.is_empty()))
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
    let restored = load_cookie_from_stronghold(app)
        .filter(|cookie| !cookie.is_empty())
        .or_else(|| {
            let legacy_cookie = load_legacy_plaintext_cookie(app)?;
            if legacy_cookie.is_empty() {
                return None;
            }

            if persist_cookie_to_stronghold(app, &legacy_cookie) {
                let _ = fs::remove_file(cookie_file_path(app));
            }

            Some(legacy_cookie)
        });

    apply_saved_cookie_to_state(state, restored)
}

pub fn is_auth_invalid_response(status: reqwest::StatusCode, body: &str) -> bool {
    let body_lower = body.to_ascii_lowercase();
    status == reqwest::StatusCode::UNAUTHORIZED
        || status == reqwest::StatusCode::FORBIDDEN
        || body_lower.contains("retcode=6102")
        || body_lower.contains("passport.weibo.com")
        || body_lower.contains("<!doctype html")
            && body_lower.contains("window.wbbotdetector")
            && body_lower.contains("weibo.com/favicon.ico")
}

pub fn save_cookie<R: Runtime>(app: &AppHandle<R>, cookie: &str) {
    if persist_cookie_to_stronghold(app, cookie) {
        let _ = fs::remove_file(cookie_file_path(app));
    }
}

pub fn clear_saved_cookie<R: Runtime>(app: &AppHandle<R>) {
    if let Some(stronghold) = open_stronghold_for_write(app) {
        if let Ok(client) = stronghold.load_client(STRONGHOLD_CLIENT_NAME) {
            let _ = client.store().delete(COOKIE_STORE_KEY);
            let _ = stronghold.save();
        }
    }
    let _ = fs::remove_file(cookie_file_path(app));
}

pub async fn open_login_window(app: AppHandle) -> Result<(), String> {
    let app_clone = app.clone();

    let _ = WebviewWindowBuilder::new(
        &app,
        LOGIN_WINDOW_LABEL,
        WebviewUrl::External("https://passport.weibo.com/sso/signin".parse().unwrap()),
    )
    .title("登录微博")
    .inner_size(480.0, 560.0)
    .resizable(false)
    .center()
    .on_navigation(move |url: &Url| {
        let url_str = url.as_str();
        if url_str.starts_with("https://weibo.com")
            && !url_str.contains("passport")
            && !url_str.contains("signin")
        {
            let app = app_clone.clone();
            tauri::async_runtime::spawn(async move {
                tokio::time::sleep(Duration::from_secs(1)).await;

                if let Some(win) = app.get_webview_window(LOGIN_WINDOW_LABEL) {
                    if let Ok(cookies) = win.cookies() {
                        let cookie_string: Vec<String> = cookies
                            .iter()
                            .map(|c| format!("{}={}", c.name(), c.value()))
                            .collect();

                        if !cookie_string.is_empty() {
                            let full_cookie = cookie_string.join("; ");
                            let state = app.state::<AppState>();
                            state.set_cookie(full_cookie.clone());
                            save_cookie(&app, &full_cookie);
                            let _ = app.emit("cookie-received", full_cookie);
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

    Ok(())
}
