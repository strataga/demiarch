//! OpenRouter LLM client implementation
//!
//! Provides async HTTP client for OpenRouter API with:
//! - Chat completions (streaming and non-streaming)
//! - Cost tracking integration
//! - Model fallback with automatic retry
//! - Rate limit handling with exponential backoff

use std::sync::Arc;
use std::time::Duration;

use reqwest::Client as HttpClient;
use tracing::{debug, error, info, warn};

use crate::config::LlmConfig;
use crate::cost::{CostTracker, TokenUsage};
use crate::error::{Error, Result};

use super::streaming::{StreamEvent, parse_sse_line};
use super::types::{ChatRequest, ChatResponse, LlmResponse, Message};

/// OpenRouter API base URL
const OPENROUTER_BASE_URL: &str = "https://openrouter.ai/api/v1";

/// Maximum number of retry attempts for rate-limited requests
const MAX_RETRY_ATTEMPTS: u32 = 3;

/// Base delay for exponential backoff (in milliseconds)
const BACKOFF_BASE_MS: u64 = 1000;

/// OpenRouter LLM client
///
/// Thread-safe client for making chat completion requests to OpenRouter API.
/// Supports cost tracking, model fallback, and automatic retry on rate limits.
#[derive(Clone)]
pub struct LlmClient {
    /// HTTP client for making requests
    http_client: HttpClient,
    /// LLM configuration (model, temperature, etc.)
    config: LlmConfig,
    /// API key for authentication
    api_key: String,
    /// Base URL for the API
    base_url: String,
    /// Cost tracker for recording usage (optional)
    cost_tracker: Option<Arc<CostTracker>>,
}

impl std::fmt::Debug for LlmClient {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("LlmClient")
            .field("base_url", &self.base_url)
            .field("default_model", &self.config.default_model)
            .field("cost_tracker", &self.cost_tracker.is_some())
            .finish()
    }
}

/// Builder for creating an LlmClient
pub struct LlmClientBuilder {
    config: Option<LlmConfig>,
    api_key: Option<String>,
    base_url: Option<String>,
    cost_tracker: Option<Arc<CostTracker>>,
    timeout_secs: Option<u64>,
}

impl Default for LlmClientBuilder {
    fn default() -> Self {
        Self::new()
    }
}

impl LlmClientBuilder {
    /// Create a new builder
    pub fn new() -> Self {
        Self {
            config: None,
            api_key: None,
            base_url: None,
            cost_tracker: None,
            timeout_secs: None,
        }
    }

    /// Set the LLM configuration
    pub fn config(mut self, config: LlmConfig) -> Self {
        self.config = Some(config);
        self
    }

    /// Set the API key
    pub fn api_key(mut self, api_key: impl Into<String>) -> Self {
        self.api_key = Some(api_key.into());
        self
    }

    /// Set the base URL (defaults to OpenRouter)
    pub fn base_url(mut self, url: impl Into<String>) -> Self {
        self.base_url = Some(url.into());
        self
    }

    /// Set the cost tracker
    pub fn cost_tracker(mut self, tracker: Arc<CostTracker>) -> Self {
        self.cost_tracker = Some(tracker);
        self
    }

    /// Set the request timeout
    pub fn timeout_secs(mut self, secs: u64) -> Self {
        self.timeout_secs = Some(secs);
        self
    }

    /// Build the LlmClient
    pub fn build(self) -> Result<LlmClient> {
        let config = self.config.unwrap_or_default();
        let api_key = self
            .api_key
            .ok_or_else(|| Error::LLMError("API key is required".to_string()))?;

        let timeout_secs = self.timeout_secs.unwrap_or(config.timeout_secs);

        let http_client = HttpClient::builder()
            .timeout(Duration::from_secs(timeout_secs))
            .build()
            .map_err(Error::NetworkError)?;

        Ok(LlmClient {
            http_client,
            config,
            api_key,
            base_url: self
                .base_url
                .unwrap_or_else(|| OPENROUTER_BASE_URL.to_string()),
            cost_tracker: self.cost_tracker,
        })
    }
}

impl LlmClient {
    /// Create a new LlmClient with the given configuration and API key
    pub fn new(config: LlmConfig, api_key: impl Into<String>) -> Result<Self> {
        LlmClientBuilder::new()
            .config(config)
            .api_key(api_key)
            .build()
    }

    /// Create a new builder for LlmClient
    pub fn builder() -> LlmClientBuilder {
        LlmClientBuilder::new()
    }

    /// Set the cost tracker for recording usage
    pub fn with_cost_tracker(mut self, tracker: Arc<CostTracker>) -> Self {
        self.cost_tracker = Some(tracker);
        self
    }

    /// Get the default model from configuration
    pub fn default_model(&self) -> &str {
        &self.config.default_model
    }

    /// Get the fallback models from configuration
    pub fn fallback_models(&self) -> &[String] {
        &self.config.fallback_models
    }

    /// Make a chat completion request
    ///
    /// Sends messages to the specified model and returns the response.
    /// Records cost if a cost tracker is configured.
    pub async fn complete(
        &self,
        messages: Vec<Message>,
        model: Option<&str>,
    ) -> Result<LlmResponse> {
        let model = model.unwrap_or(&self.config.default_model);

        // Check budget before making request
        if let Some(tracker) = &self.cost_tracker
            && tracker.is_over_limit()
        {
            let today = tracker.today_total();
            let limit = tracker.daily_limit();
            return Err(Error::BudgetExceeded(today, limit, limit * 1.5));
        }

        let request = ChatRequest::new(model, messages)
            .with_temperature(self.config.temperature)
            .with_max_tokens(self.config.max_tokens);

        self.execute_request(&request).await
    }

    /// Make a chat completion request with automatic fallback
    ///
    /// Tries the default model first, then falls back to alternative models
    /// if the primary model fails with a recoverable error.
    pub async fn complete_with_fallback(&self, messages: Vec<Message>) -> Result<LlmResponse> {
        let mut models = vec![self.config.default_model.clone()];
        models.extend(self.config.fallback_models.clone());

        let mut last_error = None;

        for model in &models {
            debug!(model = %model, "Attempting chat completion");

            match self.complete(messages.clone(), Some(model)).await {
                Ok(response) => {
                    info!(model = %model, tokens = response.tokens_used, "Chat completion successful");
                    return Ok(response);
                }
                Err(Error::RateLimited(secs)) => {
                    warn!(model = %model, wait_secs = secs, "Rate limited, trying next model");
                    last_error = Some(Error::RateLimited(secs));
                }
                Err(Error::LLMError(msg)) if is_model_error(&msg) => {
                    warn!(model = %model, error = %msg, "Model error, trying next model");
                    last_error = Some(Error::LLMError(msg));
                }
                Err(e) => {
                    // Non-recoverable error, don't try other models
                    error!(model = %model, error = %e, "Non-recoverable error");
                    return Err(e);
                }
            }
        }

        Err(last_error.unwrap_or_else(|| Error::NoSuitableModel("All models failed".to_string())))
    }

    /// Make a streaming chat completion request
    ///
    /// Returns an async stream of response chunks.
    pub async fn complete_streaming(
        &self,
        messages: Vec<Message>,
        model: Option<&str>,
    ) -> Result<impl futures_core::Stream<Item = Result<StreamEvent>>> {
        let model = model.unwrap_or(&self.config.default_model).to_string();

        // Check budget before making request
        if let Some(tracker) = &self.cost_tracker
            && tracker.is_over_limit()
        {
            let today = tracker.today_total();
            let limit = tracker.daily_limit();
            return Err(Error::BudgetExceeded(today, limit, limit * 1.5));
        }

        let request = ChatRequest::new(model, messages)
            .with_temperature(self.config.temperature)
            .with_max_tokens(self.config.max_tokens)
            .with_streaming(true);

        self.execute_streaming_request(request).await
    }

    /// Execute a chat request with retry logic
    async fn execute_request(&self, request: &ChatRequest) -> Result<LlmResponse> {
        let mut attempts = 0;

        loop {
            attempts += 1;

            match self.send_request(request).await {
                Ok(response) => {
                    // Record cost
                    if let Some(tracker) = &self.cost_tracker {
                        tracker.record(
                            &response.model,
                            TokenUsage::new(response.input_tokens, response.output_tokens),
                            None,
                        );
                    }
                    return Ok(response);
                }
                Err(Error::RateLimited(wait_secs)) if attempts < MAX_RETRY_ATTEMPTS => {
                    let backoff = calculate_backoff(attempts, wait_secs);
                    warn!(
                        attempt = attempts,
                        wait_ms = backoff,
                        "Rate limited, retrying after backoff"
                    );
                    tokio::time::sleep(Duration::from_millis(backoff)).await;
                }
                Err(e) => return Err(e),
            }
        }
    }

    /// Send a single request to the API
    async fn send_request(&self, request: &ChatRequest) -> Result<LlmResponse> {
        let url = format!("{}/chat/completions", self.base_url);

        debug!(
            model = %request.model,
            messages = request.messages.len(),
            "Sending chat completion request"
        );

        let response = self
            .http_client
            .post(&url)
            .bearer_auth(&self.api_key)
            .header("HTTP-Referer", "https://github.com/jasonjmcghee/demiarch")
            .header("X-Title", "Demiarch")
            .json(request)
            .send()
            .await
            .map_err(Error::NetworkError)?;

        let status = response.status();

        if !status.is_success() {
            return self.handle_error_response(status, response).await;
        }

        let chat_response: ChatResponse = response
            .json()
            .await
            .map_err(|e| Error::LLMError(format!("Failed to parse response: {}", e)))?;

        LlmResponse::from_chat_response(chat_response)
            .ok_or_else(|| Error::LLMError("Empty response from API".to_string()))
    }

    /// Execute a streaming request
    async fn execute_streaming_request(
        &self,
        request: ChatRequest,
    ) -> Result<impl futures_core::Stream<Item = Result<StreamEvent>>> {
        let url = format!("{}/chat/completions", self.base_url);

        debug!(
            model = %request.model,
            messages = request.messages.len(),
            "Sending streaming chat completion request"
        );

        let response = self
            .http_client
            .post(&url)
            .bearer_auth(&self.api_key)
            .header("HTTP-Referer", "https://github.com/jasonjmcghee/demiarch")
            .header("X-Title", "Demiarch")
            .json(&request)
            .send()
            .await
            .map_err(Error::NetworkError)?;

        let status = response.status();

        if !status.is_success() {
            return self.handle_error_response(status, response).await;
        }

        // Return a stream that parses SSE events
        let stream = async_stream::stream! {
            let mut bytes_stream = response.bytes_stream();
            let mut buffer = String::new();

            use futures_util::StreamExt;

            while let Some(chunk_result) = bytes_stream.next().await {
                match chunk_result {
                    Ok(bytes) => {
                        buffer.push_str(&String::from_utf8_lossy(&bytes));

                        // Process complete lines
                        while let Some(newline_pos) = buffer.find('\n') {
                            let line = buffer[..newline_pos].to_string();
                            buffer = buffer[newline_pos + 1..].to_string();

                            if let Some(event) = parse_sse_line(&line) {
                                yield Ok(event);
                            }
                        }
                    }
                    Err(e) => {
                        yield Err(Error::NetworkError(e));
                        break;
                    }
                }
            }

            // Process any remaining content in buffer
            if !buffer.trim().is_empty()
                && let Some(event) = parse_sse_line(&buffer)
            {
                yield Ok(event);
            }
        };

        Ok(stream)
    }

    /// Handle error responses from the API
    async fn handle_error_response<T>(
        &self,
        status: reqwest::StatusCode,
        response: reqwest::Response,
    ) -> Result<T> {
        let body = response.text().await.unwrap_or_default();

        match status.as_u16() {
            401 => Err(Error::LLMError(
                "Unauthorized: Invalid API key. Set DEMIARCH_API_KEY or OPENROUTER_API_KEY environment variable.".to_string(),
            )),
            429 => {
                // Try to extract retry-after from headers or response
                let wait_secs = extract_retry_after(&body).unwrap_or(60);
                Err(Error::RateLimited(wait_secs))
            }
            400 => Err(Error::LLMError(format!("Bad request: {}", body))),
            402 => Err(Error::LLMError(
                "Payment required: Insufficient credits on OpenRouter account".to_string(),
            )),
            403 => Err(Error::LLMError(format!("Forbidden: {}", body))),
            404 => Err(Error::LLMError(format!(
                "Model not found or endpoint unavailable: {}",
                body
            ))),
            500..=599 => Err(Error::LLMError(format!("Server error ({}): {}", status, body))),
            _ => Err(Error::LLMError(format!(
                "HTTP error {}: {}",
                status, body
            ))),
        }
    }

    /// Count tokens in messages (approximate)
    ///
    /// This is a rough estimate based on character count.
    /// For accurate counting, use a proper tokenizer.
    pub fn estimate_tokens(&self, messages: &[Message]) -> usize {
        // Rough estimate: ~4 characters per token on average
        let total_chars: usize = messages
            .iter()
            .map(|m| m.content.len() + m.role.to_string().len() + 4) // +4 for role formatting
            .sum();

        total_chars / 4
    }

    /// Generate an embedding for a single text
    ///
    /// Uses the embeddings API to generate a vector representation of the input text.
    /// This is useful for semantic search and similarity comparisons.
    pub async fn embed(&self, text: &str, model: Option<&str>) -> Result<super::types::Embedding> {
        let model = model.unwrap_or(DEFAULT_EMBEDDING_MODEL);

        let request = super::types::EmbeddingRequest::new(model, text);
        self.execute_embedding_request(&request).await
    }

    /// Generate embeddings for multiple texts in a batch
    ///
    /// More efficient than calling embed() multiple times for related texts.
    pub async fn embed_batch(
        &self,
        texts: Vec<String>,
        model: Option<&str>,
    ) -> Result<Vec<super::types::Embedding>> {
        if texts.is_empty() {
            return Ok(Vec::new());
        }

        let model = model.unwrap_or(DEFAULT_EMBEDDING_MODEL);

        let request = super::types::EmbeddingRequest::batch(model, texts);
        self.execute_batch_embedding_request(&request).await
    }

    /// Execute an embedding request
    async fn execute_embedding_request(
        &self,
        request: &super::types::EmbeddingRequest,
    ) -> Result<super::types::Embedding> {
        let url = format!("{}/embeddings", self.base_url);

        debug!(
            model = %request.model,
            "Sending embedding request"
        );

        let response = self
            .http_client
            .post(&url)
            .bearer_auth(&self.api_key)
            .header("HTTP-Referer", "https://github.com/jasonjmcghee/demiarch")
            .header("X-Title", "Demiarch")
            .json(request)
            .send()
            .await
            .map_err(Error::NetworkError)?;

        let status = response.status();

        if !status.is_success() {
            return self.handle_error_response(status, response).await;
        }

        let embedding_response: super::types::EmbeddingResponse = response
            .json()
            .await
            .map_err(|e| Error::EmbeddingFailed(format!("Failed to parse response: {}", e)))?;

        let data = embedding_response
            .data
            .into_iter()
            .next()
            .ok_or_else(|| Error::EmbeddingFailed("Empty embedding response".to_string()))?;

        let tokens_used = embedding_response
            .usage
            .map(|u| u.prompt_tokens)
            .unwrap_or(0);

        // Record cost if tracker is configured
        if let Some(tracker) = &self.cost_tracker {
            tracker.record(
                &embedding_response.model,
                crate::cost::TokenUsage::new(tokens_used, 0),
                None,
            );
        }

        Ok(super::types::Embedding {
            vector: data.embedding,
            model: embedding_response.model,
            tokens_used,
        })
    }

    /// Execute a batch embedding request
    async fn execute_batch_embedding_request(
        &self,
        request: &super::types::EmbeddingRequest,
    ) -> Result<Vec<super::types::Embedding>> {
        let url = format!("{}/embeddings", self.base_url);

        debug!(
            model = %request.model,
            "Sending batch embedding request"
        );

        let response = self
            .http_client
            .post(&url)
            .bearer_auth(&self.api_key)
            .header("HTTP-Referer", "https://github.com/jasonjmcghee/demiarch")
            .header("X-Title", "Demiarch")
            .json(request)
            .send()
            .await
            .map_err(Error::NetworkError)?;

        let status = response.status();

        if !status.is_success() {
            return self.handle_error_response(status, response).await;
        }

        let embedding_response: super::types::EmbeddingResponse = response
            .json()
            .await
            .map_err(|e| Error::EmbeddingFailed(format!("Failed to parse response: {}", e)))?;

        let tokens_used = embedding_response
            .usage
            .map(|u| u.prompt_tokens)
            .unwrap_or(0);

        // Record cost if tracker is configured
        if let Some(tracker) = &self.cost_tracker {
            tracker.record(
                &embedding_response.model,
                crate::cost::TokenUsage::new(tokens_used, 0),
                None,
            );
        }

        // Sort by index to maintain order
        let mut data = embedding_response.data;
        data.sort_by_key(|d| d.index);

        Ok(data
            .into_iter()
            .map(|d| super::types::Embedding {
                vector: d.embedding,
                model: embedding_response.model.clone(),
                tokens_used: 0, // Individual token count not available
            })
            .collect())
    }
}

/// Default embedding model (cost-effective with good quality)
const DEFAULT_EMBEDDING_MODEL: &str = "openai/text-embedding-3-small";

/// Check if an error message indicates a model-specific error
fn is_model_error(msg: &str) -> bool {
    let model_error_patterns = [
        "model not found",
        "unavailable",
        "not available",
        "no available provider",
        "overloaded",
        "capacity",
    ];

    let msg_lower = msg.to_lowercase();
    model_error_patterns
        .iter()
        .any(|pattern| msg_lower.contains(pattern))
}

/// Calculate backoff delay with jitter
fn calculate_backoff(attempt: u32, suggested_wait: u64) -> u64 {
    let base = BACKOFF_BASE_MS * 2u64.pow(attempt - 1);
    let max_wait = suggested_wait * 1000; // Convert to ms

    // Use the larger of calculated backoff or suggested wait
    let delay = base.max(max_wait);

    // Add some jitter (10% random variation)
    let jitter = delay / 10;
    delay + (rand_jitter() % jitter.max(1))
}

/// Generate a pseudo-random jitter value
fn rand_jitter() -> u64 {
    use std::time::SystemTime;
    SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .map(|d| d.as_nanos() as u64 % 1000)
        .unwrap_or(0)
}

/// Extract retry-after value from error response
fn extract_retry_after(body: &str) -> Option<u64> {
    // Try to parse as JSON and extract retry_after field
    if let Ok(json) = serde_json::from_str::<serde_json::Value>(body) {
        if let Some(retry_after) = json.get("retry_after").and_then(|v| v.as_u64()) {
            return Some(retry_after);
        }
        if let Some(error) = json.get("error")
            && let Some(retry_after) = error.get("retry_after").and_then(|v| v.as_u64())
        {
            return Some(retry_after);
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_config() -> LlmConfig {
        LlmConfig {
            api_key: None,
            default_model: "test/model".to_string(),
            fallback_models: vec!["fallback/model".to_string()],
            temperature: 0.7,
            max_tokens: 1024,
            timeout_secs: 30,
        }
    }

    #[test]
    fn test_client_builder() {
        let client = LlmClient::builder()
            .config(test_config())
            .api_key("test-key")
            .base_url("https://example.com")
            .timeout_secs(60)
            .build()
            .unwrap();

        assert_eq!(client.default_model(), "test/model");
        assert_eq!(client.base_url, "https://example.com");
    }

    #[test]
    fn test_client_builder_requires_api_key() {
        let result = LlmClient::builder().config(test_config()).build();

        assert!(result.is_err());
    }

    #[test]
    fn test_client_new() {
        let client = LlmClient::new(test_config(), "test-key").unwrap();
        assert_eq!(client.default_model(), "test/model");
        assert_eq!(client.fallback_models(), &["fallback/model"]);
    }

    #[test]
    fn test_client_debug() {
        let client = LlmClient::new(test_config(), "test-key").unwrap();
        let debug = format!("{:?}", client);
        assert!(debug.contains("LlmClient"));
        assert!(debug.contains("test/model"));
    }

    #[test]
    fn test_client_clone() {
        let client = LlmClient::new(test_config(), "test-key").unwrap();
        let cloned = client.clone();
        assert_eq!(client.default_model(), cloned.default_model());
    }

    #[test]
    fn test_client_send_sync() {
        fn assert_send_sync<T: Send + Sync>() {}
        assert_send_sync::<LlmClient>();
    }

    #[test]
    fn test_estimate_tokens() {
        let client = LlmClient::new(test_config(), "test-key").unwrap();
        let messages = vec![
            Message::system("You are a helpful assistant"),
            Message::user("Hello, how are you?"),
        ];

        let estimate = client.estimate_tokens(&messages);
        // Should be roughly (38 + 6 + 4 + 19 + 4 + 4) / 4 ≈ 18-19 tokens
        assert!(estimate > 10);
        assert!(estimate < 30);
    }

    #[test]
    fn test_is_model_error() {
        assert!(is_model_error("Model not found"));
        assert!(is_model_error("The model is unavailable"));
        assert!(is_model_error("No available provider for this model"));
        assert!(!is_model_error("Invalid API key"));
        assert!(!is_model_error("Network timeout"));
    }

    #[test]
    fn test_calculate_backoff() {
        let backoff1 = calculate_backoff(1, 0);
        assert!(backoff1 >= BACKOFF_BASE_MS);

        let backoff2 = calculate_backoff(2, 0);
        assert!(backoff2 >= BACKOFF_BASE_MS * 2);

        // With suggested wait
        let backoff_with_wait = calculate_backoff(1, 5);
        assert!(backoff_with_wait >= 5000); // At least 5 seconds
    }

    #[test]
    fn test_extract_retry_after() {
        let body = r#"{"retry_after": 30}"#;
        assert_eq!(extract_retry_after(body), Some(30));

        let body = r#"{"error": {"retry_after": 60}}"#;
        assert_eq!(extract_retry_after(body), Some(60));

        let body = r#"{"message": "rate limited"}"#;
        assert_eq!(extract_retry_after(body), None);
    }

    #[test]
    fn test_with_cost_tracker() {
        let client = LlmClient::new(test_config(), "test-key").unwrap();
        let tracker = Arc::new(CostTracker::new(10.0, 0.8));
        let client = client.with_cost_tracker(tracker);
        assert!(client.cost_tracker.is_some());
    }
}
