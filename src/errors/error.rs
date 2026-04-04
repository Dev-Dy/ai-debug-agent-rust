use axum::{
    Json,
    http::StatusCode,
    response::{IntoResponse, Response},
};
use serde_json::json;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum AppError {
    #[error("Redis error: {0}")]
    Redis(#[from] redis::RedisError),
    #[error("Parse error: {0}")]
    Parse(#[from] serde_json::Error),
    #[error("HTTP error: {0}")]
    Http(#[from] reqwest::Error),
    #[error("Runtime configuration error: {0}")]
    Config(String),
    #[error("Job not found: {0}")]
    NotFound(String),
    #[error("Queue data error: {0}")]
    QueueData(String),
    #[error("AI provider returned HTTP {status}: {body}")]
    AiStatus { status: u16, body: String },
    #[error("Invalid AI response: {0}")]
    AiResponse(String),
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),
}

impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        let status = match self {
            AppError::Redis(_) => StatusCode::INTERNAL_SERVER_ERROR,
            AppError::Parse(_) => StatusCode::BAD_REQUEST,
            AppError::Http(_) => StatusCode::BAD_GATEWAY,
            AppError::Config(_) => StatusCode::INTERNAL_SERVER_ERROR,
            AppError::NotFound(_) => StatusCode::NOT_FOUND,
            AppError::QueueData(_) => StatusCode::INTERNAL_SERVER_ERROR,
            AppError::AiStatus { status, .. } => {
                if status == StatusCode::TOO_MANY_REQUESTS.as_u16() {
                    StatusCode::TOO_MANY_REQUESTS
                } else {
                    StatusCode::BAD_GATEWAY
                }
            }
            AppError::AiResponse(_) => StatusCode::BAD_GATEWAY,
            AppError::Io(_) => StatusCode::INTERNAL_SERVER_ERROR,
        };

        let body = Json(json!({ "error": self.to_string() }));
        (status, body).into_response()
    }
}

impl AppError {
    pub fn is_retryable(&self) -> bool {
        match self {
            AppError::Redis(_) | AppError::Http(_) | AppError::Io(_) => true,
            AppError::AiStatus { status, .. } => {
                *status == StatusCode::TOO_MANY_REQUESTS.as_u16()
                    || *status == StatusCode::REQUEST_TIMEOUT.as_u16()
                    || (500..=599).contains(status)
            }
            AppError::Parse(_)
            | AppError::Config(_)
            | AppError::NotFound(_)
            | AppError::QueueData(_)
            | AppError::AiResponse(_) => false,
        }
    }
}
