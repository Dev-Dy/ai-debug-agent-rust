use axum::{
    Json,
    extract::{Path, State},
};
use uuid::Uuid;

use crate::app_state::AppState;
use crate::errors::error::AppError;
use crate::models::session::{
    CreateSessionRequest, CreateSessionResponse, DeleteSessionResponse,
};

pub async fn create_session(
    State(state): State<AppState>,
    Json(payload): Json<CreateSessionRequest>,
) -> Result<Json<CreateSessionResponse>, AppError> {
    if payload.openai_api_key.trim().is_empty() {
        return Err(AppError::Unauthorized(
            "openai_api_key must not be empty".to_string(),
        ));
    }

    let session_id = Uuid::new_v4().to_string();
    let expires_in_secs = state
        .queue
        .store_session_key(&session_id, payload.openai_api_key.trim())
        .await?;

    Ok(Json(CreateSessionResponse {
        session_id,
        expires_in_secs,
    }))
}

pub async fn delete_session(
    Path(session_id): Path<String>,
    State(state): State<AppState>,
) -> Result<Json<DeleteSessionResponse>, AppError> {
    let deleted = state.queue.delete_session_key(&session_id).await?;

    Ok(Json(DeleteSessionResponse {
        session_id,
        deleted,
    }))
}
