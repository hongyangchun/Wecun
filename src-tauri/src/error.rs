use thiserror::Error;

#[derive(Error, Debug)]
pub enum AppError {
    #[error("Network error: {0}")]
    Network(#[from] reqwest::Error),
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
    #[error("Parse error: {0}")]
    Parse(String),
    #[error("Download cancelled")]
    Cancelled,
    #[error("Invalid cookie")]
    InvalidCookie,
    #[error("User not found: {0}")]
    UserNotFound(String),
    #[error("API error: {0}")]
    ApiError(String),
}
