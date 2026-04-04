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
    Pdf,
    #[serde(rename = "md-single")]
    MarkdownSingle,
    #[serde(rename = "md-multi")]
    MarkdownPerPost,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DateRange {
    pub start_timestamp: Option<i64>,
    pub end_timestamp: Option<i64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DownloadRequest {
    pub uid: String,
    pub cookie: String,
    pub filter: PostFilter,
    pub include_images: bool,
    pub date_range: DateRange,
    pub export_format: ExportFormat,
    pub output_dir: String,
    pub min_text_length: usize,
}
