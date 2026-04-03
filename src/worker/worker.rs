use crate::queue::job_queue::JobQueue;
use crate::services::ai_service::call_ai;
use redis::AsyncCommands;

const MAX_RETRIES: i32 = 3;

pub async fn worker(queue: JobQueue) {
    let mut conn = queue.client.get_async_connection().await.unwrap();

    loop {
        let job: Option<String> = conn.rpop("job_queue", None).await.unwrap();

        if let Some(job_data) = job {
            let parts: Vec<&str> = job_data.splitn(3, "::").collect();

            if parts.len() != 3 {
                println!("Invalid job format: {}", job_data);
                continue;
            }

            let job_id = parts[0].to_string();
            let retry_count: i32 = parts[1].parse().unwrap_or(0);
            let logs = parts[2].to_string();

            println!("[WORKER] job={} retry={}", job_id, retry_count);

            let result = call_ai(logs).await;

            if result.is_empty() {
                if retry_count < MAX_RETRIES {
                    let new_job = format!(
                        "{}::{}::{}",
                        job_id,
                        retry_count + 1,
                        parts[2]
                    );

                    let _: () = conn.lpush("job_queue", new_job).await.unwrap();

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
        } else {
            tokio::time::sleep(std::time::Duration::from_secs(1)).await;
        }
    }
}