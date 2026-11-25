use serde::{Deserialize, Serialize};

use super::request::{PostFilter, SourceType};
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
    #[serde(default)]
    pub filter: Option<PostFilter>,
    #[serde(default)]
    pub start_timestamp: Option<i64>,
    #[serde(default)]
    pub end_timestamp: Option<i64>,
    /// Checkpoint format version. Checkpoints written before downloads processed pages one by one
    /// recorded the last *fetched* page, so resuming them skipped unprocessed posts.
    #[serde(default)]
    pub schema: u32,
    #[serde(default)]
    pub min_text_length: usize,
    #[serde(default)]
    pub ignore_deleted: bool,
    /// Long texts that failed in earlier sessions of this download, so the final report stays honest.
    #[serde(default)]
    pub long_text_failed: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CachedPosts {
    pub export_context: ExportContext,
    pub posts: Vec<WeiboPost>,
    #[serde(default)]
    pub checkpoint: Option<CheckpointMeta>,
}
