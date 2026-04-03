use crate::queue::job_queue::JobQueue;
use crate::services::ai_service::call_ai;

pub async fn worker(queue: JobQueue) {
    loop {
        let job_opt = {
            let mut q = queue.lock().await;
            q.pop_front()
        };

        if let Some(job) = job_opt {
            let result = call_ai(job).await;
            println!("Result: {}", result);
        } else {
            tokio::time::sleep(std::time::Duration::from_secs(1)).await;
        }
    }
}