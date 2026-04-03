mod app;
mod app_state;
mod handlers;
mod models;
mod queue;
mod services;
mod workers;

use queue::job_queue::JobQueue;
use redis::Client;
use tokio::net::TcpListener;
use workers::worker::worker;

use std::sync::Arc;
use tokio::sync::Semaphore;

#[tokio::main]
async fn main() {
    let redis_client = Client::open("redis://127.0.0.1/").unwrap();

    let queue = JobQueue {
        client: redis_client,
    };

    let app = app::create_app(queue.clone());

    let num_workers = 5;
    let max_concurrent_jobs = 3;

    let semaphore = Arc::new(Semaphore::new(max_concurrent_jobs));

    for _ in 0..num_workers {
        let queue_clone = queue.clone();
        let semaphore_clone = semaphore.clone();

        tokio::spawn(async move {
            worker(queue_clone, semaphore_clone).await;
        });
    }

    let listener = TcpListener::bind("127.0.0.1:3000").await.unwrap();

    println!("Server running on 127.0.0.1:3000");

    axum::serve(listener, app).await.unwrap();
}