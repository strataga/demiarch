//! Chat repository implementations
//!
//! Database operations for conversations and messages.

use chrono::Utc;
use sqlx::Row;

use crate::commands::chat::{ChatMessage, Conversation, MessageRole};
use crate::storage::Database;
use crate::Result;

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

        Ok(rows
            .into_iter()
            .map(|r| self.row_to_conversation(r))
            .collect())
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

        let mut messages: Vec<ChatMessage> =
            rows.into_iter().map(|r| self.row_to_message(r)).collect();
        messages.reverse();
        Ok(messages)
    }

    /// Count messages in a conversation
    pub async fn count_by_conversation(&self, conversation_id: &str) -> Result<i64> {
        let row: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM messages WHERE conversation_id = ?")
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::storage::Database;

    async fn create_test_db() -> Database {
        Database::in_memory()
            .await
            .expect("Failed to create test database")
    }

    async fn create_test_project(db: &Database) -> String {
        let project_id = uuid::Uuid::new_v4().to_string();
        sqlx::query("INSERT INTO projects (id, name, framework) VALUES (?, ?, ?)")
            .bind(&project_id)
            .bind("Test Project")
            .bind("rust")
            .execute(db.pool())
            .await
            .expect("Failed to insert test project");
        project_id
    }

    // ========== ConversationRepository Tests ==========

    #[tokio::test]
    async fn test_conversation_create_and_get() {
        let db = create_test_db().await;
        let project_id = create_test_project(&db).await;
        let repo = ConversationRepository::new(&db);

        let conversation = Conversation::new(&project_id).with_title("Test Conversation");

        repo.create(&conversation).await.expect("Failed to create");

        let retrieved = repo
            .get(&conversation.id)
            .await
            .expect("Failed to get")
            .expect("Conversation not found");

        assert_eq!(retrieved.id, conversation.id);
        assert_eq!(retrieved.project_id, project_id);
        assert_eq!(retrieved.title, Some("Test Conversation".to_string()));
    }

    #[tokio::test]
    async fn test_conversation_list_by_project() {
        let db = create_test_db().await;
        let project_id = create_test_project(&db).await;
        let repo = ConversationRepository::new(&db);

        // Create multiple conversations
        for i in 0..3 {
            let conv = Conversation::new(&project_id).with_title(format!("Conv {}", i));
            repo.create(&conv).await.expect("Failed to create");
        }

        let conversations = repo
            .list_by_project(&project_id)
            .await
            .expect("Failed to list");

        assert_eq!(conversations.len(), 3);
    }

    #[tokio::test]
    async fn test_conversation_update_title() {
        let db = create_test_db().await;
        let project_id = create_test_project(&db).await;
        let repo = ConversationRepository::new(&db);

        let conversation = Conversation::new(&project_id);
        repo.create(&conversation).await.expect("Failed to create");

        repo.update_title(&conversation.id, Some("Updated Title"))
            .await
            .expect("Failed to update title");

        let retrieved = repo.get(&conversation.id).await.unwrap().unwrap();
        assert_eq!(retrieved.title, Some("Updated Title".to_string()));
    }

    #[tokio::test]
    async fn test_conversation_delete() {
        let db = create_test_db().await;
        let project_id = create_test_project(&db).await;
        let repo = ConversationRepository::new(&db);

        let conversation = Conversation::new(&project_id);
        repo.create(&conversation).await.expect("Failed to create");

        assert!(repo.exists(&conversation.id).await.unwrap());

        repo.delete(&conversation.id).await.expect("Failed to delete");

        assert!(!repo.exists(&conversation.id).await.unwrap());
    }

    #[tokio::test]
    async fn test_conversation_touch_updates_timestamp() {
        let db = create_test_db().await;
        let project_id = create_test_project(&db).await;
        let repo = ConversationRepository::new(&db);

        let conversation = Conversation::new(&project_id);
        repo.create(&conversation).await.expect("Failed to create");

        let original = repo.get(&conversation.id).await.unwrap().unwrap();

        // Wait briefly to ensure timestamp changes
        tokio::time::sleep(tokio::time::Duration::from_millis(10)).await;

        repo.touch(&conversation.id).await.expect("Failed to touch");

        let updated = repo.get(&conversation.id).await.unwrap().unwrap();
        assert!(updated.updated_at > original.updated_at);
    }

    // ========== MessageRepository Tests ==========

    #[tokio::test]
    async fn test_message_create_and_get() {
        let db = create_test_db().await;
        let project_id = create_test_project(&db).await;
        let conv_repo = ConversationRepository::new(&db);
        let msg_repo = MessageRepository::new(&db);

        let conversation = Conversation::new(&project_id);
        conv_repo.create(&conversation).await.unwrap();

        let message = ChatMessage::user(&conversation.id, "Hello!");
        msg_repo.create(&message).await.expect("Failed to create");

        let retrieved = msg_repo
            .get(&message.id)
            .await
            .expect("Failed to get")
            .expect("Message not found");

        assert_eq!(retrieved.id, message.id);
        assert_eq!(retrieved.content, "Hello!");
        assert_eq!(retrieved.role, MessageRole::User);
    }

    #[tokio::test]
    async fn test_message_list_by_conversation() {
        let db = create_test_db().await;
        let project_id = create_test_project(&db).await;
        let conv_repo = ConversationRepository::new(&db);
        let msg_repo = MessageRepository::new(&db);

        let conversation = Conversation::new(&project_id);
        conv_repo.create(&conversation).await.unwrap();

        // Create messages in order
        for i in 0..5 {
            let msg = ChatMessage::user(&conversation.id, format!("Message {}", i));
            msg_repo.create(&msg).await.unwrap();
        }

        let messages = msg_repo
            .list_by_conversation(&conversation.id)
            .await
            .expect("Failed to list");

        assert_eq!(messages.len(), 5);
        // Messages should be in chronological order (ASC)
        assert_eq!(messages[0].content, "Message 0");
        assert_eq!(messages[4].content, "Message 4");
    }

    #[tokio::test]
    async fn test_message_list_recent() {
        let db = create_test_db().await;
        let project_id = create_test_project(&db).await;
        let conv_repo = ConversationRepository::new(&db);
        let msg_repo = MessageRepository::new(&db);

        let conversation = Conversation::new(&project_id);
        conv_repo.create(&conversation).await.unwrap();

        // Create 10 messages
        for i in 0..10 {
            let msg = ChatMessage::user(&conversation.id, format!("Message {}", i));
            msg_repo.create(&msg).await.unwrap();
        }

        // Get only the last 3 messages
        let recent = msg_repo
            .list_recent(&conversation.id, 3)
            .await
            .expect("Failed to get recent");

        assert_eq!(recent.len(), 3);
        // Should be the last 3 messages in chronological order
        assert_eq!(recent[0].content, "Message 7");
        assert_eq!(recent[1].content, "Message 8");
        assert_eq!(recent[2].content, "Message 9");
    }

    #[tokio::test]
    async fn test_message_pagination() {
        let db = create_test_db().await;
        let project_id = create_test_project(&db).await;
        let conv_repo = ConversationRepository::new(&db);
        let msg_repo = MessageRepository::new(&db);

        let conversation = Conversation::new(&project_id);
        conv_repo.create(&conversation).await.unwrap();

        // Create 10 messages
        for i in 0..10 {
            let msg = ChatMessage::user(&conversation.id, format!("Message {}", i));
            msg_repo.create(&msg).await.unwrap();
        }

        // Get first page
        let page1 = msg_repo
            .list_by_conversation_paginated(&conversation.id, 3, 0)
            .await
            .expect("Failed to get page 1");
        assert_eq!(page1.len(), 3);
        assert_eq!(page1[0].content, "Message 0");

        // Get second page
        let page2 = msg_repo
            .list_by_conversation_paginated(&conversation.id, 3, 3)
            .await
            .expect("Failed to get page 2");
        assert_eq!(page2.len(), 3);
        assert_eq!(page2[0].content, "Message 3");
    }

    #[tokio::test]
    async fn test_message_count() {
        let db = create_test_db().await;
        let project_id = create_test_project(&db).await;
        let conv_repo = ConversationRepository::new(&db);
        let msg_repo = MessageRepository::new(&db);

        let conversation = Conversation::new(&project_id);
        conv_repo.create(&conversation).await.unwrap();

        assert_eq!(
            msg_repo.count_by_conversation(&conversation.id).await.unwrap(),
            0
        );

        for i in 0..5 {
            let msg = ChatMessage::user(&conversation.id, format!("Message {}", i));
            msg_repo.create(&msg).await.unwrap();
        }

        assert_eq!(
            msg_repo.count_by_conversation(&conversation.id).await.unwrap(),
            5
        );
    }

    #[tokio::test]
    async fn test_message_delete() {
        let db = create_test_db().await;
        let project_id = create_test_project(&db).await;
        let conv_repo = ConversationRepository::new(&db);
        let msg_repo = MessageRepository::new(&db);

        let conversation = Conversation::new(&project_id);
        conv_repo.create(&conversation).await.unwrap();

        let message = ChatMessage::user(&conversation.id, "To delete");
        msg_repo.create(&message).await.unwrap();

        assert!(msg_repo.get(&message.id).await.unwrap().is_some());

        msg_repo.delete(&message.id).await.expect("Failed to delete");

        assert!(msg_repo.get(&message.id).await.unwrap().is_none());
    }

    #[tokio::test]
    async fn test_message_delete_by_conversation() {
        let db = create_test_db().await;
        let project_id = create_test_project(&db).await;
        let conv_repo = ConversationRepository::new(&db);
        let msg_repo = MessageRepository::new(&db);

        let conversation = Conversation::new(&project_id);
        conv_repo.create(&conversation).await.unwrap();

        for i in 0..5 {
            let msg = ChatMessage::user(&conversation.id, format!("Message {}", i));
            msg_repo.create(&msg).await.unwrap();
        }

        assert_eq!(
            msg_repo.count_by_conversation(&conversation.id).await.unwrap(),
            5
        );

        msg_repo
            .delete_by_conversation(&conversation.id)
            .await
            .expect("Failed to delete");

        assert_eq!(
            msg_repo.count_by_conversation(&conversation.id).await.unwrap(),
            0
        );
    }

    #[tokio::test]
    async fn test_message_with_model_and_tokens() {
        let db = create_test_db().await;
        let project_id = create_test_project(&db).await;
        let conv_repo = ConversationRepository::new(&db);
        let msg_repo = MessageRepository::new(&db);

        let conversation = Conversation::new(&project_id);
        conv_repo.create(&conversation).await.unwrap();

        let message = ChatMessage::assistant(&conversation.id, "Response")
            .with_model("gpt-4")
            .with_tokens(100);
        msg_repo.create(&message).await.unwrap();

        let retrieved = msg_repo.get(&message.id).await.unwrap().unwrap();
        assert_eq!(retrieved.model, Some("gpt-4".to_string()));
        assert_eq!(retrieved.tokens_used, Some(100));
    }
}
