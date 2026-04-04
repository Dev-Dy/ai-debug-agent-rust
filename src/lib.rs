pub mod app;
pub mod app_state;
pub mod config;
pub mod errors;
pub mod handlers;
pub mod models;
pub mod queue;
pub mod services;
pub mod workers;

use std::sync::Arc;

use axum::serve;
use config::helper::{Config, RuntimeRole};
use errors::error::AppError;
use queue::job_queue::JobQueue;
use services::ai_service::AiService;
use tokio::{net::TcpListener, sync::Semaphore, task::JoinHandle};
use tracing::info;
use uuid::Uuid;
use workers::worker::worker;

pub async fn run_from_env() -> Result<(), AppError> {
    let role = RuntimeRole::from_args(std::env::args().skip(1))?;
    let config = Config::from_env(&role)?;
    let queue = JobQueue::new(&config).await?;

    match role {
        RuntimeRole::Api => run_api(config, queue).await,
        RuntimeRole::Worker => run_worker_pool(config, queue).await,
        RuntimeRole::All => run_all(config, queue).await,
    }
}

async fn run_all(config: Config, queue: JobQueue) -> Result<(), AppError> {
    let worker_handles = spawn_workers(config.clone(), queue.clone()).await?;
    let api_result = run_api(config, queue).await;

    for handle in worker_handles {
        handle.abort();
    }

    api_result
}

async fn run_api(config: Config, queue: JobQueue) -> Result<(), AppError> {
    let listener = TcpListener::bind(&config.bind_addr).await?;
    let app = app::create_app(queue);

    info!(bind_addr = %config.bind_addr, "API server started");

    serve(listener, app)
        .with_graceful_shutdown(shutdown_signal())
        .await?;

    Ok(())
}

async fn run_worker_pool(config: Config, queue: JobQueue) -> Result<(), AppError> {
    let worker_handles = spawn_workers(config.clone(), queue).await?;

    info!(
        worker_count = config.worker_count,
        max_concurrent_jobs = config.max_concurrent_jobs,
        "Worker pool started"
    );

    shutdown_signal().await;

    for handle in worker_handles {
        handle.abort();
    }

    Ok(())
}

async fn spawn_workers(config: Config, queue: JobQueue) -> Result<Vec<JoinHandle<()>>, AppError> {
    queue.ensure_consumer_group().await?;

    let ai_service = AiService::new(&config)?;
    let semaphore = Arc::new(Semaphore::new(config.max_concurrent_jobs));
    let mut handles = Vec::with_capacity(config.worker_count);

    for worker_idx in 0..config.worker_count {
        let queue_clone = queue.clone();
        let ai_service_clone = ai_service.clone();
        let semaphore_clone = semaphore.clone();
        let consumer_name = format!(
            "worker-{}-{}-{}",
            std::process::id(),
            worker_idx,
            Uuid::new_v4()
        );
        let max_retries = config.max_retries;

        handles.push(tokio::spawn(async move {
            worker(
                queue_clone,
                ai_service_clone,
                semaphore_clone,
                consumer_name,
                max_retries,
            )
            .await;
        }));
    }

    Ok(handles)
}

async fn shutdown_signal() {
    if let Err(error) = tokio::signal::ctrl_c().await {
        tracing::error!(error = ?error, "Failed to listen for shutdown signal");
    }
}
