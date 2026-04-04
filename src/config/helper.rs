use std::{env, time::Duration};

use crate::errors::error::AppError;

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum RuntimeRole {
    Api,
    Worker,
    All,
}

impl RuntimeRole {
    pub fn from_args<I>(args: I) -> Result<Self, AppError>
    where
        I: IntoIterator<Item = String>,
    {
        match args.into_iter().next().as_deref() {
            None => Ok(Self::All),
            Some("api") => Ok(Self::Api),
            Some("worker") => Ok(Self::Worker),
            Some("all") => Ok(Self::All),
            Some(role) => Err(AppError::Config(format!(
                "unsupported runtime role '{role}', expected api|worker|all"
            ))),
        }
    }

    pub fn runs_workers(&self) -> bool {
        matches!(self, Self::Worker | Self::All)
    }
}

#[derive(Clone, Debug)]
pub struct Config {
    pub redis_url: String,
    pub bind_addr: String,
    pub worker_count: usize,
    pub max_concurrent_jobs: usize,
    pub max_retries: u32,
    pub job_stream: String,
    pub dlq_stream: String,
    pub consumer_group: String,
    pub read_block_ms: usize,
    pub claim_idle_ms: usize,
    pub job_status_ttl_secs: usize,
    pub session_ttl_secs: usize,
    pub session_secret: String,
    pub ai_endpoint: String,
    pub ai_model: String,
    pub ai_timeout_secs: u64,
}

impl Config {
    pub fn from_env(_role: &RuntimeRole) -> Result<Self, AppError> {
        let config = Self {
            redis_url: env_or_default("REDIS_URL", "redis://127.0.0.1:6379/"),
            bind_addr: bind_addr_from_env(),
            worker_count: parse_usize_env("WORKER_COUNT", 5)?,
            max_concurrent_jobs: parse_usize_env("MAX_CONCURRENT_JOBS", 3)?,
            max_retries: parse_u32_env("MAX_RETRIES", 3)?,
            job_stream: env_or_default("JOB_STREAM", "job_stream"),
            dlq_stream: env_or_default("DLQ_STREAM", "job_dlq_stream"),
            consumer_group: env_or_default("CONSUMER_GROUP", "job_workers"),
            read_block_ms: parse_usize_env("READ_BLOCK_MS", 5000)?,
            claim_idle_ms: parse_usize_env("CLAIM_IDLE_MS", 30000)?,
            job_status_ttl_secs: parse_usize_env("JOB_STATUS_TTL_SECS", 86400)?,
            session_ttl_secs: parse_usize_env("SESSION_TTL_SECS", 86400)?,
            session_secret: env_or_default("SESSION_SECRET", ""),
            ai_endpoint: env_or_default(
                "AI_ENDPOINT",
                "https://api.openai.com/v1/chat/completions",
            ),
            ai_model: env_or_default("AI_MODEL", "gpt-4o-mini"),
            ai_timeout_secs: parse_u64_env("AI_TIMEOUT_SECS", 30)?,
        };

        if config.session_secret.len() < 32 {
            return Err(AppError::Config(
                "SESSION_SECRET must be set and at least 32 characters long".to_string(),
            ));
        }

        if config.worker_count == 0 {
            return Err(AppError::Config("WORKER_COUNT must be > 0".to_string()));
        }

        if config.max_concurrent_jobs == 0 {
            return Err(AppError::Config(
                "MAX_CONCURRENT_JOBS must be > 0".to_string(),
            ));
        }

        Ok(config)
    }

    pub fn ai_timeout(&self) -> Duration {
        Duration::from_secs(self.ai_timeout_secs)
    }
}

fn env_or_default(name: &str, default_value: &str) -> String {
    env::var(name).unwrap_or_else(|_| default_value.to_string())
}

fn bind_addr_from_env() -> String {
    env::var("BIND_ADDR")
        .ok()
        .or_else(|| env::var("PORT").ok().map(|port| format!("0.0.0.0:{port}")))
        .unwrap_or_else(|| "0.0.0.0:3000".to_string())
}

fn parse_usize_env(name: &str, default_value: usize) -> Result<usize, AppError> {
    match env::var(name) {
        Ok(value) => value.parse::<usize>().map_err(|error| {
            AppError::Config(format!("{name} must be a positive integer: {error}"))
        }),
        Err(_) => Ok(default_value),
    }
}

fn parse_u32_env(name: &str, default_value: u32) -> Result<u32, AppError> {
    match env::var(name) {
        Ok(value) => value.parse::<u32>().map_err(|error| {
            AppError::Config(format!("{name} must be a positive integer: {error}"))
        }),
        Err(_) => Ok(default_value),
    }
}

fn parse_u64_env(name: &str, default_value: u64) -> Result<u64, AppError> {
    match env::var(name) {
        Ok(value) => value.parse::<u64>().map_err(|error| {
            AppError::Config(format!("{name} must be a positive integer: {error}"))
        }),
        Err(_) => Ok(default_value),
    }
}
