use crate::app_state::AppState;
use crate::handlers::analyze::analyze;
use crate::handlers::health::health;
use crate::handlers::result::get_result;
use crate::handlers::session::{create_session, delete_session};
use crate::queue::job_queue::JobQueue;
use axum::{
    Router,
    routing::{delete, get, post},
};

pub fn create_app(queue: JobQueue) -> Router {
    let state = AppState { queue };
    Router::new()
        .route("/health", get(health))
        .route("/analyze", post(analyze))
        .route("/result/:job_id", get(get_result))
        .route("/session", post(create_session))
        .route("/session/:session_id", delete(delete_session))
        .with_state(state)
}
