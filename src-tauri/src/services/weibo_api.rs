use std::sync::Mutex;
use std::time::{Duration, Instant};

use tauri::{AppHandle, Manager};

use crate::error::AppError;
use crate::models::{
    RawHistoryMap, RawLongText, RawPost, RawSearchProfile, RawUserInfo, UserProfile, WeiboImage,
    WeiboPost,
};
use crate::state::AppState;

const LOGIN_WINDOW_LABEL: &str = "weibo-login";
const WEIBO_BASE: &str = "https://weibo.com";
const MIN_REQUEST_INTERVAL: Duration = Duration::from_millis(1000);
const API_RESULT_EVENT: &str = "weibo-api-result";
const API_TIMEOUT: Duration = Duration::from_secs(30);
const API_POLL_INTERVAL: Duration = Duration::from_millis(100);

pub struct WeiboApiClient {
    app: AppHandle,
    last_fetch: Mutex<Option<Instant>>,
}

pub async fn weibo_api_call<K, V>(
    app: &AppHandle,
    path: &str,
    query: &[(K, V)],
) -> Result<String, AppError>
where
    K: AsRef<str>,
    V: AsRef<str>,
{
    let state = app.state::<AppState>();
    if !state.is_api_window_ready() {
        return Err(AppError::ApiError(
            "请先登录微博以建立 API 会话".to_string(),
        ));
    }

    let window = app
        .get_webview_window(LOGIN_WINDOW_LABEL)
        .ok_or_else(|| AppError::ApiError("微博登录窗口不存在，请重新登录".to_string()))?;

    let mut url = format!("{WEIBO_BASE}{path}");
    if !query.is_empty() {
        let params = query
            .iter()
            .map(|(key, value)| {
                format!(
                    "{}={}",
                    urlencoding::encode(key.as_ref()),
                    urlencoding::encode(value.as_ref())
                )
            })
            .collect::<Vec<_>>()
            .join("&");
        url.push('?');
        url.push_str(&params);
    }

    let request_id = state.next_api_request_id();
    state.clear_api_result(&request_id);

    let escaped_url = serde_json::to_string(&url)
        .map_err(|e| AppError::ApiError(format!("无法序列化请求 URL: {e}")))?;
    let escaped_request_id = serde_json::to_string(&request_id)
        .map_err(|e| AppError::ApiError(format!("无法序列化请求 ID: {e}")))?;

    let script = format!(
        r#"(async () => {{
  const requestId = {escaped_request_id};
  const emitResult = (payload) => {{
    if (window.__TAURI__?.event?.emit) {{
      window.__TAURI__.event.emit('{API_RESULT_EVENT}', JSON.stringify(payload));
      return;
    }}

    if (window.__TAURI_INTERNALS__?.postMessage) {{
      window.__TAURI_INTERNALS__.postMessage({{
        cmd: 'plugin:event|emit',
        payload: {{
          event: '{API_RESULT_EVENT}',
          payload: JSON.stringify(payload)
        }}
      }});
      return;
    }}

    throw new Error('Tauri event bridge unavailable in WebView');
  }};

  try {{
    const response = await fetch({escaped_url}, {{
      method: 'GET',
      credentials: 'include',
      headers: {{
        'Accept': 'application/json, text/plain, */*',
        'Referer': 'https://weibo.com/',
        'X-Requested-With': 'XMLHttpRequest'
      }}
    }});

    const body = await response.text();
    emitResult({{
      requestId,
      ok: response.ok,
      status: response.status,
      body,
      error: response.ok ? null : `HTTP ${{response.status}}`
    }});
  }} catch (error) {{
    emitResult({{
      requestId,
      ok: false,
      status: null,
      body: null,
      error: error instanceof Error ? error.message : String(error)
    }});
  }}
}})();"#
    );

    window
        .eval(&script)
        .map_err(|e| AppError::ApiError(format!("执行 WebView 请求失败: {e}")))?;

    let deadline = Instant::now() + API_TIMEOUT;
    loop {
        if let Some(result) = state.take_api_result(&request_id) {
            if result.ok {
                if let Some(body) = result.body {
                    return Ok(body);
                }

                return Err(AppError::ApiError("微博 API 返回空响应体".to_string()));
            }

            let status_prefix = result
                .status
                .map(|status| format!("HTTP {status}: "))
                .unwrap_or_default();
            let message = result
                .error
                .or(result.body)
                .unwrap_or_else(|| "微博 API 调用失败".to_string());
            return Err(AppError::ApiError(format!("{status_prefix}{message}")));
        }

        if Instant::now() >= deadline {
            state.clear_api_result(&request_id);
            return Err(AppError::ApiError("等待微博 API 响应超时".to_string()));
        }

        tokio::time::sleep(API_POLL_INTERVAL).await;
    }
}

impl WeiboApiClient {
    pub fn new(app: AppHandle) -> Self {
        Self {
            app,
            last_fetch: Mutex::new(None),
        }
    }

    fn throttle(&self) {
        let mut guard = self.last_fetch.lock().unwrap();
        if let Some(last) = *guard {
            let elapsed = last.elapsed();
            if elapsed < MIN_REQUEST_INTERVAL {
                std::thread::sleep(MIN_REQUEST_INTERVAL - elapsed);
            }
        }
        *guard = Some(Instant::now());
    }

    async fn get_json<T: serde::de::DeserializeOwned>(
        &self,
        path: &str,
        query: &[(&str, String)],
    ) -> Result<T, AppError> {
        self.throttle();

        let text = weibo_api_call(&self.app, path, query).await?;
        serde_json::from_str(&text)
            .map_err(|e| AppError::Parse(format!("Failed to parse {path}: {e}\nBody: {text}")))
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
            ("feature", "4".to_string()),
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
        Ok(raw.data.long_text_content)
    }

    pub fn normalize_post(&self, raw: &RawPost, uid: &str) -> WeiboPost {
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
        let text = raw.text.clone();

        WeiboPost {
            mblogid: raw.mblogid.clone(),
            created_at: raw.created_at.clone(),
            text,
            images,
            is_repost,
            repost_user,
            region,
            source_url,
            author,
        }
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
