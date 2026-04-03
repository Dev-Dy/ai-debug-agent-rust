use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
pub struct Job {
    pub id: String,
    pub retry: i32,
    pub logs: String,
}
