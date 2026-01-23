//! Image generation client for OpenRouter API
//!
//! Uses the chat completions endpoint with `modalities: ["text", "image"]`
//! for image generation capabilities.

use std::time::{Duration, Instant};

use base64::{Engine as _, engine::general_purpose::STANDARD as BASE64};
use reqwest::Client as HttpClient;
use serde::Deserialize;
use serde_json::json;
use tracing::{debug, warn};

use crate::error::{Error, Result};

use super::types::{ImageFormat, ImageRequest, ImageResponse};

/// OpenRouter API base URL
const OPENROUTER_BASE_URL: &str = "https://openrouter.ai/api/v1";

/// Maximum retry attempts for transient failures
const MAX_RETRY_ATTEMPTS: u32 = 3;

/// Base delay for exponential backoff (milliseconds)
const BACKOFF_BASE_MS: u64 = 1000;

/// Image generation client using OpenRouter API
#[derive(Clone)]
pub struct ImageClient {
    http_client: HttpClient,
    api_key: String,
    base_url: String,
}

impl std::fmt::Debug for ImageClient {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ImageClient")
            .field("base_url", &self.base_url)
            .finish()
    }
}

/// Builder for ImageClient
pub struct ImageClientBuilder {
    api_key: Option<String>,
    base_url: Option<String>,
    timeout_secs: Option<u64>,
}

impl Default for ImageClientBuilder {
    fn default() -> Self {
        Self::new()
    }
}

impl ImageClientBuilder {
    /// Create a new builder
    pub fn new() -> Self {
        Self {
            api_key: None,
            base_url: None,
            timeout_secs: None,
        }
    }

    /// Set the API key
    pub fn api_key(mut self, key: impl Into<String>) -> Self {
        self.api_key = Some(key.into());
        self
    }

    /// Set the base URL (defaults to OpenRouter)
    pub fn base_url(mut self, url: impl Into<String>) -> Self {
        self.base_url = Some(url.into());
        self
    }

    /// Set the request timeout in seconds
    pub fn timeout_secs(mut self, secs: u64) -> Self {
        self.timeout_secs = Some(secs);
        self
    }

    /// Build the ImageClient
    pub fn build(self) -> Result<ImageClient> {
        let api_key = self
            .api_key
            .ok_or_else(|| Error::ImageApiKeyMissing)?;

        let timeout = Duration::from_secs(self.timeout_secs.unwrap_or(120));

        let http_client = HttpClient::builder()
            .timeout(timeout)
            .build()
            .map_err(Error::NetworkError)?;

        Ok(ImageClient {
            http_client,
            api_key,
            base_url: self.base_url.unwrap_or_else(|| OPENROUTER_BASE_URL.to_string()),
        })
    }
}

impl ImageClient {
    /// Create a new ImageClient with the given API key
    pub fn new(api_key: impl Into<String>) -> Result<Self> {
        ImageClientBuilder::new()
            .api_key(api_key)
            .build()
    }

    /// Create a new builder
    pub fn builder() -> ImageClientBuilder {
        ImageClientBuilder::new()
    }

    /// Generate an image from a text prompt
    ///
    /// Uses the chat completions endpoint with modalities parameter.
    pub async fn generate(&self, request: &ImageRequest) -> Result<ImageResponse> {
        let start = Instant::now();

        let prompt = request.build_prompt();
        let response = self.execute_with_retry(request.model.clone(), prompt).await?;

        let generation_time = start.elapsed().as_millis() as u64;

        Ok(ImageResponse::new(
            response.image_data,
            response.format,
            request.model.clone(),
            generation_time,
        ))
    }

    /// Generate an image with an input image (image-to-image)
    pub async fn transform(
        &self,
        model: &str,
        prompt: &str,
        input_image: &[u8],
        _strength: f32,
    ) -> Result<ImageResponse> {
        let start = Instant::now();

        // Encode input image as base64
        let image_base64 = BASE64.encode(input_image);
        let mime_type = detect_image_mime_type(input_image);

        // Build message with image content
        let content = json!([
            {
                "type": "image_url",
                "image_url": {
                    "url": format!("data:{};base64,{}", mime_type, image_base64)
                }
            },
            {
                "type": "text",
                "text": format!("Transform this image: {}", prompt)
            }
        ]);

        let response = self.execute_image_request(model, content).await?;
        let generation_time = start.elapsed().as_millis() as u64;

        Ok(ImageResponse::new(
            response.image_data,
            response.format,
            model.to_string(),
            generation_time,
        ))
    }

    /// Inpaint an image with a mask
    pub async fn inpaint(
        &self,
        model: &str,
        prompt: &str,
        input_image: &[u8],
        mask_image: &[u8],
    ) -> Result<ImageResponse> {
        let start = Instant::now();

        // Encode images as base64
        let image_base64 = BASE64.encode(input_image);
        let mask_base64 = BASE64.encode(mask_image);
        let image_mime = detect_image_mime_type(input_image);
        let mask_mime = detect_image_mime_type(mask_image);

        // Build message with both images
        let content = json!([
            {
                "type": "image_url",
                "image_url": {
                    "url": format!("data:{};base64,{}", image_mime, image_base64)
                }
            },
            {
                "type": "image_url",
                "image_url": {
                    "url": format!("data:{};base64,{}", mask_mime, mask_base64)
                }
            },
            {
                "type": "text",
                "text": format!("Inpaint the masked region with: {}. The second image is a mask where white areas should be edited.", prompt)
            }
        ]);

        let response = self.execute_image_request(model, content).await?;
        let generation_time = start.elapsed().as_millis() as u64;

        Ok(ImageResponse::new(
            response.image_data,
            response.format,
            model.to_string(),
            generation_time,
        ))
    }

    /// Execute request with retry logic
    async fn execute_with_retry(&self, model: String, prompt: String) -> Result<ParsedImageResponse> {
        let mut attempts = 0;
        let mut last_error = None;

        while attempts < MAX_RETRY_ATTEMPTS {
            attempts += 1;

            match self.send_text_to_image_request(&model, &prompt).await {
                Ok(response) => return Ok(response),
                Err(e) => {
                    if is_retryable_error(&e) {
                        let delay = calculate_backoff(attempts);
                        warn!(
                            attempt = attempts,
                            delay_ms = delay,
                            error = %e,
                            "Retrying image generation after error"
                        );
                        tokio::time::sleep(Duration::from_millis(delay)).await;
                        last_error = Some(e);
                    } else {
                        return Err(e);
                    }
                }
            }
        }

        Err(last_error.unwrap_or_else(|| Error::ImageGenerationError("Max retries exceeded".to_string())))
    }

    /// Send a text-to-image request
    async fn send_text_to_image_request(
        &self,
        model: &str,
        prompt: &str,
    ) -> Result<ParsedImageResponse> {
        let url = format!("{}/chat/completions", self.base_url);

        // Build request body with modalities for image generation
        let body = json!({
            "model": model,
            "messages": [{
                "role": "user",
                "content": prompt
            }],
            "modalities": ["text", "image"],
            "max_tokens": 4096
        });

        debug!(model = %model, "Sending image generation request");

        let response = self
            .http_client
            .post(&url)
            .bearer_auth(&self.api_key)
            .header("HTTP-Referer", "https://github.com/jasonjmcghee/demiarch")
            .header("X-Title", "Demiarch")
            .json(&body)
            .send()
            .await
            .map_err(Error::NetworkError)?;

        let status = response.status();

        if !status.is_success() {
            return self.handle_error_response(status, response).await;
        }

        let chat_response: ChatCompletionResponse = response
            .json()
            .await
            .map_err(|e| Error::ImageGenerationError(format!("Failed to parse response: {}", e)))?;

        self.extract_image_from_response(chat_response)
    }

    /// Execute request with multi-part content (for image-to-image)
    async fn execute_image_request(
        &self,
        model: &str,
        content: serde_json::Value,
    ) -> Result<ParsedImageResponse> {
        let url = format!("{}/chat/completions", self.base_url);

        let body = json!({
            "model": model,
            "messages": [{
                "role": "user",
                "content": content
            }],
            "modalities": ["text", "image"],
            "max_tokens": 4096
        });

        debug!(model = %model, "Sending image transformation request");

        let response = self
            .http_client
            .post(&url)
            .bearer_auth(&self.api_key)
            .header("HTTP-Referer", "https://github.com/jasonjmcghee/demiarch")
            .header("X-Title", "Demiarch")
            .json(&body)
            .send()
            .await
            .map_err(Error::NetworkError)?;

        let status = response.status();

        if !status.is_success() {
            return self.handle_error_response(status, response).await;
        }

        let chat_response: ChatCompletionResponse = response
            .json()
            .await
            .map_err(|e| Error::ImageGenerationError(format!("Failed to parse response: {}", e)))?;

        self.extract_image_from_response(chat_response)
    }

    /// Extract image data from chat completion response
    fn extract_image_from_response(
        &self,
        response: ChatCompletionResponse,
    ) -> Result<ParsedImageResponse> {
        let choice = response
            .choices
            .first()
            .ok_or_else(|| Error::ImageGenerationError("No response choices".to_string()))?;

        // Handle different content formats
        match &choice.message.content {
            Some(ChatMessageContent::Parts(parts)) => {
                // Try to extract from content parts
                for part in parts {
                    if part.part_type == "image_url" || part.part_type == "image" {
                        if let Some(image_url) = &part.image_url {
                            return self.parse_image_data(&image_url.url);
                        }
                        if let Some(data) = &part.data {
                            return self.parse_image_data(data);
                        }
                    }
                }
            }
            Some(ChatMessageContent::Simple(content)) => {
                // Try parsing the content as base64 directly
                // Check if it starts with data URL
                if content.starts_with("data:image") {
                    return self.parse_image_data(content);
                }
                // Try as raw base64
                if let Ok(bytes) = BASE64.decode(content.trim()) {
                    if is_valid_image_data(&bytes) {
                        let format = detect_image_format(&bytes);
                        return Ok(ParsedImageResponse {
                            image_data: bytes,
                            format,
                        });
                    }
                }
            }
            None => {}
        }

        Err(Error::ImageGenerationError(
            "No image data found in response".to_string(),
        ))
    }

    /// Parse image data from URL or base64 string
    fn parse_image_data(&self, data: &str) -> Result<ParsedImageResponse> {
        // Handle data URL format: data:image/png;base64,<base64_data>
        if data.starts_with("data:image") {
            let parts: Vec<&str> = data.splitn(2, ',').collect();
            if parts.len() == 2 {
                let header = parts[0];
                let base64_data = parts[1];

                let format = if header.contains("png") {
                    ImageFormat::Png
                } else if header.contains("jpeg") || header.contains("jpg") {
                    ImageFormat::Jpeg
                } else if header.contains("webp") {
                    ImageFormat::WebP
                } else {
                    ImageFormat::Png // Default
                };

                let image_data = BASE64
                    .decode(base64_data)
                    .map_err(|e| Error::ImageGenerationError(format!("Invalid base64: {}", e)))?;

                return Ok(ParsedImageResponse { image_data, format });
            }
        }

        // Try as raw base64
        let image_data = BASE64
            .decode(data.trim())
            .map_err(|e| Error::ImageGenerationError(format!("Invalid base64: {}", e)))?;

        let format = detect_image_format(&image_data);

        Ok(ParsedImageResponse { image_data, format })
    }

    /// Handle error responses from the API
    async fn handle_error_response<T>(
        &self,
        status: reqwest::StatusCode,
        response: reqwest::Response,
    ) -> Result<T> {
        let body = response.text().await.unwrap_or_default();

        match status.as_u16() {
            401 => Err(Error::ImageApiKeyMissing),
            400 => Err(Error::ImageGenerationError(format!("Bad request: {}", body))),
            402 => Err(Error::ImageGenerationError(
                "Payment required: Insufficient credits".to_string(),
            )),
            404 => Err(Error::ImageModelNotAvailable(format!(
                "Model not found: {}",
                body
            ))),
            429 => Err(Error::ImageGenerationError(format!(
                "Rate limited: {}",
                body
            ))),
            500..=599 => Err(Error::ImageGenerationError(format!(
                "Server error ({}): {}",
                status, body
            ))),
            _ => Err(Error::ImageGenerationError(format!(
                "HTTP error {}: {}",
                status, body
            ))),
        }
    }
}

/// Parsed image response (internal)
struct ParsedImageResponse {
    image_data: Vec<u8>,
    format: ImageFormat,
}

/// Chat completion response structure
#[derive(Debug, Deserialize)]
struct ChatCompletionResponse {
    choices: Vec<ChatChoice>,
}

#[derive(Debug, Deserialize)]
struct ChatChoice {
    message: ChatMessage,
}

/// Chat message - can be either a string content or structured content parts
#[derive(Debug, Deserialize)]
#[serde(untagged)]
enum ChatMessageContent {
    /// Simple string content
    Simple(String),
    /// Structured content with parts
    Parts(Vec<ContentPart>),
}

#[derive(Debug, Deserialize)]
struct ChatMessage {
    content: Option<ChatMessageContent>,
}

#[derive(Debug, Deserialize)]
struct ContentPart {
    #[serde(rename = "type")]
    part_type: String,
    image_url: Option<ImageUrl>,
    data: Option<String>,
}

#[derive(Debug, Deserialize)]
struct ImageUrl {
    url: String,
}

/// Detect MIME type from image bytes
fn detect_image_mime_type(data: &[u8]) -> &'static str {
    if data.starts_with(&[0x89, 0x50, 0x4E, 0x47]) {
        "image/png"
    } else if data.starts_with(&[0xFF, 0xD8, 0xFF]) {
        "image/jpeg"
    } else if data.starts_with(b"RIFF") && data.get(8..12) == Some(b"WEBP") {
        "image/webp"
    } else {
        "application/octet-stream"
    }
}

/// Detect image format from bytes
fn detect_image_format(data: &[u8]) -> ImageFormat {
    if data.starts_with(&[0x89, 0x50, 0x4E, 0x47]) {
        ImageFormat::Png
    } else if data.starts_with(&[0xFF, 0xD8, 0xFF]) {
        ImageFormat::Jpeg
    } else if data.starts_with(b"RIFF") && data.get(8..12) == Some(b"WEBP") {
        ImageFormat::WebP
    } else {
        ImageFormat::Png // Default
    }
}

/// Check if bytes represent valid image data
fn is_valid_image_data(data: &[u8]) -> bool {
    // PNG magic bytes
    if data.starts_with(&[0x89, 0x50, 0x4E, 0x47]) {
        return true;
    }
    // JPEG magic bytes
    if data.starts_with(&[0xFF, 0xD8, 0xFF]) {
        return true;
    }
    // WebP magic bytes
    if data.starts_with(b"RIFF") && data.get(8..12) == Some(b"WEBP") {
        return true;
    }
    false
}

/// Check if an error is retryable
fn is_retryable_error(error: &Error) -> bool {
    match error {
        Error::NetworkError(_) => true,
        Error::ImageGenerationError(msg) => {
            msg.contains("Rate limited") || msg.contains("Server error")
        }
        _ => false,
    }
}

/// Calculate exponential backoff delay
fn calculate_backoff(attempt: u32) -> u64 {
    let base = BACKOFF_BASE_MS * 2u64.pow(attempt - 1);
    // Add jitter (10%)
    let jitter = base / 10;
    base + (rand_jitter() % jitter.max(1))
}

/// Generate pseudo-random jitter
fn rand_jitter() -> u64 {
    use std::time::SystemTime;
    SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .map(|d| d.as_nanos() as u64 % 1000)
        .unwrap_or(0)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_builder_requires_api_key() {
        let result = ImageClientBuilder::new().build();
        assert!(result.is_err());
    }

    #[test]
    fn test_builder_with_api_key() {
        let result = ImageClientBuilder::new()
            .api_key("test-key")
            .build();
        assert!(result.is_ok());
    }

    #[test]
    fn test_detect_image_format() {
        // PNG
        let png_bytes = [0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A];
        assert!(matches!(detect_image_format(&png_bytes), ImageFormat::Png));

        // JPEG
        let jpeg_bytes = [0xFF, 0xD8, 0xFF, 0xE0];
        assert!(matches!(detect_image_format(&jpeg_bytes), ImageFormat::Jpeg));

        // WebP
        let mut webp_bytes = vec![0u8; 12];
        webp_bytes[0..4].copy_from_slice(b"RIFF");
        webp_bytes[8..12].copy_from_slice(b"WEBP");
        assert!(matches!(detect_image_format(&webp_bytes), ImageFormat::WebP));
    }

    #[test]
    fn test_is_valid_image_data() {
        let png_bytes = [0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A];
        assert!(is_valid_image_data(&png_bytes));

        let random_bytes = [0x00, 0x01, 0x02, 0x03];
        assert!(!is_valid_image_data(&random_bytes));
    }

    #[test]
    fn test_calculate_backoff() {
        let delay1 = calculate_backoff(1);
        assert!(delay1 >= BACKOFF_BASE_MS);

        let delay2 = calculate_backoff(2);
        assert!(delay2 >= BACKOFF_BASE_MS * 2);
    }
}
