// handlers/analyze.rs
use crate::app_state::AppState;
use crate::models::log::{LogRequest, LogResponse};
use axum::{Json, debug_handler, extract::State};

#[debug_handler]
pub async fn analyze(
    State(state): State<AppState>,
    Json(payload): Json<LogRequest>,
) -> Json<LogResponse> {
    let logs = payload.logs;
    {
        let mut q = state.queue.lock().await;
        q.push_back(logs);
    }
    Json(LogResponse {
        analysis: "Job queued".to_string(),
    })
}
