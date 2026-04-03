use redis::AsyncCommands;
use std::sync::Arc;
use tokio::sync::Semaphore;
use tracing::{error, info, warn};

use crate::models::job::Job;
use crate::queue::job_queue::JobQueue;
use crate::services::ai_service::call_ai;

const MAX_RETRIES: i32 = 3;

pub async fn worker(queue: JobQueue, semaphore: Arc<Semaphore>) {
    loop {
        let mut conn = match queue.client.get_async_connection().await {
            Ok(c) => c,
            Err(e) => {
                error!(error = ?e, "Redis connection failed");
                continue;
            }
        };

        let (_key, job_data): (String, String) = match conn.blpop("job_queue", 0.0).await {
            Ok(Some(data)) => data,
            Ok(None) => continue,
            Err(e) => {
                error!(error = ?e, "BLPOP failed");
                continue;
            }
        };

        let permit = match semaphore.acquire().await {
            Ok(p) => p,
            Err(e) => {
                error!(error = ?e, "Semaphore acquire failed");
                continue;
            }
        };

        let job: Job = match serde_json::from_str(&job_data) {
            Ok(job) => job,
            Err(e) => {
                error!(error = ?e, "Invalid job JSON");
                drop(permit);
                continue;
            }
        };

        let job_id = job.id.clone();
        let retry_count = job.retry;
        let logs = job.logs.clone();

        info!(job_id = %job_id, retry = retry_count, "Processing job");

        let result = match call_ai(logs).await {
            Ok(res) => res,
            Err(e) => {
                error!(error = ?e, job_id = %job_id, "AI call failed");

                if retry_count < MAX_RETRIES {
                    warn!(job_id = %job_id, retry = retry_count, "Retrying job");

                    let new_job = Job {
                        id: job_id.clone(),
                        retry: retry_count + 1,
                        logs: job.logs.clone(),
                    };

                    let job_json = match serde_json::to_string(&new_job) {
                        Ok(j) => j,
                        Err(e) => {
                            error!(error = ?e, "Failed to serialize job");
                            drop(permit);
                            continue;
                        }
                    };

                    if let Err(e) = conn.lpush::<_,_, ()>("job_queue", job_json).await {
                        error!(error = ?e, "Failed to requeue job");
                    }
                } else {
                    error!(job_id = %job_id, "Moved to DLQ");

                    if let Err(e) = conn.lpush::<_, _, ()>("dlq", job_data).await {
                        error!(error = ?e, "Failed to push DLQ");
                    }
                }

                drop(permit);
                continue;
            }
        };

        // ✅ Store result
        if let Err(e) = conn.set_ex::<_, _, ()>(format!("result:{}", job_id), result, 300).await {
            error!(error = ?e, "Failed to store result");
        }

        info!(job_id = %job_id, "Job completed");

        drop(permit);
    }
}
