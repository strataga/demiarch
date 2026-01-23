//! Chat infrastructure module
//!
//! Database repositories for conversations and messages.

pub mod repository;

pub use repository::{ConversationRepository, MessageRepository};
