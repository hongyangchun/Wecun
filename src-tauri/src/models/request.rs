use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum PostFilter {
    Original,
    All,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum ExportFormat {
    #[serde(rename = "html")]
    Html,
    #[serde(rename = "md-single")]
    MarkdownSingle,
    #[serde(rename = "md-multi")]
    MarkdownPerPost,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SourceType {
    Profile,
    Favorites,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DateRange {
    pub start_timestamp: Option<i64>,
    pub end_timestamp: Option<i64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DownloadRequest {
    pub uid: String,
    pub source_type: SourceType,
    pub cookie: String,
    pub filter: PostFilter,
    pub include_images: bool,
    pub date_range: DateRange,
    pub output_dir: String,
    pub ignore_deleted: bool,
    #[serde(default = "default_min_text_length")]
    pub min_text_length: usize,
}

fn default_min_text_length() -> usize {
    20
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExportRequest {
    pub output_dir: String,
    pub export_format: ExportFormat,
}
