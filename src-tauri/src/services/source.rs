use std::future::Future;

use crate::error::AppError;
use crate::models::{ProgressPhase, RawPost, UserProfile};

/// The Weibo data a download needs. The real client talks to Weibo; tests script a fake.
pub trait WeiboSource: Send + Sync {
    fn get_user_info(&self, uid: &str) -> impl Future<Output = Result<UserProfile, AppError>> + Send;

    /// One page of a profile timeline. The second item is the post count Weibo reports, or <= 0 when it reports none.
    fn fetch_posts_page(&self, uid: &str, page: i64) -> impl Future<Output = Result<(Vec<RawPost>, i64), AppError>> + Send;

    /// One page of the signed-in account's favorites.
    fn fetch_favorites_page(
        &self,
        page: i64,
        starttime: Option<i64>,
        endtime: Option<i64>,
    ) -> impl Future<Output = Result<(Vec<RawPost>, i64), AppError>> + Send;

    /// The full text of a long post.
    fn get_long_text(&self, mblogid: &str) -> impl Future<Output = Result<String, AppError>> + Send;
}

/// Where a download reports progress. The app window listens; tests collect.
pub trait ProgressSink: Send + Sync {
    fn emit(&self, phase: ProgressPhase, current: usize, total: usize, message: &str);
}
