use serde::{Deserialize, Serialize};

use super::WeiboPost;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExportContext {
    pub date_range_label: String,
}

impl Default for ExportContext {
    fn default() -> Self {
        Self {
            date_range_label: "全部时间".to_string(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CachedPosts {
    pub export_context: ExportContext,
    pub posts: Vec<WeiboPost>,
}
