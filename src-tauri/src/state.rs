use std::sync::atomic::{AtomicBool, AtomicUsize};
use std::sync::Mutex;

pub struct AppState {
    pub cancel_flag: AtomicBool,
    pub current_page: AtomicUsize,
    pub total_pages: AtomicUsize,
    pub posts_fetched: AtomicUsize,
    pub total_posts: AtomicUsize,
    pub posts_exported: AtomicUsize,
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
            cookie: Mutex::new(String::new()),
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
        self.posts_exported
            .store(0, std::sync::atomic::Ordering::Relaxed);
    }

    pub fn is_cancelled(&self) -> bool {
        self.cancel_flag.load(std::sync::atomic::Ordering::Relaxed)
    }

    pub fn set_cookie(&self, cookie: String) {
        let mut guard = self.cookie.lock().unwrap();
        *guard = cookie;
    }

    pub fn get_cookie(&self) -> String {
        self.cookie.lock().unwrap().clone()
    }
}
