use crate::app_state::AppState;
use crate::handlers::analyze::analyze;
use crate::handlers::result::get_result;
use crate::queue::job_queue::JobQueue;
use axum::{
    Router,
    routing::{get, post},
};

pub fn create_app(queue: JobQueue) -> Router {
    let state = AppState { queue };
    Router::new()
        .route("/analyze", post(analyze))
        .route("/result/:job_id", get(get_result))
        .with_state(state)
}
