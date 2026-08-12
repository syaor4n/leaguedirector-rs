use thiserror::Error;

#[derive(Debug, Error)]
pub enum ApiError {
    #[error("http: {0}")]
    Http(#[from] reqwest::Error),
    #[error("json: {0}")]
    Json(#[from] serde_json::Error),
    #[error("disconnected")]
    Disconnected,
    #[error("{0}")]
    Message(String),
}

pub type Result<T> = std::result::Result<T, ApiError>;
