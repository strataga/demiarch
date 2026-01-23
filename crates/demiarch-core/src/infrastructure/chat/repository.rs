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
