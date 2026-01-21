//! LLM integration - OpenRouter API
//!
//! This module provides:
//! - OpenRouter HTTP client for chat completions
//! - Request/response types matching OpenAI-compatible API
//! - Cost tracking integration
//! - Model fallback with automatic retry
//! - Streaming response support

mod client;
mod streaming;
mod types;

pub use client::LlmClient;
pub use streaming::{StreamChunk, StreamEvent};
pub use types::{
    ChatRequest, ChatResponse, Choice, FinishReason, LlmResponse, Message, MessageRole, Usage,
};
