use serde::{Deserialize, Serialize};

use crate::config::helper::Config;
use crate::errors::error::AppError;

#[derive(Clone)]
pub struct AiService {
    client: reqwest::Client,
    endpoint: String,
    api_key: String,
    model: String,
}

impl AiService {
    pub fn new(config: &Config) -> Result<Self, AppError> {
        let api_key = config.openai_api_key.clone().ok_or_else(|| {
            AppError::Config("OPENAI_API_KEY must be configured for workers".to_string())
        })?;

        let client = reqwest::Client::builder()
            .timeout(config.ai_timeout())
            .build()?;

        Ok(Self {
            client,
            endpoint: config.ai_endpoint.clone(),
            api_key,
            model: config.ai_model.clone(),
        })
    }

    pub async fn analyze_logs(&self, logs: &str) -> Result<String, AppError> {
        let request = ChatCompletionRequest {
            model: &self.model,
            messages: vec![
                ChatMessage {
                    role: "system",
                    content: "You are a backend debugging expert. Return a concise root-cause analysis and concrete remediation steps.",
                },
                ChatMessage {
                    role: "user",
                    content: logs,
                },
            ],
        };

        let response = self
            .client
            .post(&self.endpoint)
            .bearer_auth(&self.api_key)
            .json(&request)
            .send()
            .await?;

        let status = response.status();
        if !status.is_success() {
            let body = response.text().await.unwrap_or_else(|_| "".to_string());
            return Err(AppError::AiStatus {
                status: status.as_u16(),
                body,
            });
        }

        let body = response.json::<ChatCompletionResponse>().await?;
        parse_ai_response(body)
    }
}

#[derive(Debug, Serialize)]
struct ChatCompletionRequest<'a> {
    model: &'a str,
    messages: Vec<ChatMessage<'a>>,
}

#[derive(Debug, Serialize)]
struct ChatMessage<'a> {
    role: &'a str,
    content: &'a str,
}

#[derive(Debug, Deserialize)]
struct ChatCompletionResponse {
    choices: Vec<ChatChoice>,
}

#[derive(Debug, Deserialize)]
struct ChatChoice {
    message: ChatCompletionMessage,
}

#[derive(Debug, Deserialize)]
struct ChatCompletionMessage {
    content: String,
}

fn parse_ai_response(response: ChatCompletionResponse) -> Result<String, AppError> {
    response
        .choices
        .into_iter()
        .next()
        .map(|choice| choice.message.content.trim().to_string())
        .filter(|content| !content.is_empty())
        .ok_or_else(|| {
            AppError::AiResponse(
                "provider response did not contain choices[0].message.content".to_string(),
            )
        })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_non_empty_ai_response() {
        let parsed = parse_ai_response(ChatCompletionResponse {
            choices: vec![ChatChoice {
                message: ChatCompletionMessage {
                    content: " Investigate connection pool saturation. ".to_string(),
                },
            }],
        })
        .unwrap();

        assert_eq!(parsed, "Investigate connection pool saturation.");
    }

    #[test]
    fn rejects_empty_ai_response() {
        let error = parse_ai_response(ChatCompletionResponse { choices: vec![] }).unwrap_err();
        assert!(matches!(error, AppError::AiResponse(_)));
    }
}
