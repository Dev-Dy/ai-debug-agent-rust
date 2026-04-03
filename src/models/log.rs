// models/log.rs
use serde::{Deserialize, Serialize};

#[derive(Deserialize)]
pub struct LogRequest {
    pub logs: String,
}

#[derive(Serialize)]
pub struct LogResponse {
    pub analysis: String,
}
