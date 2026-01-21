//! Streaming response support for OpenRouter API
//!
//! Server-Sent Events (SSE) parsing for streaming chat completions.

use serde::Deserialize;

use super::types::{FinishReason, MessageRole};

/// A delta update in a streaming response
#[derive(Debug, Clone, Deserialize)]
pub struct StreamDelta {
    /// Role of the message (only present in first chunk)
    pub role: Option<MessageRole>,
    /// Content fragment
    pub content: Option<String>,
}

/// A streaming choice (partial response)
#[derive(Debug, Clone, Deserialize)]
pub struct StreamChoice {
    /// Index of this choice
    pub index: usize,
    /// Incremental content update
    pub delta: StreamDelta,
    /// Reason for finishing (only in final chunk)
    pub finish_reason: Option<FinishReason>,
}

/// A chunk from a streaming response
#[derive(Debug, Clone, Deserialize)]
pub struct StreamChunk {
    /// Unique identifier for this completion
    pub id: String,
    /// Object type (always "chat.completion.chunk")
    pub object: String,
    /// Unix timestamp of when the chunk was created
    pub created: u64,
    /// Model used for the completion
    pub model: String,
    /// List of streaming choices
    pub choices: Vec<StreamChoice>,
}

impl StreamChunk {
    /// Get the content from this chunk (if any)
    pub fn content(&self) -> Option<&str> {
        self.choices.first()?.delta.content.as_deref()
    }

    /// Check if this is the final chunk
    pub fn is_done(&self) -> bool {
        self.choices
            .first()
            .and_then(|c| c.finish_reason.as_ref())
            .is_some()
    }

    /// Get the finish reason (if this is the final chunk)
    pub fn finish_reason(&self) -> Option<&FinishReason> {
        self.choices.first()?.finish_reason.as_ref()
    }
}

/// Event from streaming response parsing
#[derive(Debug, Clone)]
pub enum StreamEvent {
    /// A content chunk was received
    Chunk(StreamChunk),
    /// Stream completed
    Done,
    /// Error parsing chunk
    Error(String),
}

/// Parse a Server-Sent Events line into a StreamEvent
pub fn parse_sse_line(line: &str) -> Option<StreamEvent> {
    let line = line.trim();

    // Skip empty lines and comments
    if line.is_empty() || line.starts_with(':') {
        return None;
    }

    // Handle "data: [DONE]" marker
    if line == "data: [DONE]" {
        return Some(StreamEvent::Done);
    }

    // Parse "data: {json}" lines
    if let Some(data) = line.strip_prefix("data: ") {
        match serde_json::from_str::<StreamChunk>(data) {
            Ok(chunk) => Some(StreamEvent::Chunk(chunk)),
            Err(e) => Some(StreamEvent::Error(format!("Failed to parse chunk: {}", e))),
        }
    } else {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_sse_content_chunk() {
        let line = r#"data: {"id":"gen-123","object":"chat.completion.chunk","created":1234567890,"model":"test","choices":[{"index":0,"delta":{"content":"Hello"},"finish_reason":null}]}"#;

        let event = parse_sse_line(line).unwrap();
        match event {
            StreamEvent::Chunk(chunk) => {
                assert_eq!(chunk.content(), Some("Hello"));
                assert!(!chunk.is_done());
            }
            _ => panic!("Expected Chunk event"),
        }
    }

    #[test]
    fn test_parse_sse_done() {
        let line = "data: [DONE]";
        let event = parse_sse_line(line).unwrap();
        assert!(matches!(event, StreamEvent::Done));
    }

    #[test]
    fn test_parse_sse_empty_line() {
        assert!(parse_sse_line("").is_none());
        assert!(parse_sse_line("   ").is_none());
    }

    #[test]
    fn test_parse_sse_comment() {
        assert!(parse_sse_line(": keep-alive").is_none());
    }

    #[test]
    fn test_parse_sse_final_chunk() {
        let line = r#"data: {"id":"gen-123","object":"chat.completion.chunk","created":1234567890,"model":"test","choices":[{"index":0,"delta":{},"finish_reason":"stop"}]}"#;

        let event = parse_sse_line(line).unwrap();
        match event {
            StreamEvent::Chunk(chunk) => {
                assert!(chunk.is_done());
                assert_eq!(chunk.finish_reason(), Some(&FinishReason::Stop));
            }
            _ => panic!("Expected Chunk event"),
        }
    }

    #[test]
    fn test_stream_chunk_content() {
        let chunk = StreamChunk {
            id: "test".to_string(),
            object: "chat.completion.chunk".to_string(),
            created: 0,
            model: "test".to_string(),
            choices: vec![StreamChoice {
                index: 0,
                delta: StreamDelta {
                    role: None,
                    content: Some("Hello".to_string()),
                },
                finish_reason: None,
            }],
        };

        assert_eq!(chunk.content(), Some("Hello"));
        assert!(!chunk.is_done());
    }
}
