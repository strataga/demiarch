//! Chat commands for conversational discovery

use crate::Result;

/// Send a chat message
pub async fn send(_project_id: &str, _message: &str) -> Result<String> {
    Ok("Chat message received. (Not yet implemented)".to_string())
}

/// Get chat history
pub async fn history(_project_id: &str, _limit: usize) -> Result<Vec<ChatMessage>> {
    Ok(Vec::new())
}

#[derive(Debug, Clone)]
pub struct ChatMessage {
    pub id: String,
    pub role: String,
    pub content: String,
    pub created_at: String,
}
