use std::sync::Arc;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use tokio::sync::Semaphore;
use tracing::{error, info, warn};

use crate::errors::error::AppError;
use crate::queue::job_queue::{JobQueue, StreamJob};
use crate::services::ai_service::AiService;

pub async fn worker(
    queue: JobQueue,
    ai_service: AiService,
    semaphore: Arc<Semaphore>,
    consumer_name: String,
    max_retries: u32,
) {
    let mut consumer_conn = loop {
        match queue.create_consumer_connection().await {
            Ok(conn) => break conn,
            Err(error) => {
                error!(error = ?error, consumer = %consumer_name, "Failed to create Redis consumer connection");
                tokio::time::sleep(Duration::from_millis(500)).await;
            }
        }
    };

    loop {
        let permit = match semaphore.clone().acquire_owned().await {
            Ok(permit) => permit,
            Err(error) => {
                error!(error = ?error, consumer = %consumer_name, "Semaphore acquire failed");
                return;
            }
        };

        let maybe_stream_job = match queue
            .fetch_next_job(&mut consumer_conn, &consumer_name)
            .await
        {
            Ok(stream_job) => stream_job,
            Err(error) => {
                error!(error = ?error, consumer = %consumer_name, "Failed to fetch job from stream");
                drop(permit);

                match queue.create_consumer_connection().await {
                    Ok(conn) => consumer_conn = conn,
                    Err(reconnect_error) => {
                        error!(error = ?reconnect_error, consumer = %consumer_name, "Redis consumer reconnect failed");
                        tokio::time::sleep(Duration::from_millis(500)).await;
                    }
                }

                continue;
            }
        };

        let Some(stream_job) = maybe_stream_job else {
            drop(permit);
            continue;
        };

        if let Err(error) = process_stream_job(&queue, &ai_service, &stream_job, max_retries).await
        {
            error!(
                error = ?error,
                job_id = %stream_job.job.id,
                stream_id = %stream_job.stream_id,
                consumer = %consumer_name,
                "Job processing failed"
            );
        }

        drop(permit);
    }
}

async fn process_stream_job(
    queue: &JobQueue,
    ai_service: &AiService,
    stream_job: &StreamJob,
    max_retries: u32,
) -> Result<(), AppError> {
    if !queue.start_job(stream_job).await? {
        info!(
            job_id = %stream_job.job.id,
            stream_id = %stream_job.stream_id,
            "Skipping stale or duplicate stream entry"
        );
        return Ok(());
    }

    info!(
        job_id = %stream_job.job.id,
        session_id = %stream_job.job.session_id,
        retry = stream_job.job.retry,
        stream_id = %stream_job.stream_id,
        claimed_stale = stream_job.claimed_stale,
        "Processing job"
    );

    let session_api_key = match queue
        .load_session_api_key(&stream_job.job.session_id)
        .await
    {
        Ok(api_key) => api_key,
        Err(error) => {
            handle_processing_error(queue, stream_job, max_retries, error).await?;
            return Ok(());
        }
    };

    match ai_service
        .analyze_logs(&stream_job.job.logs, &session_api_key)
        .await
    {
        Ok(result) => {
            queue.complete_job(stream_job, &result).await?;
            info!(
                job_id = %stream_job.job.id,
                stream_id = %stream_job.stream_id,
                "Job completed"
            );
        }
        Err(error) => {
            handle_processing_error(queue, stream_job, max_retries, error).await?;
        }
    }

    Ok(())
}

async fn handle_processing_error(
    queue: &JobQueue,
    stream_job: &StreamJob,
    max_retries: u32,
    error: AppError,
) -> Result<(), AppError> {
    let should_retry = error.is_retryable() && stream_job.job.retry < max_retries;

    if should_retry {
        let delay = retry_backoff(stream_job.job.retry);
        warn!(
            job_id = %stream_job.job.id,
            stream_id = %stream_job.stream_id,
            retry = stream_job.job.retry,
            delay_ms = delay.as_millis(),
            error = ?error,
            "Retrying job"
        );
        tokio::time::sleep(delay).await;
        queue.retry_job(stream_job, &error.to_string()).await?;
    } else {
        error!(
            job_id = %stream_job.job.id,
            stream_id = %stream_job.stream_id,
            retry = stream_job.job.retry,
            error = ?error,
            "Moving job to DLQ"
        );
        queue
            .dead_letter_job(stream_job, &error.to_string())
            .await?;
    }

    Ok(())
}

fn retry_backoff(retry: u32) -> Duration {
    let base_ms = 250_u64.saturating_mul(1_u64 << retry.min(6));
    let jitter_ms = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as u64
        % 250;

    Duration::from_millis(base_ms + jitter_ms)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn retry_backoff_increases_with_retry_count() {
        let first = retry_backoff(0);
        let second = retry_backoff(1);

        assert!(second >= first);
        assert!(first >= Duration::from_millis(250));
    }
}
