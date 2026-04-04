use axum::{
    Json,
    extract::{Path, State},
};

use crate::app_state::AppState;
use crate::errors::error::AppError;
use crate::models::job::JobState;

pub async fn get_result(
    Path(job_id): Path<String>,
    State(state): State<AppState>,
) -> Result<Json<JobState>, AppError> {
    let Some(job_state) = state.queue.get_job_state(&job_id).await? else {
        return Err(AppError::NotFound(job_id));
    };

    Ok(Json(job_state))
}
