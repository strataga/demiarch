use std::time::Duration;
use reqwest::{Client, Response};
use serde::{Deserialize, Serialize};
use thiserror::Error;
use tokio::time::sleep;
use uuid::Uuid;

#[derive(Debug, Error)]
pub enum LlmClientError {
    #[error("Request failed: {0}")]
    RequestFailed(String),
    #[error("Rate limit exceeded")]
    RateLimitExceeded,
    #[error("Model unavailable")]
    ModelUnavailable,
    #[error("Invalid response format")]
    InvalidResponse,
}

#[derive(Debug, Serialize)]
struct OpenRouterRequest {
    model: String,
    messages: Vec<Message>,
    temperature: Option<f32>,
    max_tokens: Option<u32>,
}

#[derive(Debug, Serialize, Deserialize)]
struct Message {
    role: String,
    content: String,
}

#[derive(Debug, Deserialize)]
struct OpenRouterResponse {
    choices: Vec<Choice>,
    usage: Usage,
}

#[derive(Debug, Deserialize)]
struct Choice {
    message: Message,
}

#[derive(Debug, Deserialize)]
struct Usage {
    prompt_tokens: u32,
    completion_tokens: u32,
}

pub struct LlmClient {
    client: Client,
    api_key: String,
    base_url: String,
    retry_count: u32,
    retry_base_delay: u64,
}

impl LlmClient {
    pub fn new(api_key: String) -> Self {
        Self {
            client: Client::new(),
            api_key,
            base_url: "https://openrouter.ai/api/v1".to_string(),
            retry_count: 3,
            retry_base_delay: 1,
        }
    }

    pub async fn chat(
        &self,
        model: &str,
        messages: Vec<Message>,
        temperature: Option<f32>,
        max_tokens: Option<u32>,
    ) -> Result<Message, LlmClientError> {
        let request = OpenRouterRequest {
            model: model.to_string(),
            messages,
            temperature,
            max_tokens,
        };

        let mut attempt = 0;
        loop {
            let response = self.send_request(&request).await;

            match response {
                Ok(resp) => {
                    let response: OpenRouterResponse = resp.json().await.map_err(|_| LlmClientError::InvalidResponse)?;
                    return Ok(response.choices[0].message.clone());
                }
                Err(LlmClientError::RateLimitExceeded) => {
                    attempt += 1;
                    if attempt >= self.retry_count {
                        return Err(LlmClientError::RateLimitExceeded);
                    }
                    let delay = self.retry_base_delay * 2u64.pow(attempt);
                    sleep(Duration::from_secs(delay)).await;
                }
                Err(e) => return Err(e),
            }
        }
    }

    async fn send_request(&self, request: &OpenRouterRequest) -> Result<Response, LlmClientError> {
        let response = self.client
            .post(&format!("{}/chat/completions", self.base_url))
            .header("Authorization", format!("Bearer {}", self.api_key))
            .json(request)
            .send()
            .await
            .map_err(|e| LlmClientError::RequestFailed(e.to_string()))?;

        match response.status() {
            reqwest::StatusCode::TOO_MANY_REQUESTS => Err(LlmClientError::RateLimitExceeded),
            reqwest::StatusCode::SERVICE_UNAVAILABLE => Err(LlmClientError::ModelUnavailable),
            reqwest::StatusCode::OK => Ok(response),
            _ => Err(LlmClientError::RequestFailed(response.status().to_string())),
        }
    }
}
