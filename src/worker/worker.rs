use crate::queue::job_queue::JobQueue;
use crate::services::ai_service::call_ai;
use redis::AsyncCommands;

pub async fn worker(queue: JobQueue) {
    loop {
        let mut conn = queue.client.get_async_connection().await.unwrap();

        let job: Option<String> = conn.rpop("job_queue", None).await.unwrap();

        if let Some(job_data) = job {
            let parts: Vec<&str> = job_data.split("::").collect();
            let job_id = parts[0];
            let logs = parts[1];

            println!("Processing job {}", job_id);

            let result = call_ai(logs.to_string()).await;

            println!("Result: {}", result);
        } else {
            tokio::time::sleep(std::time::Duration::from_secs(1)).await;
        }
    }
}