// handlers/analyze.rs
use crate::app_state::AppState;
use crate::models::log::{LogRequest, LogResponse};
use axum::{Json, debug_handler, extract::State};
use uuid::Uuid;
use redis::AsyncCommands;

#[debug_handler]
pub async fn analyze(
    State(state): State<AppState>,
    Json(payload): Json<LogRequest>,
) -> Json<LogResponse> {
    let job_id = Uuid::new_v4().to_string();
    let mut conn = state.queue.client.get_async_connection().await.unwrap();

    let job_data = format!("{}:{}", job_id, payload.logs);
    let _: () = conn.rpush("job_queue", job_data).await.unwrap();
    
    Json(LogResponse {
        analysis: "Job queued".to_string(),
    })
}
