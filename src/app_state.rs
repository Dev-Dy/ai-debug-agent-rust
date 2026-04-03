use crate::queue::job_queue::JobQueue;

#[derive(Clone)]
pub struct AppState {
   pub queue: JobQueue,
}