mod app;
mod app_state;
mod handlers;
mod models;
mod queue;
mod services;
mod worker;

use queue::job_queue::JobQueue;
use std::collections::VecDeque;
use std::sync::Arc;
use tokio::net::TcpListener;
use tokio::sync::Mutex;
use worker::worker::worker;

#[tokio::main]
async fn main() {
    let queue: JobQueue = Arc::new(Mutex::new(VecDeque::new()));

    let app = app::create_app(queue.clone());

    tokio::spawn(worker(queue.clone()));

    let listener = TcpListener::bind("127.0.0.1:3000").await.unwrap();

    axum::serve(listener, app).await.unwrap();
}
