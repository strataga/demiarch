//! Chat commands for conversational discovery

use crate::Result;

/// Send a chat message
pub async fn send(_project_id: &str, _message: &str) -> Result<String> {
    todo!("Implement chat message handling")
}

/// Get chat history
pub async fn history(_project_id: &str, _limit: usize) -> Result<Vec<ChatMessage>> {
    todo!("Implement chat history")
}

#[derive(Debug)]
pub struct ChatMessage {
    pub id: String,
    pub role: String,
    pub content: String,
    pub created_at: String,
}
