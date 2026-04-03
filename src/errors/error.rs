use axum::{
    http::StatusCode,
    response::{IntoResponse, Response},
    Json
};
use serde_json::json;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum AppError {
    #[error("Redis error: {0}")]
    Redis(#[from] redis::RedisError),   
    #[error("Parse error: {0}")]
    Parse(#[from] serde_json::Error),
    #[error("AI Error: {0}")]
    AI(String),
    #[error("Internal error")]
    InternalError,
    #[error("HTTP error: {0}")]
    Http(#[from] reqwest::Error),
}

impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        let status = match self {
            AppError::Redis(_) => StatusCode::INTERNAL_SERVER_ERROR,
            AppError::Parse(_) => StatusCode::BAD_REQUEST,
            AppError::AI(_) => StatusCode::BAD_GATEWAY,
            AppError::InternalError =>StatusCode::INTERNAL_SERVER_ERROR,
            AppError::Http(_) => StatusCode::BAD_GATEWAY,
        };

        let body = Json(json!({ "error": self.to_string() }));
        (status, body).into_response()
    }
}