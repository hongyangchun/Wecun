use serde::{Deserialize, Serialize};

use super::request::SourceType;
use super::WeiboPost;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExportContext {
    pub date_range_label: String,
    #[serde(default)]
    pub type_label: String,
}

impl Default for ExportContext {
    fn default() -> Self {
        Self {
            date_range_label: "全部时间".to_string(),
            type_label: "微博备份".to_string(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CheckpointMeta {
    pub uid: String,
    pub source_type: Option<SourceType>,
    pub last_page: usize,
    pub total_fetched: usize,
    pub total_posts: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CachedPosts {
    pub export_context: ExportContext,
    pub posts: Vec<WeiboPost>,
    #[serde(default)]
    pub checkpoint: Option<CheckpointMeta>,
}
