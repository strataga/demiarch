//! Database types and utilities

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use uuid::Uuid;

/// Database record ID type
pub type DbId = Uuid;

/// Project database record
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProjectRecord {
    pub id: DbId,
    pub name: String,
    pub description: Option<String>,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub updated_at: chrono::DateTime<chrono::Utc>,
    pub settings: HashMap<String, serde_json::Value>,
    pub status: ProjectStatus,
    pub metadata: HashMap<String, serde_json::Value>,
}

/// Project status
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
pub enum ProjectStatus {
    #[default]
    Active,
    Archived,
    Deleted,
}

impl From<String> for ProjectStatus {
    fn from(s: String) -> Self {
        match s.to_lowercase().as_str() {
            "archived" => Self::Archived,
            "deleted" => Self::Deleted,
            _ => Self::Active,
        }
    }
}

impl From<ProjectStatus> for String {
    fn from(value: ProjectStatus) -> Self {
        match value {
            ProjectStatus::Active => "active".to_string(),
            ProjectStatus::Archived => "archived".to_string(),
            ProjectStatus::Deleted => "deleted".to_string(),
        }
    }
}

/// Conversation database record
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConversationRecord {
    pub id: DbId,
    pub project_id: DbId,
    pub title: String,
    pub messages: Vec<ConversationMessage>,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub updated_at: chrono::DateTime<chrono::Utc>,
    pub metadata: HashMap<String, serde_json::Value>,
}

/// Conversation message
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConversationMessage {
    pub id: DbId,
    pub role: MessageRole,
    pub content: String,
    pub timestamp: chrono::DateTime<chrono::Utc>,
    pub metadata: HashMap<String, serde_json::Value>,
}

/// Message role
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum MessageRole {
    User,
    Assistant,
    System,
}

impl From<String> for MessageRole {
    fn from(s: String) -> Self {
        match s.to_lowercase().as_str() {
            "assistant" => Self::Assistant,
            "system" => Self::System,
            _ => Self::User,
        }
    }
}

impl From<MessageRole> for String {
    fn from(value: MessageRole) -> Self {
        match value {
            MessageRole::User => "user".to_string(),
            MessageRole::Assistant => "assistant".to_string(),
            MessageRole::System => "system".to_string(),
        }
    }
}

/// Agent database record
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentRecord {
    pub id: DbId,
    pub project_id: DbId,
    pub name: String,
    pub agent_type: String,
    pub configuration: HashMap<String, serde_json::Value>,
    pub state: HashMap<String, serde_json::Value>,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub updated_at: chrono::DateTime<chrono::Utc>,
    pub status: AgentStatus,
}

/// Agent status
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
pub enum AgentStatus {
    #[default]
    Active,
    Inactive,
    Paused,
    Error,
}

impl From<String> for AgentStatus {
    fn from(s: String) -> Self {
        match s.to_lowercase().as_str() {
            "inactive" => Self::Inactive,
            "paused" => Self::Paused,
            "error" => Self::Error,
            _ => Self::Active,
        }
    }
}

impl From<AgentStatus> for String {
    fn from(value: AgentStatus) -> Self {
        match value {
            AgentStatus::Active => "active".to_string(),
            AgentStatus::Inactive => "inactive".to_string(),
            AgentStatus::Paused => "paused".to_string(),
            AgentStatus::Error => "error".to_string(),
        }
    }
}

/// Skill database record
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SkillRecord {
    pub id: DbId,
    pub project_id: DbId,
    pub name: String,
    pub description: Option<String>,
    pub category: String,
    pub code: String,
    pub metadata: HashMap<String, serde_json::Value>,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub updated_at: chrono::DateTime<chrono::Utc>,
}

/// Code generation database record
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CodeGenerationRecord {
    pub id: DbId,
    pub project_id: DbId,
    pub conversation_id: Option<DbId>,
    pub agent_id: Option<DbId>,
    pub generation_type: String,
    pub language: String,
    pub code: String,
    pub file_path: Option<String>,
    pub dependencies: Vec<String>,
    pub metadata: HashMap<String, serde_json::Value>,
    pub created_at: chrono::DateTime<chrono::Utc>,
}

/// LLM call database record for cost tracking
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LlmCallRecord {
    pub id: DbId,
    pub project_id: DbId,
    pub conversation_id: Option<DbId>,
    pub agent_id: Option<DbId>,
    pub model: String,
    pub provider: String,
    pub prompt_tokens: u32,
    pub completion_tokens: u32,
    pub total_tokens: u32,
    pub cost_usd: f64,
    pub duration_ms: u32,
    pub status: CallStatus,
    pub error_message: Option<String>,
    pub request_text: Option<String>,
    pub response_text: Option<String>,
    pub created_at: chrono::DateTime<chrono::Utc>,
}

/// LLM call status
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
pub enum CallStatus {
    #[default]
    Success,
    Error,
    Timeout,
    Cancelled,
}

impl From<String> for CallStatus {
    fn from(s: String) -> Self {
        match s.to_lowercase().as_str() {
            "error" => Self::Error,
            "timeout" => Self::Timeout,
            "cancelled" => Self::Cancelled,
            _ => Self::Success,
        }
    }
}

impl From<CallStatus> for String {
    fn from(value: CallStatus) -> Self {
        match value {
            CallStatus::Success => "success".to_string(),
            CallStatus::Error => "error".to_string(),
            CallStatus::Timeout => "timeout".to_string(),
            CallStatus::Cancelled => "cancelled".to_string(),
        }
    }
}

/// Checkpoint database record
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CheckpointRecord {
    pub id: DbId,
    pub project_id: DbId,
    pub name: String,
    pub description: Option<String>,
    pub checkpoint_type: String,
    pub data: serde_json::Value,
    pub metadata: HashMap<String, serde_json::Value>,
    pub created_at: chrono::DateTime<chrono::Utc>,
}

/// Plugin database record
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PluginRecord {
    pub id: DbId,
    pub project_id: DbId,
    pub name: String,
    pub version: String,
    pub plugin_type: String,
    pub configuration: HashMap<String, serde_json::Value>,
    pub state: HashMap<String, serde_json::Value>,
    pub enabled: bool,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub updated_at: chrono::DateTime<chrono::Utc>,
}

/// Session database record
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionRecord {
    pub id: DbId,
    pub project_id: DbId,
    pub user_id: Option<DbId>,
    pub session_data: HashMap<String, serde_json::Value>,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub expires_at: Option<chrono::DateTime<chrono::Utc>>,
    pub last_accessed: chrono::DateTime<chrono::Utc>,
    pub status: SessionStatus,
}

/// Session status
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
pub enum SessionStatus {
    #[default]
    Active,
    Expired,
    Revoked,
}

impl From<String> for SessionStatus {
    fn from(s: String) -> Self {
        match s.to_lowercase().as_str() {
            "expired" => Self::Expired,
            "revoked" => Self::Revoked,
            _ => Self::Active,
        }
    }
}

impl From<SessionStatus> for String {
    fn from(value: SessionStatus) -> Self {
        match value {
            SessionStatus::Active => "active".to_string(),
            SessionStatus::Expired => "expired".to_string(),
            SessionStatus::Revoked => "revoked".to_string(),
        }
    }
}

/// Database query result
#[derive(Debug, Clone)]
pub struct QueryResult<T> {
    pub data: Vec<T>,
    pub total: u32,
    pub page: u32,
    pub page_size: u32,
}

/// Database error types
#[derive(Debug, thiserror::Error)]
pub enum DatabaseError {
    #[error("Connection error: {0}")]
    Connection(String),

    #[error("Query error: {0}")]
    Query(String),

    #[error("Migration error: {0}")]
    Migration(String),

    #[error("Record not found: {0}")]
    NotFound(String),

    #[error("Validation error: {0}")]
    Validation(String),

    #[error("Concurrency error: {0}")]
    Concurrency(String),

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("SQLx error: {0}")]
    Sqlx(#[from] sqlx::Error),

    #[error("JSON error: {0}")]
    Json(#[from] serde_json::Error),

    #[error("Other error: {0}")]
    Other(#[from] anyhow::Error),
}

/// Database operation result
pub type DatabaseResult<T> = Result<T, DatabaseError>;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_project_status_default() {
        assert_eq!(ProjectStatus::default(), ProjectStatus::Active);
    }

    #[test]
    fn test_project_status_from_string() {
        assert_eq!(
            ProjectStatus::from("active".to_string()),
            ProjectStatus::Active
        );
        assert_eq!(
            ProjectStatus::from("ACTIVE".to_string()),
            ProjectStatus::Active
        );
        assert_eq!(
            ProjectStatus::from("archived".to_string()),
            ProjectStatus::Archived
        );
        assert_eq!(
            ProjectStatus::from("deleted".to_string()),
            ProjectStatus::Deleted
        );
        assert_eq!(
            ProjectStatus::from("unknown".to_string()),
            ProjectStatus::Active
        );
    }

    #[test]
    fn test_project_status_to_string() {
        let status: String = String::from(ProjectStatus::Active);
        assert_eq!(status, "active".to_string());

        let status: String = String::from(ProjectStatus::Archived);
        assert_eq!(status, "archived".to_string());

        let status: String = String::from(ProjectStatus::Deleted);
        assert_eq!(status, "deleted".to_string());
    }

    #[test]
    fn test_message_role_from_string() {
        assert_eq!(MessageRole::from("user".to_string()), MessageRole::User);
        assert_eq!(
            MessageRole::from("assistant".to_string()),
            MessageRole::Assistant
        );
        assert_eq!(MessageRole::from("system".to_string()), MessageRole::System);
        assert_eq!(MessageRole::from("unknown".to_string()), MessageRole::User);
    }

    #[test]
    fn test_message_role_to_string() {
        let role: String = String::from(MessageRole::User);
        assert_eq!(role, "user".to_string());

        let role: String = String::from(MessageRole::Assistant);
        assert_eq!(role, "assistant".to_string());

        let role: String = String::from(MessageRole::System);
        assert_eq!(role, "system".to_string());
    }

    #[test]
    fn test_agent_status_default() {
        assert_eq!(AgentStatus::default(), AgentStatus::Active);
    }

    #[test]
    fn test_agent_status_from_string() {
        assert_eq!(AgentStatus::from("active".to_string()), AgentStatus::Active);
        assert_eq!(
            AgentStatus::from("inactive".to_string()),
            AgentStatus::Inactive
        );
        assert_eq!(AgentStatus::from("paused".to_string()), AgentStatus::Paused);
        assert_eq!(AgentStatus::from("error".to_string()), AgentStatus::Error);
        assert_eq!(
            AgentStatus::from("unknown".to_string()),
            AgentStatus::Active
        );
    }

    #[test]
    fn test_agent_status_to_string() {
        let status: String = String::from(AgentStatus::Active);
        assert_eq!(status, "active".to_string());

        let status: String = String::from(AgentStatus::Inactive);
        assert_eq!(status, "inactive".to_string());

        let status: String = String::from(AgentStatus::Paused);
        assert_eq!(status, "paused".to_string());

        let status: String = String::from(AgentStatus::Error);
        assert_eq!(status, "error".to_string());
    }

    #[test]
    fn test_call_status_default() {
        assert_eq!(CallStatus::default(), CallStatus::Success);
    }

    #[test]
    fn test_call_status_from_string() {
        assert_eq!(CallStatus::from("success".to_string()), CallStatus::Success);
        assert_eq!(CallStatus::from("error".to_string()), CallStatus::Error);
        assert_eq!(CallStatus::from("timeout".to_string()), CallStatus::Timeout);
        assert_eq!(
            CallStatus::from("cancelled".to_string()),
            CallStatus::Cancelled
        );
        assert_eq!(CallStatus::from("unknown".to_string()), CallStatus::Success);
    }

    #[test]
    fn test_call_status_to_string() {
        let status: String = String::from(CallStatus::Success);
        assert_eq!(status, "success".to_string());

        let status: String = String::from(CallStatus::Error);
        assert_eq!(status, "error".to_string());

        let status: String = String::from(CallStatus::Timeout);
        assert_eq!(status, "timeout".to_string());

        let status: String = String::from(CallStatus::Cancelled);
        assert_eq!(status, "cancelled".to_string());
    }

    #[test]
    fn test_session_status_default() {
        assert_eq!(SessionStatus::default(), SessionStatus::Active);
    }

    #[test]
    fn test_session_status_from_string() {
        assert_eq!(
            SessionStatus::from("active".to_string()),
            SessionStatus::Active
        );
        assert_eq!(
            SessionStatus::from("expired".to_string()),
            SessionStatus::Expired
        );
        assert_eq!(
            SessionStatus::from("revoked".to_string()),
            SessionStatus::Revoked
        );
        assert_eq!(
            SessionStatus::from("unknown".to_string()),
            SessionStatus::Active
        );
    }

    #[test]
    fn test_session_status_to_string() {
        let status: String = String::from(SessionStatus::Active);
        assert_eq!(status, "active".to_string());

        let status: String = String::from(SessionStatus::Expired);
        assert_eq!(status, "expired".to_string());

        let status: String = String::from(SessionStatus::Revoked);
        assert_eq!(status, "revoked".to_string());
    }

    #[test]
    fn test_query_result() {
        let result: QueryResult<String> = QueryResult {
            data: vec!["a".to_string(), "b".to_string()],
            total: 2,
            page: 1,
            page_size: 10,
        };

        assert_eq!(result.data.len(), 2);
        assert_eq!(result.total, 2);
        assert_eq!(result.page, 1);
        assert_eq!(result.page_size, 10);
    }

    #[test]
    fn test_database_error_display() {
        let err = DatabaseError::Connection("test".to_string());
        assert!(err.to_string().contains("Connection"));

        let err = DatabaseError::Query("test".to_string());
        assert!(err.to_string().contains("Query"));

        let err = DatabaseError::NotFound("test".to_string());
        assert!(err.to_string().contains("not found"));
    }
}
