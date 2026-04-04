use std::{
    collections::HashMap,
    time::{SystemTime, UNIX_EPOCH},
};

use aes_gcm::{
    Aes256Gcm, Nonce,
    aead::{Aead, AeadCore, KeyInit, OsRng},
};
use base64::{Engine as _, engine::general_purpose::STANDARD_NO_PAD};
use redis::{
    AsyncCommands, Client,
    aio::{Connection, ConnectionManager},
    streams::{
        StreamClaimReply, StreamId, StreamPendingCountReply, StreamReadOptions, StreamReadReply,
    },
};
use sha2::{Digest, Sha256};

use crate::{
    config::helper::Config,
    errors::error::AppError,
    models::job::{Job, JobState, JobStatus},
};

#[derive(Clone)]
pub struct JobQueue {
    client: Client,
    connection_manager: ConnectionManager,
    job_stream: String,
    dlq_stream: String,
    consumer_group: String,
    read_block_ms: usize,
    claim_idle_ms: usize,
    job_status_ttl_secs: usize,
    session_ttl_secs: usize,
    session_cipher: Aes256Gcm,
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct StreamJob {
    pub stream_id: String,
    pub job: Job,
    pub claimed_stale: bool,
}

impl JobQueue {
    pub async fn new(config: &Config) -> Result<Self, AppError> {
        let client = Client::open(config.redis_url.as_str())?;
        let connection_manager = ConnectionManager::new(client.clone()).await?;

        Ok(Self {
            client,
            connection_manager,
            job_stream: config.job_stream.clone(),
            dlq_stream: config.dlq_stream.clone(),
            consumer_group: config.consumer_group.clone(),
            read_block_ms: config.read_block_ms,
            claim_idle_ms: config.claim_idle_ms,
            job_status_ttl_secs: config.job_status_ttl_secs,
            session_ttl_secs: config.session_ttl_secs,
            session_cipher: build_session_cipher(&config.session_secret)?,
        })
    }

    pub async fn create_consumer_connection(&self) -> Result<Connection, AppError> {
        Ok(self.client.get_async_connection().await?)
    }

    pub async fn ensure_consumer_group(&self) -> Result<(), AppError> {
        let mut conn = self.connection_manager.clone();
        let created: Result<(), redis::RedisError> = conn
            .xgroup_create_mkstream(&self.job_stream, &self.consumer_group, "0")
            .await;

        match created {
            Ok(()) => Ok(()),
            Err(error) if error.to_string().contains("BUSYGROUP") => Ok(()),
            Err(error) => Err(AppError::Redis(error)),
        }
    }

    pub async fn store_session_key(
        &self,
        session_id: &str,
        api_key: &str,
    ) -> Result<usize, AppError> {
        let encrypted_api_key = encrypt_api_key(&self.session_cipher, api_key)?;
        let mut conn = self.connection_manager.clone();
        let session_key = self.session_key(session_id);

        let mut pipe = redis::pipe();
        pipe.atomic()
            .cmd("HSET")
            .arg(&session_key)
            .arg("session_id")
            .arg(session_id)
            .arg("encrypted_api_key")
            .arg(encrypted_api_key)
            .arg("created_at_ms")
            .arg(now_ms())
            .ignore()
            .cmd("EXPIRE")
            .arg(&session_key)
            .arg(self.session_ttl_secs)
            .ignore();

        let _: () = pipe.query_async(&mut conn).await?;
        Ok(self.session_ttl_secs)
    }

    pub async fn load_session_api_key(&self, session_id: &str) -> Result<String, AppError> {
        let mut conn = self.connection_manager.clone();
        let encrypted_api_key: Option<String> = conn
            .hget(self.session_key(session_id), "encrypted_api_key")
            .await?;

        let encrypted_api_key = encrypted_api_key.ok_or_else(|| {
            AppError::Unauthorized("session is invalid or expired".to_string())
        })?;

        decrypt_api_key(&self.session_cipher, &encrypted_api_key)
    }

    pub async fn delete_session_key(&self, session_id: &str) -> Result<bool, AppError> {
        let mut conn = self.connection_manager.clone();
        let deleted_count: usize = conn.del(self.session_key(session_id)).await?;
        Ok(deleted_count > 0)
    }

    pub async fn session_exists(&self, session_id: &str) -> Result<bool, AppError> {
        let mut conn = self.connection_manager.clone();
        let exists: bool = conn.exists(self.session_key(session_id)).await?;
        Ok(exists)
    }

    pub async fn enqueue_job(&self, job: &Job) -> Result<(), AppError> {
        let payload = serde_json::to_string(job)?;
        let mut conn = self.connection_manager.clone();
        let stream_id: String = conn
            .xadd(
                &self.job_stream,
                "*",
                &[("job_id", job.id.as_str()), ("payload", payload.as_str())],
            )
            .await?;

        self.persist_job_state(
            &job.id,
            JobStatus::Queued,
            job.retry,
            Some(stream_id.as_str()),
            None,
            None,
        )
        .await
    }

    pub async fn fetch_next_job(
        &self,
        consumer_conn: &mut Connection,
        consumer_name: &str,
    ) -> Result<Option<StreamJob>, AppError> {
        if let Some(job) = self.claim_stale_job(consumer_conn, consumer_name).await? {
            return Ok(Some(job));
        }

        let options = StreamReadOptions::default()
            .group(&self.consumer_group, consumer_name)
            .count(1)
            .block(self.read_block_ms);

        let reply: StreamReadReply = consumer_conn
            .xread_options(&[self.job_stream.as_str()], &[">"], &options)
            .await?;

        self.decode_first_job(reply, false).await
    }

    pub async fn start_job(&self, stream_job: &StreamJob) -> Result<bool, AppError> {
        if let Some(state) = self.get_job_state(&stream_job.job.id).await? {
            if state.status.is_terminal() {
                self.ack_stream_entry(&stream_job.stream_id).await?;
                return Ok(false);
            }

            if state.retry > stream_job.job.retry {
                self.ack_stream_entry(&stream_job.stream_id).await?;
                return Ok(false);
            }

            if state.status == JobStatus::Running && !stream_job.claimed_stale {
                self.ack_stream_entry(&stream_job.stream_id).await?;
                return Ok(false);
            }
        }

        self.persist_job_state(
            &stream_job.job.id,
            JobStatus::Running,
            stream_job.job.retry,
            Some(stream_job.stream_id.as_str()),
            None,
            None,
        )
        .await?;

        Ok(true)
    }

    pub async fn complete_job(&self, stream_job: &StreamJob, result: &str) -> Result<(), AppError> {
        let mut conn = self.connection_manager.clone();
        let mut pipe = redis::pipe();
        pipe.atomic()
            .cmd("HSET")
            .arg(self.job_key(&stream_job.job.id))
            .arg("job_id")
            .arg(&stream_job.job.id)
            .arg("status")
            .arg(JobStatus::Completed.as_str())
            .arg("retry")
            .arg(stream_job.job.retry)
            .arg("stream_id")
            .arg(&stream_job.stream_id)
            .arg("result")
            .arg(result)
            .arg("error")
            .arg("")
            .arg("updated_at_ms")
            .arg(now_ms())
            .ignore()
            .cmd("EXPIRE")
            .arg(self.job_key(&stream_job.job.id))
            .arg(self.job_status_ttl_secs)
            .ignore()
            .cmd("XACK")
            .arg(&self.job_stream)
            .arg(&self.consumer_group)
            .arg(&stream_job.stream_id)
            .ignore();

        let _: () = pipe.query_async(&mut conn).await?;
        Ok(())
    }

    pub async fn retry_job(&self, stream_job: &StreamJob, error: &str) -> Result<(), AppError> {
        let retry_job = stream_job.job.next_retry();
        let payload = serde_json::to_string(&retry_job)?;
        let mut conn = self.connection_manager.clone();
        let new_stream_id: String = conn
            .xadd(
                &self.job_stream,
                "*",
                &[
                    ("job_id", retry_job.id.as_str()),
                    ("payload", payload.as_str()),
                ],
            )
            .await?;

        let mut pipe = redis::pipe();
        pipe.atomic()
            .cmd("HSET")
            .arg(self.job_key(&retry_job.id))
            .arg("job_id")
            .arg(&retry_job.id)
            .arg("status")
            .arg(JobStatus::Queued.as_str())
            .arg("retry")
            .arg(retry_job.retry)
            .arg("stream_id")
            .arg(&new_stream_id)
            .arg("result")
            .arg("")
            .arg("error")
            .arg(error)
            .arg("updated_at_ms")
            .arg(now_ms())
            .ignore()
            .cmd("EXPIRE")
            .arg(self.job_key(&retry_job.id))
            .arg(self.job_status_ttl_secs)
            .ignore()
            .cmd("XACK")
            .arg(&self.job_stream)
            .arg(&self.consumer_group)
            .arg(&stream_job.stream_id)
            .ignore();

        let _: () = pipe.query_async(&mut conn).await?;
        Ok(())
    }

    pub async fn dead_letter_job(
        &self,
        stream_job: &StreamJob,
        error: &str,
    ) -> Result<(), AppError> {
        let payload = serde_json::to_string(&stream_job.job)?;
        let mut conn = self.connection_manager.clone();
        let _: String = conn
            .xadd(
                &self.dlq_stream,
                "*",
                &[
                    ("job_id", stream_job.job.id.as_str()),
                    ("payload", payload.as_str()),
                    ("error", error),
                    ("source_stream_id", stream_job.stream_id.as_str()),
                ],
            )
            .await?;

        let mut pipe = redis::pipe();
        pipe.atomic()
            .cmd("HSET")
            .arg(self.job_key(&stream_job.job.id))
            .arg("job_id")
            .arg(&stream_job.job.id)
            .arg("status")
            .arg(JobStatus::DeadLettered.as_str())
            .arg("retry")
            .arg(stream_job.job.retry)
            .arg("stream_id")
            .arg(&stream_job.stream_id)
            .arg("result")
            .arg("")
            .arg("error")
            .arg(error)
            .arg("updated_at_ms")
            .arg(now_ms())
            .ignore()
            .cmd("EXPIRE")
            .arg(self.job_key(&stream_job.job.id))
            .arg(self.job_status_ttl_secs)
            .ignore()
            .cmd("XACK")
            .arg(&self.job_stream)
            .arg(&self.consumer_group)
            .arg(&stream_job.stream_id)
            .ignore();

        let _: () = pipe.query_async(&mut conn).await?;
        Ok(())
    }

    pub async fn get_job_state(&self, job_id: &str) -> Result<Option<JobState>, AppError> {
        let mut conn = self.connection_manager.clone();
        let values: HashMap<String, String> = conn.hgetall(self.job_key(job_id)).await?;

        if values.is_empty() {
            return Ok(None);
        }

        parse_job_state(values).map(Some)
    }

    async fn claim_stale_job(
        &self,
        consumer_conn: &mut Connection,
        consumer_name: &str,
    ) -> Result<Option<StreamJob>, AppError> {
        let pending: StreamPendingCountReply = consumer_conn
            .xpending_count(&self.job_stream, &self.consumer_group, "-", "+", 10)
            .await?;

        let Some(stale_entry) = pending
            .ids
            .into_iter()
            .find(|entry| entry.last_delivered_ms >= self.claim_idle_ms)
        else {
            return Ok(None);
        };

        let claim_reply: StreamClaimReply = consumer_conn
            .xclaim(
                &self.job_stream,
                &self.consumer_group,
                consumer_name,
                self.claim_idle_ms,
                &[stale_entry.id.as_str()],
            )
            .await?;

        self.decode_first_claimed_job(claim_reply).await
    }

    async fn decode_first_job(
        &self,
        reply: StreamReadReply,
        claimed_stale: bool,
    ) -> Result<Option<StreamJob>, AppError> {
        let Some(entry) = reply
            .keys
            .into_iter()
            .next()
            .and_then(|stream_key| stream_key.ids.into_iter().next())
        else {
            return Ok(None);
        };

        match decode_stream_job(entry.clone(), claimed_stale) {
            Ok(stream_job) => Ok(Some(stream_job)),
            Err(error) => {
                self.dead_letter_corrupt_entry(&entry, &error.to_string())
                    .await?;
                Ok(None)
            }
        }
    }

    async fn decode_first_claimed_job(
        &self,
        reply: StreamClaimReply,
    ) -> Result<Option<StreamJob>, AppError> {
        let Some(entry) = reply.ids.into_iter().next() else {
            return Ok(None);
        };

        match decode_stream_job(entry.clone(), true) {
            Ok(stream_job) => Ok(Some(stream_job)),
            Err(error) => {
                self.dead_letter_corrupt_entry(&entry, &error.to_string())
                    .await?;
                Ok(None)
            }
        }
    }

    async fn dead_letter_corrupt_entry(
        &self,
        entry: &StreamId,
        error: &str,
    ) -> Result<(), AppError> {
        let mut conn = self.connection_manager.clone();
        let job_id = entry
            .get::<String>("job_id")
            .unwrap_or_else(|| "unknown".to_string());
        let payload = entry.get::<String>("payload").unwrap_or_default();

        let _: String = conn
            .xadd(
                &self.dlq_stream,
                "*",
                &[
                    ("job_id", job_id.as_str()),
                    ("payload", payload.as_str()),
                    ("error", error),
                    ("source_stream_id", entry.id.as_str()),
                ],
            )
            .await?;

        self.ack_stream_entry(&entry.id).await
    }

    async fn ack_stream_entry(&self, stream_id: &str) -> Result<(), AppError> {
        let mut conn = self.connection_manager.clone();
        let _: usize = conn
            .xack(&self.job_stream, &self.consumer_group, &[stream_id])
            .await?;
        Ok(())
    }

    async fn persist_job_state(
        &self,
        job_id: &str,
        status: JobStatus,
        retry: u32,
        stream_id: Option<&str>,
        result: Option<&str>,
        error: Option<&str>,
    ) -> Result<(), AppError> {
        let mut conn = self.connection_manager.clone();
        let mut pipe = redis::pipe();
        pipe.atomic()
            .cmd("HSET")
            .arg(self.job_key(job_id))
            .arg("job_id")
            .arg(job_id)
            .arg("status")
            .arg(status.as_str())
            .arg("retry")
            .arg(retry)
            .arg("stream_id")
            .arg(stream_id.unwrap_or(""))
            .arg("result")
            .arg(result.unwrap_or(""))
            .arg("error")
            .arg(error.unwrap_or(""))
            .arg("updated_at_ms")
            .arg(now_ms())
            .ignore()
            .cmd("EXPIRE")
            .arg(self.job_key(job_id))
            .arg(self.job_status_ttl_secs)
            .ignore();

        let _: () = pipe.query_async(&mut conn).await?;
        Ok(())
    }

    fn job_key(&self, job_id: &str) -> String {
        format!("job:{job_id}")
    }

    fn session_key(&self, session_id: &str) -> String {
        format!("session:{session_id}")
    }
}

fn build_session_cipher(session_secret: &str) -> Result<Aes256Gcm, AppError> {
    let digest = Sha256::digest(session_secret.as_bytes());
    Aes256Gcm::new_from_slice(&digest)
        .map_err(|_| AppError::Crypto("failed to initialize session cipher".to_string()))
}

fn encrypt_api_key(cipher: &Aes256Gcm, api_key: &str) -> Result<String, AppError> {
    let nonce = Aes256Gcm::generate_nonce(&mut OsRng);
    let ciphertext = cipher
        .encrypt(&nonce, api_key.as_bytes())
        .map_err(|_| AppError::Crypto("failed to encrypt session API key".to_string()))?;

    let mut encoded_payload = nonce.to_vec();
    encoded_payload.extend_from_slice(&ciphertext);
    Ok(STANDARD_NO_PAD.encode(encoded_payload))
}

fn decrypt_api_key(cipher: &Aes256Gcm, encrypted_api_key: &str) -> Result<String, AppError> {
    let decoded_payload = STANDARD_NO_PAD.decode(encrypted_api_key).map_err(|_| {
        AppError::Crypto("stored session API key is not valid base64".to_string())
    })?;

    if decoded_payload.len() <= 12 {
        return Err(AppError::Crypto(
            "stored session API key payload is malformed".to_string(),
        ));
    }

    let (nonce_bytes, ciphertext) = decoded_payload.split_at(12);
    let plaintext = cipher
        .decrypt(Nonce::from_slice(nonce_bytes), ciphertext)
        .map_err(|_| AppError::Crypto("failed to decrypt session API key".to_string()))?;

    String::from_utf8(plaintext)
        .map_err(|_| AppError::Crypto("session API key is not valid UTF-8".to_string()))
}

fn decode_stream_job(entry: StreamId, claimed_stale: bool) -> Result<StreamJob, AppError> {
    let payload = entry.get::<String>("payload").ok_or_else(|| {
        AppError::QueueData(format!("stream entry {} is missing payload", entry.id))
    })?;
    let job = serde_json::from_str::<Job>(&payload)?;

    Ok(StreamJob {
        stream_id: entry.id,
        job,
        claimed_stale,
    })
}

fn parse_job_state(values: HashMap<String, String>) -> Result<JobState, AppError> {
    let job_id = values
        .get("job_id")
        .cloned()
        .ok_or_else(|| AppError::QueueData("job state hash missing job_id".to_string()))?;
    let status = values
        .get("status")
        .ok_or_else(|| AppError::QueueData(format!("job state {job_id} missing status")))
        .and_then(|value| JobStatus::try_from(value.as_str()).map_err(AppError::QueueData))?;
    let retry = values
        .get("retry")
        .map(|value| {
            value.parse::<u32>().map_err(|error| {
                AppError::QueueData(format!("job state {job_id} has invalid retry: {error}"))
            })
        })
        .transpose()?
        .unwrap_or(0);
    let updated_at_ms = values
        .get("updated_at_ms")
        .map(|value| {
            value.parse::<u64>().map_err(|error| {
                AppError::QueueData(format!(
                    "job state {job_id} has invalid updated_at_ms: {error}"
                ))
            })
        })
        .transpose()?
        .unwrap_or(0);

    Ok(JobState {
        job_id,
        status,
        retry,
        result: non_empty_value(values.get("result")),
        error: non_empty_value(values.get("error")),
        updated_at_ms,
    })
}

fn non_empty_value(value: Option<&String>) -> Option<String> {
    value.cloned().filter(|item| !item.is_empty())
}

fn now_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as u64
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_job_state_converts_empty_result_and_error_to_none() {
        let mut values = HashMap::new();
        values.insert("job_id".to_string(), "job-1".to_string());
        values.insert("status".to_string(), "queued".to_string());
        values.insert("retry".to_string(), "2".to_string());
        values.insert("result".to_string(), "".to_string());
        values.insert("error".to_string(), "".to_string());
        values.insert("updated_at_ms".to_string(), "42".to_string());

        let parsed = parse_job_state(values).unwrap();

        assert_eq!(parsed.job_id, "job-1");
        assert_eq!(parsed.status, JobStatus::Queued);
        assert_eq!(parsed.retry, 2);
        assert_eq!(parsed.result, None);
        assert_eq!(parsed.error, None);
        assert_eq!(parsed.updated_at_ms, 42);
    }

    #[test]
    fn decode_stream_job_rejects_missing_payload() {
        let stream_id = StreamId {
            id: "1-0".to_string(),
            map: HashMap::new(),
        };

        let error = decode_stream_job(stream_id, false).unwrap_err();
        assert!(matches!(error, AppError::QueueData(_)));
    }

    #[test]
    fn encrypt_and_decrypt_api_key_round_trip() {
        let cipher = build_session_cipher("0123456789abcdef0123456789abcdef").unwrap();
        let encrypted = encrypt_api_key(&cipher, "sk-test-123").unwrap();

        assert_ne!(encrypted, "sk-test-123");
        assert_eq!(decrypt_api_key(&cipher, &encrypted).unwrap(), "sk-test-123");
    }
}
