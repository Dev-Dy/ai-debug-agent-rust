// handlers/analyze.rs
use crate::app_state::AppState;
use crate::models::job::Job;
use crate::models::log::{LogRequest, LogResponse};
use axum::{Json, debug_handler, extract::State};
use redis::AsyncCommands;
use uuid::Uuid;

#[debug_handler]
pub async fn analyze(
    State(state): State<AppState>,
    Json(payload): Json<LogRequest>,
) -> Json<LogResponse> {
    let job_id = Uuid::new_v4().to_string();
    let mut conn = state.queue.client.get_async_connection().await.unwrap();
    let job = Job {
        id: job_id.clone(),
        retry: 0,
        logs: payload.logs,
    };
    let job_data = serde_json::to_string(&job).unwrap();
    let _: () = conn.lpush("job_queue", job_data).await.unwrap();

    Json(LogResponse {
        analysis: "Job queued".to_string(),
    })
}
