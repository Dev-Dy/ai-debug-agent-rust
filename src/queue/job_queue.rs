use redis::Client;

#[derive(Clone)]
pub struct JobQueue {
    pub client: Client,
}