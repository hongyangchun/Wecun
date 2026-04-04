use std::collections::HashMap;
use std::sync::atomic::{AtomicBool, AtomicUsize};
use std::sync::{Mutex, OnceLock};

use serde::{Deserialize, Serialize};
use tauri::AppHandle;

static APP_HANDLE: OnceLock<AppHandle> = OnceLock::new();

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ApiFetchResult {
    pub request_id: String,
    pub ok: bool,
    pub status: Option<u16>,
    pub body: Option<String>,
    pub error: Option<String>,
}

pub struct AppState {
    pub cancel_flag: AtomicBool,
    pub current_page: AtomicUsize,
    pub total_pages: AtomicUsize,
    pub posts_fetched: AtomicUsize,
    pub total_posts: AtomicUsize,
    pub api_window_ready: AtomicBool,
    api_request_counter: AtomicUsize,
    api_results: Mutex<HashMap<String, ApiFetchResult>>,
}

impl Default for AppState {
    fn default() -> Self {
        Self {
            cancel_flag: AtomicBool::new(false),
            current_page: AtomicUsize::new(0),
            total_pages: AtomicUsize::new(0),
            posts_fetched: AtomicUsize::new(0),
            total_posts: AtomicUsize::new(0),
            api_window_ready: AtomicBool::new(false),
            api_request_counter: AtomicUsize::new(0),
            api_results: Mutex::new(HashMap::new()),
        }
    }
}

impl AppState {
    pub fn reset(&self) {
        self.cancel_flag
            .store(false, std::sync::atomic::Ordering::Relaxed);
        self.current_page
            .store(0, std::sync::atomic::Ordering::Relaxed);
        self.total_pages
            .store(0, std::sync::atomic::Ordering::Relaxed);
        self.posts_fetched
            .store(0, std::sync::atomic::Ordering::Relaxed);
        self.total_posts
            .store(0, std::sync::atomic::Ordering::Relaxed);
    }

    pub fn is_cancelled(&self) -> bool {
        self.cancel_flag.load(std::sync::atomic::Ordering::Relaxed)
    }

    pub fn next_api_request_id(&self) -> String {
        let id = self
            .api_request_counter
            .fetch_add(1, std::sync::atomic::Ordering::Relaxed)
            + 1;
        format!("weibo-api-{id}")
    }

    pub fn store_api_result(&self, result: ApiFetchResult) {
        let mut guard = self.api_results.lock().unwrap();
        guard.insert(result.request_id.clone(), result);
    }

    pub fn take_api_result(&self, request_id: &str) -> Option<ApiFetchResult> {
        let mut guard = self.api_results.lock().unwrap();
        guard.remove(request_id)
    }

    pub fn clear_api_result(&self, request_id: &str) {
        let mut guard = self.api_results.lock().unwrap();
        guard.remove(request_id);
    }

    pub fn clear_all_api_results(&self) {
        let mut guard = self.api_results.lock().unwrap();
        guard.clear();
    }

    pub fn is_api_window_ready(&self) -> bool {
        self.api_window_ready
            .load(std::sync::atomic::Ordering::Relaxed)
    }
}

pub fn set_app_handle(app: AppHandle) {
    let _ = APP_HANDLE.set(app);
}

pub fn app_handle() -> Option<AppHandle> {
    APP_HANDLE.get().cloned()
}
