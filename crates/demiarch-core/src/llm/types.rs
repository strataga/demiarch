//! LLM types for OpenRouter API
//!
//! These types match the OpenAI-compatible API format used by OpenRouter.

use serde::{Deserialize, Serialize};

/// Role of a message in a conversation
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum MessageRole {
    /// System message (instructions/context)
    System,
    /// User message (human input)
    User,
    /// Assistant message (LLM response)
    Assistant,
}

impl std::fmt::Display for MessageRole {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            MessageRole::System => write!(f, "system"),
            MessageRole::User => write!(f, "user"),
            MessageRole::Assistant => write!(f, "assistant"),
        }
    }
}

/// A message in a conversation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Message {
    /// Role of the message sender
    pub role: MessageRole,
    /// Content of the message
    pub content: String,
}

impl Message {
    /// Create a new message
    pub fn new(role: MessageRole, content: impl Into<String>) -> Self {
        Self {
            role,
            content: content.into(),
        }
    }

    /// Create a system message
    pub fn system(content: impl Into<String>) -> Self {
        Self::new(MessageRole::System, content)
    }

    /// Create a user message
    pub fn user(content: impl Into<String>) -> Self {
        Self::new(MessageRole::User, content)
    }

    /// Create an assistant message
    pub fn assistant(content: impl Into<String>) -> Self {
        Self::new(MessageRole::Assistant, content)
    }
}

/// Request body for chat completions
#[derive(Debug, Clone, Serialize)]
pub struct ChatRequest {
    /// Model identifier (e.g., "anthropic/claude-sonnet-4-20250514")
    pub model: String,
    /// List of messages in the conversation
    pub messages: Vec<Message>,
    /// Sampling temperature (0.0 to 2.0)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub temperature: Option<f32>,
    /// Maximum tokens to generate
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_tokens: Option<usize>,
    /// Enable streaming responses
    #[serde(skip_serializing_if = "Option::is_none")]
    pub stream: Option<bool>,
}

impl ChatRequest {
    /// Create a new chat request with required fields
    pub fn new(model: impl Into<String>, messages: Vec<Message>) -> Self {
        Self {
            model: model.into(),
            messages,
            temperature: None,
            max_tokens: None,
            stream: None,
        }
    }

    /// Set the temperature
    pub fn with_temperature(mut self, temperature: f32) -> Self {
        self.temperature = Some(temperature);
        self
    }

    /// Set max tokens
    pub fn with_max_tokens(mut self, max_tokens: usize) -> Self {
        self.max_tokens = Some(max_tokens);
        self
    }

    /// Enable streaming
    pub fn with_streaming(mut self, stream: bool) -> Self {
        self.stream = Some(stream);
        self
    }
}

/// Token usage information from the API response
#[derive(Debug, Clone, Deserialize)]
pub struct Usage {
    /// Number of tokens in the prompt
    pub prompt_tokens: u32,
    /// Number of tokens in the completion
    pub completion_tokens: u32,
    /// Total tokens used
    #[serde(default)]
    pub total_tokens: u32,
}

/// Reason for completion finishing
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FinishReason {
    /// Natural stop (end of response)
    Stop,
    /// Max tokens reached
    Length,
    /// Tool/function calls requested
    ToolCalls,
    /// Content filtered by safety system
    ContentFilter,
    /// Error occurred
    Error,
    /// Unknown reason (catch-all)
    #[serde(other)]
    Unknown,
}

impl std::fmt::Display for FinishReason {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            FinishReason::Stop => write!(f, "stop"),
            FinishReason::Length => write!(f, "length"),
            FinishReason::ToolCalls => write!(f, "tool_calls"),
            FinishReason::ContentFilter => write!(f, "content_filter"),
            FinishReason::Error => write!(f, "error"),
            FinishReason::Unknown => write!(f, "unknown"),
        }
    }
}

/// A single completion choice from the API response
#[derive(Debug, Clone, Deserialize)]
pub struct Choice {
    /// Index of this choice
    pub index: usize,
    /// The generated message
    pub message: Message,
    /// Reason the generation stopped
    pub finish_reason: Option<FinishReason>,
}

/// Response from the chat completions API
#[derive(Debug, Clone, Deserialize)]
pub struct ChatResponse {
    /// Unique identifier for this completion
    pub id: String,
    /// Object type (always "chat.completion")
    pub object: String,
    /// Unix timestamp of when the completion was created
    pub created: u64,
    /// Model used for the completion
    pub model: String,
    /// List of completion choices
    pub choices: Vec<Choice>,
    /// Token usage information
    pub usage: Option<Usage>,
}

/// Simplified response returned by the LLM client
#[derive(Debug, Clone)]
pub struct LlmResponse {
    /// The generated content
    pub content: String,
    /// Model that generated the response
    pub model: String,
    /// Total tokens used (input + output)
    pub tokens_used: u32,
    /// Input tokens
    pub input_tokens: u32,
    /// Output tokens
    pub output_tokens: u32,
    /// Reason for stopping
    pub finish_reason: FinishReason,
}

impl LlmResponse {
    /// Create a new LLM response from API response
    pub fn from_chat_response(response: ChatResponse) -> Option<Self> {
        let choice = response.choices.first()?;
        let usage = response.usage.as_ref();

        Some(Self {
            content: choice.message.content.clone(),
            model: response.model,
            tokens_used: usage.map(|u| u.total_tokens).unwrap_or(0),
            input_tokens: usage.map(|u| u.prompt_tokens).unwrap_or(0),
            output_tokens: usage.map(|u| u.completion_tokens).unwrap_or(0),
            finish_reason: choice
                .finish_reason
                .clone()
                .unwrap_or(FinishReason::Unknown),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_message_creation() {
        let system = Message::system("You are a helpful assistant");
        assert_eq!(system.role, MessageRole::System);
        assert_eq!(system.content, "You are a helpful assistant");

        let user = Message::user("Hello!");
        assert_eq!(user.role, MessageRole::User);

        let assistant = Message::assistant("Hi there!");
        assert_eq!(assistant.role, MessageRole::Assistant);
    }

    #[test]
    fn test_chat_request_builder() {
        let request = ChatRequest::new("anthropic/claude-sonnet-4-20250514", vec![])
            .with_temperature(0.7)
            .with_max_tokens(1024)
            .with_streaming(false);

        assert_eq!(request.model, "anthropic/claude-sonnet-4-20250514");
        assert_eq!(request.temperature, Some(0.7));
        assert_eq!(request.max_tokens, Some(1024));
        assert_eq!(request.stream, Some(false));
    }

    #[test]
    fn test_message_role_display() {
        assert_eq!(MessageRole::System.to_string(), "system");
        assert_eq!(MessageRole::User.to_string(), "user");
        assert_eq!(MessageRole::Assistant.to_string(), "assistant");
    }

    #[test]
    fn test_finish_reason_display() {
        assert_eq!(FinishReason::Stop.to_string(), "stop");
        assert_eq!(FinishReason::Length.to_string(), "length");
        assert_eq!(FinishReason::ToolCalls.to_string(), "tool_calls");
    }

    #[test]
    fn test_chat_request_serialization() {
        let request =
            ChatRequest::new("test-model", vec![Message::user("Hello")]).with_temperature(0.5);

        let json = serde_json::to_string(&request).unwrap();
        assert!(json.contains("\"model\":\"test-model\""));
        assert!(json.contains("\"temperature\":0.5"));
        assert!(json.contains("\"role\":\"user\""));
    }

    #[test]
    fn test_chat_response_deserialization() {
        let json = r#"{
            "id": "gen-123",
            "object": "chat.completion",
            "created": 1234567890,
            "model": "anthropic/claude-sonnet-4-20250514",
            "choices": [{
                "index": 0,
                "message": {
                    "role": "assistant",
                    "content": "Hello! How can I help you?"
                },
                "finish_reason": "stop"
            }],
            "usage": {
                "prompt_tokens": 10,
                "completion_tokens": 8,
                "total_tokens": 18
            }
        }"#;

        let response: ChatResponse = serde_json::from_str(json).unwrap();
        assert_eq!(response.id, "gen-123");
        assert_eq!(response.model, "anthropic/claude-sonnet-4-20250514");
        assert_eq!(response.choices.len(), 1);
        assert_eq!(
            response.choices[0].message.content,
            "Hello! How can I help you?"
        );
        assert_eq!(response.usage.as_ref().unwrap().total_tokens, 18);
    }

    #[test]
    fn test_llm_response_from_chat_response() {
        let chat_response = ChatResponse {
            id: "test".to_string(),
            object: "chat.completion".to_string(),
            created: 1234567890,
            model: "test-model".to_string(),
            choices: vec![Choice {
                index: 0,
                message: Message::assistant("Test response"),
                finish_reason: Some(FinishReason::Stop),
            }],
            usage: Some(Usage {
                prompt_tokens: 10,
                completion_tokens: 5,
                total_tokens: 15,
            }),
        };

        let llm_response = LlmResponse::from_chat_response(chat_response).unwrap();
        assert_eq!(llm_response.content, "Test response");
        assert_eq!(llm_response.model, "test-model");
        assert_eq!(llm_response.tokens_used, 15);
        assert_eq!(llm_response.input_tokens, 10);
        assert_eq!(llm_response.output_tokens, 5);
        assert_eq!(llm_response.finish_reason, FinishReason::Stop);
    }
}
