mod app;
mod app_state;
mod handlers;
mod models;
mod queue;
mod services;
mod worker;

use queue::job_queue::JobQueue;
use redis::Client;
use tokio::net::TcpListener;
use worker::worker::worker;

#[tokio::main]
async fn main() {
    let redis_client = Client::open("redis://127.0.0.1/").unwrap();
    let queue = JobQueue {
        client: redis_client,
    };
    let app = app::create_app(queue.clone());

    tokio::spawn(worker(queue.clone()));

    let listener = TcpListener::bind("127.0.0.1:3000").await.unwrap();

    axum::serve(listener, app).await.unwrap();
}
