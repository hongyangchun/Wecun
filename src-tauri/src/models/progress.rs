use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ProgressPhase {
    FetchingUserInfo,
    FetchingPostList,
    FetchingLongText,
    DownloadingImages,
    Exporting,
    Complete,
    Error,
    Cancelled,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProgressEvent {
    pub phase: ProgressPhase,
    pub current: usize,
    pub total: usize,
    pub message: String,
}

impl ProgressEvent {
    pub fn new(phase: ProgressPhase, current: usize, total: usize, message: &str) -> Self {
        Self {
            phase,
            current,
            total,
            message: message.to_string(),
        }
    }
}
