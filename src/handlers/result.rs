use axum::{
    Json,
    extract::{Path, State},
};

use crate::app_state::AppState;
use redis::AsyncCommands;

pub async fn get_result(
    Path(job_id): Path<String>,
    State(state): State<AppState>,
) -> Json<serde_json::Value> {
    let mut conn = state.queue.client.get_async_connection().await.unwrap();

    let key = format!("result:{}", job_id);
    let result: Option<String> = conn.get(&key).await.unwrap();

    match result {
        Some(res) => Json(serde_json::json!({ "status": "completed", "result": res })),
        None => Json(serde_json::json!({ "status": "processing" })),
    }
}
