use axum::{Json, extract::State};
use uuid::Uuid;

use crate::app_state::AppState;
use crate::errors::error::AppError;
use crate::models::job::{Job, JobStatus};
use crate::models::log::{LogRequest, LogResponse};

pub async fn analyze(
    State(state): State<AppState>,
    Json(payload): Json<LogRequest>,
) -> Result<Json<LogResponse>, AppError> {
    let job_id = Uuid::new_v4().to_string();
    let job = Job {
        id: job_id.clone(),
        retry: 0,
        logs: payload.logs,
    };

    state.queue.enqueue_job(&job).await?;

    Ok(Json(LogResponse {
        job_id,
        status: JobStatus::Queued,
    }))
}
