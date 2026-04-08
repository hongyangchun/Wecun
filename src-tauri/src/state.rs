use std::sync::atomic::{AtomicBool, AtomicUsize};
use std::sync::Mutex;
use std::time::Duration;

const USER_AGENT: &str = "Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/120.0.0.0 Safari/537.36";

pub struct AppState {
    pub cancel_flag: AtomicBool,
    pub current_page: AtomicUsize,
    pub total_pages: AtomicUsize,
    pub posts_fetched: AtomicUsize,
    pub total_posts: AtomicUsize,
    pub posts_exported: AtomicUsize,
    http_client: Mutex<reqwest::Client>,
    cookie: Mutex<String>,
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
            http_client: Mutex::new(Self::create_client()),
            cookie: Mutex::new(String::new()),
        }
    }
}

impl AppState {
    fn create_client() -> reqwest::Client {
        reqwest::Client::builder()
            .user_agent(USER_AGENT)
            .timeout(Duration::from_secs(30))
            .pool_idle_timeout(Duration::from_secs(90))
            .pool_max_idle_per_host(4)
            .build()
            .expect("Failed to build HTTP client")
    }

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
        self.posts_exported
            .store(0, std::sync::atomic::Ordering::Relaxed);
    }

    pub fn is_cancelled(&self) -> bool {
        self.cancel_flag.load(std::sync::atomic::Ordering::Relaxed)
    }

    pub fn set_cookie(&self, cookie: String) {
        {
            let mut guard = self.cookie.lock().unwrap();
            *guard = cookie;
        }
        // Force refresh the HTTP client to clear internal connection pool state/cookies
        let mut client_guard = self.http_client.lock().unwrap();
        *client_guard = Self::create_client();
    }

    pub fn get_cookie(&self) -> String {
        self.cookie.lock().unwrap().clone()
    }

    pub fn get_client(&self) -> reqwest::Client {
        self.http_client.lock().unwrap().clone()
    }
}
