use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize)]
pub struct CreateSessionRequest {
    pub openai_api_key: String,
}

#[derive(Debug, Serialize)]
pub struct CreateSessionResponse {
    pub session_id: String,
    pub expires_in_secs: usize,
}

#[derive(Debug, Serialize)]
pub struct DeleteSessionResponse {
    pub session_id: String,
    pub deleted: bool,
}
