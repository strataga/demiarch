//! Commands module tests

use crate::Result;
use crate::commands::{
    chat::{self, ChatMessage},
    feature,
    generate::{self, GenerationResult},
    project,
    sync::{self, SyncStatus},
};

#[tokio::test]
async fn test_chat_message_structure() {
    let message = ChatMessage {
        id: "test-123".to_string(),
        role: "user".to_string(),
        content: "Hello".to_string(),
        created_at: "2024-01-01T00:00:00Z".to_string(),
    };

    assert_eq!(message.id, "test-123");
    assert_eq!(message.role, "user");
    assert_eq!(message.content, "Hello");
    assert!(!message.id.is_empty());
}

#[tokio::test]
async fn test_chat_message_clone() {
    let message = ChatMessage {
        id: "test-456".to_string(),
        role: "assistant".to_string(),
        content: "Hi there!".to_string(),
        created_at: "2024-01-01T00:00:00Z".to_string(),
    };

    let cloned = message.clone();
    assert_eq!(message.id, cloned.id);
    assert_eq!(message.role, cloned.role);
}

#[tokio::test]
async fn test_chat_send_returns_result() {
    let result: Result<String> = chat::send("project-id", "Hello").await;
    assert!(result.is_ok());
    assert_eq!(
        result.unwrap(),
        "Chat message received. (Not yet implemented)"
    );
}

#[tokio::test]
async fn test_chat_history_returns_result() {
    let result: Result<Vec<ChatMessage>> = chat::history("project-id", 10).await;
    assert!(result.is_ok());
    assert!(result.unwrap().is_empty());
}

#[tokio::test]
async fn test_feature_create_returns_result() {
    let result: Result<String> = feature::create("project-id", "Test Feature", None).await;
    assert!(result.is_ok());
    assert_eq!(result.unwrap(), "feature-placeholder-id");
}

#[tokio::test]
async fn test_feature_list_returns_result() {
    let result: Result<Vec<String>> = feature::list("project-id", None).await;
    assert!(result.is_ok());
    assert!(result.unwrap().is_empty());
}

#[tokio::test]
async fn test_feature_update_returns_result() {
    let result: Result<()> = feature::update("feature-id", Some("done"), None).await;
    assert!(result.is_ok());
}

#[tokio::test]
async fn test_feature_delete_returns_result() {
    let result: Result<()> = feature::delete("feature-id").await;
    assert!(result.is_ok());
}

#[tokio::test]
async fn test_generate_result_structure() {
    let result = GenerationResult {
        files_created: 5,
        files_modified: 3,
        tokens_used: 1500,
        cost_usd: 0.05,
    };

    assert_eq!(result.files_created, 5);
    assert_eq!(result.files_modified, 3);
    assert_eq!(result.tokens_used, 1500);
    assert_eq!(result.cost_usd, 0.05);
}

#[tokio::test]
async fn test_generate_result_clone() {
    let result = GenerationResult {
        files_created: 1,
        files_modified: 2,
        tokens_used: 500,
        cost_usd: 0.02,
    };

    let cloned = result.clone();
    assert_eq!(result.files_created, cloned.files_created);
}

#[tokio::test]
async fn test_generate_returns_result() {
    let result: Result<GenerationResult> = generate::generate("feature-id", false).await;
    assert!(result.is_ok());
    let gen_result = result.unwrap();
    assert_eq!(gen_result.files_created, 0);
    assert_eq!(gen_result.files_modified, 0);
}

#[tokio::test]
async fn test_project_create_returns_result() {
    let result: Result<String> =
        project::create("my-project", "nextjs", "https://github.com/user/repo").await;
    assert!(result.is_ok());
    assert_eq!(result.unwrap(), "project-placeholder-id");
}

#[tokio::test]
async fn test_project_list_returns_result() {
    let result: Result<Vec<String>> = project::list().await;
    assert!(result.is_ok());
    assert!(result.unwrap().is_empty());
}

#[tokio::test]
async fn test_project_get_returns_result() {
    let result: Result<Option<String>> = project::get("project-id").await;
    assert!(result.is_ok());
    assert!(result.unwrap().is_none());
}

#[tokio::test]
async fn test_project_archive_returns_result() {
    let result: Result<()> = project::archive("project-id").await;
    assert!(result.is_ok());
}

#[tokio::test]
async fn test_project_delete_returns_result() {
    let result: Result<()> = project::delete("project-id", false).await;
    assert!(result.is_ok());
}

#[tokio::test]
async fn test_sync_flush_returns_result() {
    let result: Result<()> = sync::flush().await;
    assert!(result.is_ok());
}

#[tokio::test]
async fn test_sync_import_returns_result() {
    let result: Result<()> = sync::import().await;
    assert!(result.is_ok());
}

#[tokio::test]
async fn test_sync_status_structure() {
    let status = SyncStatus {
        dirty: false,
        last_sync_at: Some("2024-01-01T00:00:00Z".to_string()),
        pending_changes: 10,
    };

    assert_eq!(status.pending_changes, 10);
    assert!(status.last_sync_at.is_some());
    assert!(!status.last_sync_at.clone().unwrap().is_empty());
}

#[tokio::test]
async fn test_sync_status_returns_result() {
    let result: Result<SyncStatus> = sync::status().await;
    assert!(result.is_ok());
    let status = result.unwrap();
    assert_eq!(status.pending_changes, 0);
    assert!(!status.dirty);
}
