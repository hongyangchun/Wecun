use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::Mutex;
use std::time::{Duration, Instant};

use tokio::sync::Notify;

const USER_AGENT: &str = "Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/120.0.0.0 Safari/537.36";

pub struct AppState {
    pub cancel_flag: AtomicBool,
    pub current_page: AtomicUsize,
    pub total_pages: AtomicUsize,
    pub posts_fetched: AtomicUsize,
    pub total_posts: AtomicUsize,
    pub posts_exported: AtomicUsize,
    cancel_notify: Notify,
    http_client: Mutex<reqwest::Client>,
    cookie: Mutex<String>,
    last_cookie_set: Mutex<Option<Instant>>,
}

impl Default for AppState {
    fn default() -> Self {
        Self {
            cancel_flag: AtomicBool::new(false),
            current_page: AtomicUsize::new(0),
            total_pages: AtomicUsize::new(0),
            posts_fetched: AtomicUsize::new(0),
            total_posts: AtomicUsize::new(0),
            posts_exported: AtomicUsize::new(0),
            cancel_notify: Notify::new(),
            http_client: Mutex::new(Self::create_client()),
            cookie: Mutex::new(String::new()),
            last_cookie_set: Mutex::new(None),
        }
    }
}

impl AppState {
    fn create_client() -> reqwest::Client {
        reqwest::Client::builder()
            .user_agent(USER_AGENT)
            .timeout(Duration::from_secs(30))
            .pool_idle_timeout(Duration::from_secs(90))
            .pool_max_idle_per_host(6)
            .build()
            .expect("Failed to build HTTP client")
    }

    pub fn reset(&self) {
        self.cancel_flag.store(false, Ordering::Relaxed);
        self.current_page.store(0, Ordering::Relaxed);
        self.total_pages.store(0, Ordering::Relaxed);
        self.posts_fetched.store(0, Ordering::Relaxed);
        self.total_posts.store(0, Ordering::Relaxed);
        self.posts_exported.store(0, Ordering::Relaxed);
    }

    pub fn cancel(&self) {
        self.cancel_flag.store(true, Ordering::Relaxed);
        self.cancel_notify.notify_waiters();
    }

    pub fn is_cancelled(&self) -> bool {
        self.cancel_flag.load(Ordering::Relaxed)
    }

    /// Sleep for `duration`, returning `true` early if a cancellation is triggered.
    pub async fn sleep_cancellable(&self, duration: Duration) -> bool {
        if self.is_cancelled() {
            return true;
        }
        let notified = self.cancel_notify.notified();
        tokio::pin!(notified);
        if self.is_cancelled() {
            return true;
        }
        tokio::select! {
            _ = tokio::time::sleep(duration) => self.is_cancelled(),
            _ = &mut notified => true,
        }
    }

    pub fn set_cookie(&self, cookie: String) {
        let changed = {
            let mut guard = self.cookie.lock().unwrap_or_else(|e| e.into_inner());
            if *guard == cookie {
                false
            } else {
                *guard = cookie.clone();
                true
            }
        };

        if !changed {
            return;
        }

        // Note the timestamp when a new cookie is set (for grace period during account switching)
        if !cookie.is_empty() {
            *self.last_cookie_set.lock().unwrap_or_else(|e| e.into_inner()) = Some(Instant::now());
        } else {
            *self.last_cookie_set.lock().unwrap_or_else(|e| e.into_inner()) = None;
        }
        // Force refresh the HTTP client only when cookie actually changed
        let mut client_guard = self.http_client.lock().unwrap_or_else(|e| e.into_inner());
        *client_guard = Self::create_client();
    }

    pub fn get_last_cookie_set_time(&self) -> Option<Instant> {
        *self.last_cookie_set.lock().unwrap_or_else(|e| e.into_inner())
    }

    pub fn get_cookie(&self) -> String {
        self.cookie.lock().unwrap_or_else(|e| e.into_inner()).clone()
    }

    pub fn get_client(&self) -> reqwest::Client {
        self.http_client
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .clone()
    }
}
