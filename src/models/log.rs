use serde::{Deserialize, Serialize};

use crate::models::job::JobStatus;

#[derive(Deserialize)]
pub struct LogRequest {
    pub logs: String,
}

#[derive(Serialize)]
pub struct LogResponse {
    pub job_id: String,
    pub status: JobStatus,
}
