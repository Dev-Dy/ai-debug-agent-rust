// queue/job_queue.rs
use std::sync::Arc;
use tokio::sync::Mutex;
use std::collections::VecDeque;

pub type JobQueue = Arc<Mutex<VecDeque<String>>>;