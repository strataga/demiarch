//! Chat commands for conversational discovery
//!
//! Provides conversation management and message threading for demiarch projects.

use crate::Result;
use crate::storage::Database;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::Row;
use uuid::Uuid;

/// Message role in a conversation
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum MessageRole {
    #[default]
    User,
    Assistant,
    System,
}

impl MessageRole {
    /// Convert to string for database storage
    pub fn as_str(&self) -> &'static str {
        match self {
            MessageRole::User => "user",
            MessageRole::Assistant => "assistant",
            MessageRole::System => "system",
        }
    }

    /// Parse from database string
    pub fn parse(s: &str) -> Option<Self> {
        match s {
            "user" => Some(MessageRole::User),
            "assistant" => Some(MessageRole::Assistant),
            "system" => Some(MessageRole::System),
            _ => None,
        }
    }
}

/// A chat message within a conversation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatMessage {
    /// Unique message identifier
    pub id: String,
    /// ID of the conversation this message belongs to
    pub conversation_id: String,
    /// Role of the message sender (user, assistant, system)
    pub role: MessageRole,
    /// Message content
    pub content: String,
    /// Model used for assistant responses (optional)
    pub model: Option<String>,
    /// Number of tokens used (optional)
    pub tokens_used: Option<i32>,
    /// When the message was created
    pub created_at: DateTime<Utc>,
}

impl ChatMessage {
    /// Create a new user message
    pub fn user(conversation_id: impl Into<String>, content: impl Into<String>) -> Self {
        Self {
            id: Uuid::new_v4().to_string(),
            conversation_id: conversation_id.into(),
            role: MessageRole::User,
            content: content.into(),
            model: None,
            tokens_used: None,
            created_at: Utc::now(),
        }
    }

    /// Create a new assistant message
    pub fn assistant(conversation_id: impl Into<String>, content: impl Into<String>) -> Self {
        Self {
            id: Uuid::new_v4().to_string(),
            conversation_id: conversation_id.into(),
            role: MessageRole::Assistant,
            content: content.into(),
            model: None,
            tokens_used: None,
            created_at: Utc::now(),
        }
    }

    /// Create a new system message
    pub fn system(conversation_id: impl Into<String>, content: impl Into<String>) -> Self {
        Self {
            id: Uuid::new_v4().to_string(),
            conversation_id: conversation_id.into(),
            role: MessageRole::System,
            content: content.into(),
            model: None,
            tokens_used: None,
            created_at: Utc::now(),
        }
    }

    /// Set the model used for this message
    pub fn with_model(mut self, model: impl Into<String>) -> Self {
        self.model = Some(model.into());
        self
    }

    /// Set the number of tokens used
    pub fn with_tokens(mut self, tokens: i32) -> Self {
        self.tokens_used = Some(tokens);
        self
    }
}

/// A conversation thread containing multiple messages
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Conversation {
    /// Unique conversation identifier
    pub id: String,
    /// ID of the project this conversation belongs to
    pub project_id: String,
    /// Optional title for the conversation
    pub title: Option<String>,
    /// When the conversation was created
    pub created_at: DateTime<Utc>,
    /// When the conversation was last updated
    pub updated_at: DateTime<Utc>,
}

impl Conversation {
    /// Create a new conversation for a project
    pub fn new(project_id: impl Into<String>) -> Self {
        let now = Utc::now();
        Self {
            id: Uuid::new_v4().to_string(),
            project_id: project_id.into(),
            title: None,
            created_at: now,
            updated_at: now,
        }
    }

    /// Set the conversation title
    pub fn with_title(mut self, title: impl Into<String>) -> Self {
        self.title = Some(title.into());
        self
    }
}

/// Conversation repository for database operations
pub struct ConversationRepository<'a> {
    db: &'a Database,
}

impl<'a> ConversationRepository<'a> {
    /// Create a new conversation repository
    pub fn new(db: &'a Database) -> Self {
        Self { db }
    }

    /// Create a new conversation in the database
    pub async fn create(&self, conversation: &Conversation) -> Result<()> {
        sqlx::query(
            r#"
            INSERT INTO conversations (id, project_id, title, created_at, updated_at)
            VALUES (?, ?, ?, ?, ?)
            "#,
        )
        .bind(&conversation.id)
        .bind(&conversation.project_id)
        .bind(&conversation.title)
        .bind(conversation.created_at)
        .bind(conversation.updated_at)
        .execute(self.db.pool())
        .await?;

        Ok(())
    }

    /// Get a conversation by ID
    pub async fn get(&self, id: &str) -> Result<Option<Conversation>> {
        let row = sqlx::query(
            "SELECT id, project_id, title, created_at, updated_at FROM conversations WHERE id = ?",
        )
        .bind(id)
        .fetch_optional(self.db.pool())
        .await?;

        Ok(row.map(|r| self.row_to_conversation(r)))
    }

    /// List all conversations for a project
    pub async fn list_by_project(&self, project_id: &str) -> Result<Vec<Conversation>> {
        let rows = sqlx::query(
            "SELECT id, project_id, title, created_at, updated_at FROM conversations WHERE project_id = ? ORDER BY updated_at DESC",
        )
        .bind(project_id)
        .fetch_all(self.db.pool())
        .await?;

        Ok(rows.into_iter().map(|r| self.row_to_conversation(r)).collect())
    }

    /// Update a conversation's title
    pub async fn update_title(&self, id: &str, title: Option<&str>) -> Result<()> {
        sqlx::query("UPDATE conversations SET title = ?, updated_at = ? WHERE id = ?")
            .bind(title)
            .bind(Utc::now())
            .bind(id)
            .execute(self.db.pool())
            .await?;

        Ok(())
    }

    /// Update the conversation's updated_at timestamp
    pub async fn touch(&self, id: &str) -> Result<()> {
        sqlx::query("UPDATE conversations SET updated_at = ? WHERE id = ?")
            .bind(Utc::now())
            .bind(id)
            .execute(self.db.pool())
            .await?;

        Ok(())
    }

    /// Delete a conversation and all its messages
    pub async fn delete(&self, id: &str) -> Result<()> {
        sqlx::query("DELETE FROM conversations WHERE id = ?")
            .bind(id)
            .execute(self.db.pool())
            .await?;

        Ok(())
    }

    /// Check if a conversation exists
    pub async fn exists(&self, id: &str) -> Result<bool> {
        let row: Option<(i32,)> = sqlx::query_as("SELECT 1 FROM conversations WHERE id = ?")
            .bind(id)
            .fetch_optional(self.db.pool())
            .await?;

        Ok(row.is_some())
    }

    /// Convert a database row to a Conversation
    fn row_to_conversation(&self, row: sqlx::sqlite::SqliteRow) -> Conversation {
        Conversation {
            id: row.get("id"),
            project_id: row.get("project_id"),
            title: row.get("title"),
            created_at: row.get("created_at"),
            updated_at: row.get("updated_at"),
        }
    }
}

/// Message repository for database operations
pub struct MessageRepository<'a> {
    db: &'a Database,
}

impl<'a> MessageRepository<'a> {
    /// Create a new message repository
    pub fn new(db: &'a Database) -> Self {
        Self { db }
    }

    /// Create a new message in the database
    pub async fn create(&self, message: &ChatMessage) -> Result<()> {
        sqlx::query(
            r#"
            INSERT INTO messages (id, conversation_id, role, content, model, tokens_used, created_at)
            VALUES (?, ?, ?, ?, ?, ?, ?)
            "#,
        )
        .bind(&message.id)
        .bind(&message.conversation_id)
        .bind(message.role.as_str())
        .bind(&message.content)
        .bind(&message.model)
        .bind(message.tokens_used)
        .bind(message.created_at)
        .execute(self.db.pool())
        .await?;

        Ok(())
    }

    /// Get a message by ID
    pub async fn get(&self, id: &str) -> Result<Option<ChatMessage>> {
        let row = sqlx::query(
            "SELECT id, conversation_id, role, content, model, tokens_used, created_at FROM messages WHERE id = ?",
        )
        .bind(id)
        .fetch_optional(self.db.pool())
        .await?;

        Ok(row.map(|r| self.row_to_message(r)))
    }

    /// List all messages in a conversation (ordered by creation time)
    pub async fn list_by_conversation(&self, conversation_id: &str) -> Result<Vec<ChatMessage>> {
        let rows = sqlx::query(
            "SELECT id, conversation_id, role, content, model, tokens_used, created_at FROM messages WHERE conversation_id = ? ORDER BY created_at ASC",
        )
        .bind(conversation_id)
        .fetch_all(self.db.pool())
        .await?;

        Ok(rows.into_iter().map(|r| self.row_to_message(r)).collect())
    }

    /// List messages with pagination
    pub async fn list_by_conversation_paginated(
        &self,
        conversation_id: &str,
        limit: usize,
        offset: usize,
    ) -> Result<Vec<ChatMessage>> {
        let rows = sqlx::query(
            "SELECT id, conversation_id, role, content, model, tokens_used, created_at FROM messages WHERE conversation_id = ? ORDER BY created_at ASC LIMIT ? OFFSET ?",
        )
        .bind(conversation_id)
        .bind(limit as i64)
        .bind(offset as i64)
        .fetch_all(self.db.pool())
        .await?;

        Ok(rows.into_iter().map(|r| self.row_to_message(r)).collect())
    }

    /// Get the most recent messages in a conversation
    pub async fn list_recent(
        &self,
        conversation_id: &str,
        limit: usize,
    ) -> Result<Vec<ChatMessage>> {
        // Get the most recent messages, then reverse to maintain chronological order
        let rows = sqlx::query(
            "SELECT id, conversation_id, role, content, model, tokens_used, created_at FROM messages WHERE conversation_id = ? ORDER BY created_at DESC LIMIT ?",
        )
        .bind(conversation_id)
        .bind(limit as i64)
        .fetch_all(self.db.pool())
        .await?;

        let mut messages: Vec<ChatMessage> = rows.into_iter().map(|r| self.row_to_message(r)).collect();
        messages.reverse();
        Ok(messages)
    }

    /// Count messages in a conversation
    pub async fn count_by_conversation(&self, conversation_id: &str) -> Result<i64> {
        let row: (i64,) =
            sqlx::query_as("SELECT COUNT(*) FROM messages WHERE conversation_id = ?")
                .bind(conversation_id)
                .fetch_one(self.db.pool())
                .await?;

        Ok(row.0)
    }

    /// Delete a message
    pub async fn delete(&self, id: &str) -> Result<()> {
        sqlx::query("DELETE FROM messages WHERE id = ?")
            .bind(id)
            .execute(self.db.pool())
            .await?;

        Ok(())
    }

    /// Delete all messages in a conversation
    pub async fn delete_by_conversation(&self, conversation_id: &str) -> Result<()> {
        sqlx::query("DELETE FROM messages WHERE conversation_id = ?")
            .bind(conversation_id)
            .execute(self.db.pool())
            .await?;

        Ok(())
    }

    /// Convert a database row to a ChatMessage
    fn row_to_message(&self, row: sqlx::sqlite::SqliteRow) -> ChatMessage {
        ChatMessage {
            id: row.get("id"),
            conversation_id: row.get("conversation_id"),
            role: MessageRole::parse(row.get("role")).unwrap_or_default(),
            content: row.get("content"),
            model: row.get("model"),
            tokens_used: row.get("tokens_used"),
            created_at: row.get("created_at"),
        }
    }
}

// ============================================================================
// High-level chat API functions
// ============================================================================

/// Create a new conversation for a project
pub async fn create_conversation(
    db: &Database,
    project_id: &str,
    title: Option<&str>,
) -> Result<Conversation> {
    // Verify project exists
    let project_exists: Option<(i32,)> = sqlx::query_as("SELECT 1 FROM projects WHERE id = ?")
        .bind(project_id)
        .fetch_optional(db.pool())
        .await?;

    if project_exists.is_none() {
        return Err(crate::Error::ProjectNotFound(project_id.to_string()));
    }

    let mut conversation = Conversation::new(project_id);
    if let Some(t) = title {
        conversation = conversation.with_title(t);
    }

    let repo = ConversationRepository::new(db);
    repo.create(&conversation).await?;

    Ok(conversation)
}

/// List all conversations for a project
pub async fn list_conversations(db: &Database, project_id: &str) -> Result<Vec<Conversation>> {
    let repo = ConversationRepository::new(db);
    repo.list_by_project(project_id).await
}

/// Get a conversation by ID
pub async fn get_conversation(db: &Database, id: &str) -> Result<Option<Conversation>> {
    let repo = ConversationRepository::new(db);
    repo.get(id).await
}

/// Delete a conversation and all its messages
pub async fn delete_conversation(db: &Database, id: &str) -> Result<()> {
    let repo = ConversationRepository::new(db);

    if !repo.exists(id).await? {
        return Err(crate::Error::NotFound(format!(
            "Conversation not found: {}",
            id
        )));
    }

    repo.delete(id).await
}

/// Send a message in a conversation
pub async fn send_message(
    db: &Database,
    conversation_id: &str,
    role: MessageRole,
    content: &str,
) -> Result<ChatMessage> {
    // Verify conversation exists
    let conv_repo = ConversationRepository::new(db);
    if !conv_repo.exists(conversation_id).await? {
        return Err(crate::Error::NotFound(format!(
            "Conversation not found: {}",
            conversation_id
        )));
    }

    let message = match role {
        MessageRole::User => ChatMessage::user(conversation_id, content),
        MessageRole::Assistant => ChatMessage::assistant(conversation_id, content),
        MessageRole::System => ChatMessage::system(conversation_id, content),
    };

    let msg_repo = MessageRepository::new(db);
    msg_repo.create(&message).await?;

    // Update conversation's updated_at timestamp
    conv_repo.touch(conversation_id).await?;

    Ok(message)
}

/// Get chat history for a conversation
pub async fn get_history(
    db: &Database,
    conversation_id: &str,
    limit: Option<usize>,
) -> Result<Vec<ChatMessage>> {
    let msg_repo = MessageRepository::new(db);

    if let Some(limit) = limit {
        msg_repo.list_recent(conversation_id, limit).await
    } else {
        msg_repo.list_by_conversation(conversation_id).await
    }
}

/// Get all messages in a conversation with pagination
pub async fn get_history_paginated(
    db: &Database,
    conversation_id: &str,
    limit: usize,
    offset: usize,
) -> Result<Vec<ChatMessage>> {
    let msg_repo = MessageRepository::new(db);
    msg_repo
        .list_by_conversation_paginated(conversation_id, limit, offset)
        .await
}

/// Count messages in a conversation
pub async fn count_messages(db: &Database, conversation_id: &str) -> Result<i64> {
    let msg_repo = MessageRepository::new(db);
    msg_repo.count_by_conversation(conversation_id).await
}

// ============================================================================
// Legacy API (for backwards compatibility)
// ============================================================================

/// Send a chat message (legacy API)
pub async fn send(_project_id: &str, _message: &str) -> Result<String> {
    Ok("Chat message received. Use send_message() with a Database instance for full functionality.".to_string())
}

/// Get chat history (legacy API)
pub async fn history(_project_id: &str, _limit: usize) -> Result<Vec<LegacyChatMessage>> {
    Ok(Vec::new())
}

/// Legacy chat message format (for backwards compatibility)
#[derive(Debug, Clone)]
pub struct LegacyChatMessage {
    pub id: String,
    pub role: String,
    pub content: String,
    pub created_at: String,
}

impl From<ChatMessage> for LegacyChatMessage {
    fn from(msg: ChatMessage) -> Self {
        Self {
            id: msg.id,
            role: msg.role.as_str().to_string(),
            content: msg.content,
            created_at: msg.created_at.to_rfc3339(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_create_conversation() {
        let db = Database::in_memory().await.expect("Failed to create database");

        // First create a project
        sqlx::query("INSERT INTO projects (id, name) VALUES (?, ?)")
            .bind("test-project-id")
            .bind("Test Project")
            .execute(db.pool())
            .await
            .expect("Failed to create project");

        let conversation = create_conversation(&db, "test-project-id", Some("Test Conversation"))
            .await
            .expect("Failed to create conversation");

        assert_eq!(conversation.project_id, "test-project-id");
        assert_eq!(conversation.title, Some("Test Conversation".to_string()));
    }

    #[tokio::test]
    async fn test_create_conversation_project_not_found() {
        let db = Database::in_memory().await.expect("Failed to create database");

        let result = create_conversation(&db, "nonexistent-project", None).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_send_and_retrieve_messages() {
        let db = Database::in_memory().await.expect("Failed to create database");

        // Create a project
        sqlx::query("INSERT INTO projects (id, name) VALUES (?, ?)")
            .bind("test-project-id")
            .bind("Test Project")
            .execute(db.pool())
            .await
            .expect("Failed to create project");

        // Create a conversation
        let conversation = create_conversation(&db, "test-project-id", Some("Test Chat"))
            .await
            .expect("Failed to create conversation");

        // Send messages
        let msg1 = send_message(&db, &conversation.id, MessageRole::User, "Hello, assistant!")
            .await
            .expect("Failed to send user message");

        let msg2 = send_message(
            &db,
            &conversation.id,
            MessageRole::Assistant,
            "Hello! How can I help you?",
        )
        .await
        .expect("Failed to send assistant message");

        let _msg3 = send_message(&db, &conversation.id, MessageRole::User, "Help me build a feature")
            .await
            .expect("Failed to send second user message");

        // Verify message properties
        assert_eq!(msg1.role, MessageRole::User);
        assert_eq!(msg2.role, MessageRole::Assistant);

        // Get history
        let history = get_history(&db, &conversation.id, None)
            .await
            .expect("Failed to get history");

        assert_eq!(history.len(), 3);
        assert_eq!(history[0].content, "Hello, assistant!");
        assert_eq!(history[1].content, "Hello! How can I help you?");
        assert_eq!(history[2].content, "Help me build a feature");
    }

    #[tokio::test]
    async fn test_get_recent_messages() {
        let db = Database::in_memory().await.expect("Failed to create database");

        // Create project and conversation
        sqlx::query("INSERT INTO projects (id, name) VALUES (?, ?)")
            .bind("test-project-id")
            .bind("Test Project")
            .execute(db.pool())
            .await
            .unwrap();

        let conversation = create_conversation(&db, "test-project-id", None)
            .await
            .unwrap();

        // Send 5 messages
        for i in 1..=5 {
            send_message(
                &db,
                &conversation.id,
                MessageRole::User,
                &format!("Message {}", i),
            )
            .await
            .unwrap();
        }

        // Get only the last 3 messages
        let recent = get_history(&db, &conversation.id, Some(3))
            .await
            .expect("Failed to get recent history");

        assert_eq!(recent.len(), 3);
        assert_eq!(recent[0].content, "Message 3");
        assert_eq!(recent[1].content, "Message 4");
        assert_eq!(recent[2].content, "Message 5");
    }

    #[tokio::test]
    async fn test_list_conversations() {
        let db = Database::in_memory().await.expect("Failed to create database");

        // Create a project
        sqlx::query("INSERT INTO projects (id, name) VALUES (?, ?)")
            .bind("test-project-id")
            .bind("Test Project")
            .execute(db.pool())
            .await
            .unwrap();

        // Create multiple conversations
        create_conversation(&db, "test-project-id", Some("Conversation 1"))
            .await
            .unwrap();
        create_conversation(&db, "test-project-id", Some("Conversation 2"))
            .await
            .unwrap();
        create_conversation(&db, "test-project-id", Some("Conversation 3"))
            .await
            .unwrap();

        let conversations = list_conversations(&db, "test-project-id")
            .await
            .expect("Failed to list conversations");

        assert_eq!(conversations.len(), 3);
    }

    #[tokio::test]
    async fn test_delete_conversation_cascades_messages() {
        let db = Database::in_memory().await.expect("Failed to create database");

        // Create project and conversation
        sqlx::query("INSERT INTO projects (id, name) VALUES (?, ?)")
            .bind("test-project-id")
            .bind("Test Project")
            .execute(db.pool())
            .await
            .unwrap();

        let conversation = create_conversation(&db, "test-project-id", Some("To Delete"))
            .await
            .unwrap();

        // Add messages
        send_message(&db, &conversation.id, MessageRole::User, "Message 1")
            .await
            .unwrap();
        send_message(&db, &conversation.id, MessageRole::Assistant, "Response 1")
            .await
            .unwrap();

        // Verify messages exist
        let count = count_messages(&db, &conversation.id).await.unwrap();
        assert_eq!(count, 2);

        // Delete conversation
        delete_conversation(&db, &conversation.id).await.unwrap();

        // Verify conversation is gone
        let conv = get_conversation(&db, &conversation.id).await.unwrap();
        assert!(conv.is_none());

        // Verify messages are also deleted (cascade)
        let messages: Vec<(String,)> =
            sqlx::query_as("SELECT id FROM messages WHERE conversation_id = ?")
                .bind(&conversation.id)
                .fetch_all(db.pool())
                .await
                .unwrap();
        assert!(messages.is_empty());
    }

    #[tokio::test]
    async fn test_message_with_model_and_tokens() {
        let db = Database::in_memory().await.expect("Failed to create database");

        // Create project and conversation
        sqlx::query("INSERT INTO projects (id, name) VALUES (?, ?)")
            .bind("test-project-id")
            .bind("Test Project")
            .execute(db.pool())
            .await
            .unwrap();

        let conversation = create_conversation(&db, "test-project-id", None)
            .await
            .unwrap();

        // Create a message with model and token info
        let message = ChatMessage::assistant(&conversation.id, "Hello!")
            .with_model("gpt-4")
            .with_tokens(150);

        let msg_repo = MessageRepository::new(&db);
        msg_repo.create(&message).await.unwrap();

        // Retrieve and verify
        let retrieved = msg_repo.get(&message.id).await.unwrap().unwrap();
        assert_eq!(retrieved.model, Some("gpt-4".to_string()));
        assert_eq!(retrieved.tokens_used, Some(150));
    }

    #[tokio::test]
    async fn test_conversation_updated_at() {
        let db = Database::in_memory().await.expect("Failed to create database");

        // Create project
        sqlx::query("INSERT INTO projects (id, name) VALUES (?, ?)")
            .bind("test-project-id")
            .bind("Test Project")
            .execute(db.pool())
            .await
            .unwrap();

        let conversation = create_conversation(&db, "test-project-id", None)
            .await
            .unwrap();

        let original_updated = conversation.updated_at;

        // Wait a tiny bit to ensure timestamp difference
        tokio::time::sleep(tokio::time::Duration::from_millis(10)).await;

        // Send a message (should update conversation)
        send_message(&db, &conversation.id, MessageRole::User, "Test")
            .await
            .unwrap();

        // Retrieve updated conversation
        let updated = get_conversation(&db, &conversation.id).await.unwrap().unwrap();

        assert!(updated.updated_at > original_updated);
    }

    #[tokio::test]
    async fn test_paginated_history() {
        let db = Database::in_memory().await.expect("Failed to create database");

        // Create project and conversation
        sqlx::query("INSERT INTO projects (id, name) VALUES (?, ?)")
            .bind("test-project-id")
            .bind("Test Project")
            .execute(db.pool())
            .await
            .unwrap();

        let conversation = create_conversation(&db, "test-project-id", None)
            .await
            .unwrap();

        // Send 10 messages
        for i in 1..=10 {
            send_message(
                &db,
                &conversation.id,
                MessageRole::User,
                &format!("Message {}", i),
            )
            .await
            .unwrap();
        }

        // Get first page (messages 1-5)
        let page1 = get_history_paginated(&db, &conversation.id, 5, 0)
            .await
            .unwrap();
        assert_eq!(page1.len(), 5);
        assert_eq!(page1[0].content, "Message 1");
        assert_eq!(page1[4].content, "Message 5");

        // Get second page (messages 6-10)
        let page2 = get_history_paginated(&db, &conversation.id, 5, 5)
            .await
            .unwrap();
        assert_eq!(page2.len(), 5);
        assert_eq!(page2[0].content, "Message 6");
        assert_eq!(page2[4].content, "Message 10");
    }

    #[tokio::test]
    async fn test_update_conversation_title() {
        let db = Database::in_memory().await.expect("Failed to create database");

        // Create project
        sqlx::query("INSERT INTO projects (id, name) VALUES (?, ?)")
            .bind("test-project-id")
            .bind("Test Project")
            .execute(db.pool())
            .await
            .unwrap();

        let conversation = create_conversation(&db, "test-project-id", Some("Original Title"))
            .await
            .unwrap();

        // Update title
        let repo = ConversationRepository::new(&db);
        repo.update_title(&conversation.id, Some("New Title"))
            .await
            .unwrap();

        // Verify
        let updated = get_conversation(&db, &conversation.id).await.unwrap().unwrap();
        assert_eq!(updated.title, Some("New Title".to_string()));
    }

    #[tokio::test]
    async fn test_message_roles() {
        assert_eq!(MessageRole::User.as_str(), "user");
        assert_eq!(MessageRole::Assistant.as_str(), "assistant");
        assert_eq!(MessageRole::System.as_str(), "system");

        assert_eq!(MessageRole::parse("user"), Some(MessageRole::User));
        assert_eq!(MessageRole::parse("assistant"), Some(MessageRole::Assistant));
        assert_eq!(MessageRole::parse("system"), Some(MessageRole::System));
        assert_eq!(MessageRole::parse("invalid"), None);
    }
}
