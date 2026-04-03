use crate::models::job::Job;
use crate::queue::job_queue::JobQueue;
use crate::services::ai_service::call_ai;
use redis::AsyncCommands;
use std::sync::Arc;
use tokio::sync::Semaphore;

const MAX_RETRIES: i32 = 3;

pub async fn worker(queue: JobQueue, semaphore: Arc<Semaphore>) {
    let mut conn = queue.client.get_async_connection().await.unwrap();

    loop {
        let result: Option<(String, String)> = conn.blpop("job_queue", 0.0).await.unwrap();

        if let Some((_key, job_data)) = result {
            let permit = semaphore.acquire().await.unwrap();

            let job: Job = match serde_json::from_str(&job_data) {
                Ok(job) => job,
                Err(e) => {
                    println!("Error parsing job data: {}", e);
                    drop(permit); // 🔥 IMPORTANT
                    continue;
                }
            };

            let job_id = job.id.clone();
            let retry_count = job.retry;
            let logs = job.logs;

            println!("[WORKER] job={} retry={}", job_id, retry_count);

            let result = call_ai(logs.clone()).await;

            if result.is_empty() {
                if retry_count < MAX_RETRIES {
                    let new_job = Job {
                        id: job.id.clone(),
                        retry: retry_count + 1,
                        logs: logs.clone(),
                    };

                    let job_json = serde_json::to_string(&new_job).unwrap();

                    let _: () = conn.lpush("job_queue", job_json).await.unwrap();

                    println!("Retrying job {}", job_id);
                } else {
                    let _: () = conn.lpush("dlq", job_data).await.unwrap();

                    println!("Moved to DLQ: {}", job_id);
                }
            } else {
                let _: () = conn
                    .set_ex(format!("result:{}", job_id), result, 300)
                    .await
                    .unwrap();

                println!("Job {} completed", job_id);
            }

            drop(permit); // release slot
        }
    }
}
