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
    /// Posts the walk started from, when it is adding to an earlier backup.
    #[serde(default)]
    pub base_posts: usize,
    /// Posts this run saw and left out because of the rules. An update treats them as handled.
    #[serde(default)]
    pub skipped_ids: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CachedPosts {
    pub export_context: ExportContext,
    pub posts: Vec<WeiboPost>,
    #[serde(default)]
    pub checkpoint: Option<CheckpointMeta>,
    /// Set once the download is finished: what the backup holds and the rules it was made with.
    #[serde(default)]
    pub backup: Option<BackupMeta>,
    /// Posts a finished download saw and left out because of the rules, so that an update knows it
    /// has been through them. They do not go into `BackupMeta`: that is read while the user types.
    #[serde(default)]
    pub skipped_ids: Vec<String>,
}

/// What a finished backup holds and the rules it was made with. An update may only add to a backup
/// made with the same content rules, so they travel with the posts.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct BackupMeta {
    pub uid: String,
    pub source_type: SourceType,
    pub post_count: usize,
    pub author: String,
    pub filter: PostFilter,
    pub min_text_length: usize,
    pub ignore_deleted: bool,
    pub start_timestamp: Option<i64>,
    pub end_timestamp: Option<i64>,
}
