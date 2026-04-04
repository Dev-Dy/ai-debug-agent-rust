use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, Eq, PartialEq)]
pub struct Job {
    pub id: String,
    pub session_id: String,
    pub retry: u32,
    pub logs: String,
}

impl Job {
    pub fn next_retry(&self) -> Self {
        Self {
            id: self.id.clone(),
            session_id: self.session_id.clone(),
            retry: self.retry + 1,
            logs: self.logs.clone(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Eq, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum JobStatus {
    Queued,
    Running,
    Completed,
    DeadLettered,
}

impl JobStatus {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Queued => "queued",
            Self::Running => "running",
            Self::Completed => "completed",
            Self::DeadLettered => "dead_lettered",
        }
    }

    pub fn is_terminal(&self) -> bool {
        matches!(self, Self::Completed | Self::DeadLettered)
    }
}

impl TryFrom<&str> for JobStatus {
    type Error = String;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        match value {
            "queued" => Ok(Self::Queued),
            "running" => Ok(Self::Running),
            "completed" => Ok(Self::Completed),
            "dead_lettered" => Ok(Self::DeadLettered),
            unknown => Err(format!("unknown job status '{unknown}'")),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Eq, PartialEq)]
pub struct JobState {
    pub job_id: String,
    pub status: JobStatus,
    pub retry: u32,
    pub result: Option<String>,
    pub error: Option<String>,
    pub updated_at_ms: u64,
}
