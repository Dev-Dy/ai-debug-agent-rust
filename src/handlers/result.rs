use axum::{
    Json,
    extract::{Path, State},
};
use tracing::error;

use crate::{app_state::AppState, errors::error::AppError};
use redis::AsyncCommands;

pub async fn get_result(
    Path(job_id): Path<String>,
    State(state): State<AppState>,
) -> Result<Json<serde_json::Value>, AppError> {
    let mut conn = match state.queue.client.get_async_connection().await {
        Ok(c) => c,
        Err(e) => {
            error!(error = ?e, "Redis connection failed");
            return Err(AppError::Redis(e));
        }
    };

    let key = format!("result:{}", job_id);
    let result: Option<String> = conn.get(&key).await?;

    Ok(match result {
        Some(res) => Json(serde_json::json!({ "status": "completed", "result": res })),
        None => Json(serde_json::json!({ "status": "processing" })),
    })
}
