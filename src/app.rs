use axum::{Router, routing::post};
use crate::handlers::analyze::analyze;
use crate::queue::job_queue::JobQueue;
use crate::app_state::AppState;

pub fn create_app(queue: JobQueue) -> Router {
    let state = AppState { queue };
    Router::new()
        .route("/analyze", post(analyze))
        .with_state(state)
}