//! LLM integration - OpenRouter API
//!
//! This module provides:
//! - OpenRouter HTTP client for chat completions and embeddings
//! - Request/response types matching OpenAI-compatible API
//! - Cost tracking integration
//! - Model fallback with automatic retry
//! - Streaming response support
//! - Embedding generation for semantic search

mod client;
mod streaming;
mod types;

pub use client::LlmClient;
pub use streaming::{StreamChunk, StreamEvent};
pub use types::{
    ChatRequest, ChatResponse, Choice, Embedding, EmbeddingData, EmbeddingInput, EmbeddingRequest,
    EmbeddingResponse, EmbeddingUsage, FinishReason, LlmResponse, Message, MessageRole, Usage,
};
