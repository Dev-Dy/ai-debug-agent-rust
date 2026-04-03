use axum::{Router, routing::post};
use crate::handlers::analyze::analyze;
use crate::queue::job_queue::JobQueue;
use crate::app_state::AppState;
use crate::handlers::result::get_result;

pub fn create_app(queue: JobQueue) -> Router {
    let state = AppState { queue };
    Router::new()
        .route("/analyze", post(analyze))
        .route("/result/:job_id", axum::routing::get(get_result))
        .with_state(state)
}