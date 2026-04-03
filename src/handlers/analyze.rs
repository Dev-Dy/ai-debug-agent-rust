use axum::{Json, extract::State};
use redis::AsyncCommands;
use uuid::Uuid;

use crate::app_state::AppState;
use crate::errors::error::AppError;
use crate::models::job::Job;
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
    let job_data = serde_json::to_string(&job)?;

    let mut conn = state
        .queue
        .client
        .get_async_connection()
        .await?;

    let _: () = conn.lpush("job_queue", job_data)
        .await?;

    Ok(Json(LogResponse {
        analysis: "Job queued".to_string(),
    }))
}
