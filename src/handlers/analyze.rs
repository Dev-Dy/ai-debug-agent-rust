use axum::{
    Json,
    extract::State,
    http::HeaderMap,
};
use uuid::Uuid;

use crate::app_state::AppState;
use crate::errors::error::AppError;
use crate::models::job::{Job, JobStatus};
use crate::models::log::{LogRequest, LogResponse};

pub async fn analyze(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(payload): Json<LogRequest>,
) -> Result<Json<LogResponse>, AppError> {
    let session_id = headers
        .get("x-session-id")
        .and_then(|value| value.to_str().ok())
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .ok_or_else(|| {
            AppError::Unauthorized("missing x-session-id header".to_string())
        })?
        .to_string();

    if !state.queue.session_exists(&session_id).await? {
        return Err(AppError::Unauthorized(
            "session is invalid or expired".to_string(),
        ));
    }

    let job_id = Uuid::new_v4().to_string();
    let job = Job {
        id: job_id.clone(),
        session_id,
        retry: 0,
        logs: payload.logs,
    };

    state.queue.enqueue_job(&job).await?;

    Ok(Json(LogResponse {
        job_id,
        status: JobStatus::Queued,
    }))
}
