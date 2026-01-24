//! Commands module tests

use crate::commands::{
    chat::{self, ChatMessage, LegacyChatMessage, MessageRole},
    feature,
    generate::{self, GenerationResult},
    project,
    sync::{self, SyncStatus},
};
use crate::Result;

#[tokio::test]
async fn test_chat_message_structure() {
    // Test new ChatMessage with proper structure
    let message = ChatMessage::user("conv-123", "Hello");

    assert!(!message.id.is_empty());
    assert_eq!(message.conversation_id, "conv-123");
    assert_eq!(message.role, MessageRole::User);
    assert_eq!(message.content, "Hello");
}

#[tokio::test]
async fn test_chat_message_clone() {
    let message = ChatMessage::assistant("conv-456", "Hi there!");

    let cloned = message.clone();
    assert_eq!(message.id, cloned.id);
    assert_eq!(message.role, cloned.role);
    assert_eq!(message.conversation_id, cloned.conversation_id);
}

#[tokio::test]
async fn test_legacy_chat_message() {
    // Test the legacy ChatMessage format for backwards compatibility
    let legacy = LegacyChatMessage {
        id: "test-123".to_string(),
        role: "user".to_string(),
        content: "Hello".to_string(),
        created_at: "2024-01-01T00:00:00Z".to_string(),
    };

    assert_eq!(legacy.id, "test-123");
    assert_eq!(legacy.role, "user");
    assert_eq!(legacy.content, "Hello");
}

#[tokio::test]
async fn test_chat_send_returns_result() {
    let result: Result<String> = chat::send("project-id", "Hello").await;
    assert!(result.is_ok());
    assert!(result.unwrap().contains("Chat message received"));
}

#[tokio::test]
async fn test_chat_history_returns_result() {
    let result: Result<Vec<LegacyChatMessage>> = chat::history("project-id", 10).await;
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
        files: vec![],
    };

    assert_eq!(result.files_created, 5);
    assert_eq!(result.files_modified, 3);
    assert_eq!(result.tokens_used, 1500);
    assert_eq!(result.cost_usd, 0.05);
    assert!(result.files.is_empty());
}

#[tokio::test]
async fn test_generate_result_clone() {
    let result = GenerationResult {
        files_created: 1,
        files_modified: 2,
        tokens_used: 500,
        cost_usd: 0.02,
        files: vec![],
    };

    let cloned = result.clone();
    assert_eq!(result.files_created, cloned.files_created);
    assert_eq!(result.files.len(), cloned.files.len());
}

#[tokio::test]
async fn test_generate_requires_api_key() {
    // Generate now requires an API key, so it should return an error without one
    // (unless DEMIARCH_API_KEY or OPENROUTER_API_KEY is set)
    let result: Result<GenerationResult> = generate::generate("test description", true).await;
    // Without an API key, this should fail with an LLM error
    // With an API key, it would succeed
    // We just verify it returns a result either way
    match result {
        Ok(gen_result) => {
            // API key was set, verify structure
            assert!(gen_result.tokens_used > 0 || gen_result.files.is_empty());
        }
        Err(e) => {
            // No API key, should be an LLM error
            let error_str = format!("{}", e);
            assert!(
                error_str.contains("API key") || error_str.contains("LLM"),
                "Expected API key error, got: {}",
                error_str
            );
        }
    }
}

#[tokio::test]
async fn test_project_create_returns_result() {
    let result: Result<String> =
        project::create("my-project", "nextjs", "https://github.com/user/repo").await;
    assert!(result.is_ok());
    // The create function now returns a valid UUID instead of a placeholder
    let id = result.unwrap();
    assert!(
        uuid::Uuid::parse_str(&id).is_ok(),
        "Should return a valid UUID"
    );
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
    // Invalid UUID format returns None
    assert!(result.unwrap().is_none());
}

#[tokio::test]
async fn test_project_get_with_valid_uuid() {
    // Valid UUID should return the ID
    let valid_uuid = "550e8400-e29b-41d4-a716-446655440000";
    let result: Result<Option<String>> = project::get(valid_uuid).await;
    assert!(result.is_ok());
    assert_eq!(result.unwrap(), Some(valid_uuid.to_string()));
}

#[tokio::test]
async fn test_project_archive_returns_result() {
    // Archive now requires a valid UUID format
    let valid_uuid = "550e8400-e29b-41d4-a716-446655440000";
    let result: Result<()> = project::archive(valid_uuid).await;
    assert!(result.is_ok());
}

#[tokio::test]
async fn test_project_archive_invalid_uuid() {
    // Invalid UUID should return an error
    let result: Result<()> = project::archive("invalid-id").await;
    assert!(result.is_err());
}

#[tokio::test]
async fn test_project_delete_returns_result() {
    // Delete now requires a valid UUID format
    let valid_uuid = "550e8400-e29b-41d4-a716-446655440000";
    let result: Result<()> = project::delete(valid_uuid, false).await;
    assert!(result.is_ok());
}

#[tokio::test]
async fn test_project_delete_invalid_uuid() {
    // Invalid UUID should return an error
    let result: Result<()> = project::delete("invalid-id", false).await;
    assert!(result.is_err());
}

#[tokio::test]
async fn test_sync_flush_returns_result() {
    use crate::storage::Database;
    use tempfile::TempDir;

    let db = Database::in_memory()
        .await
        .expect("Failed to create test database");
    let temp_dir = TempDir::new().expect("Failed to create temp directory");

    let result = sync::flush(db.pool(), temp_dir.path()).await;
    assert!(result.is_ok());
    let export_result = result.unwrap();
    assert!(export_result.sync_dir.exists());
}

#[tokio::test]
async fn test_sync_import_returns_result() {
    use crate::storage::Database;
    use tempfile::TempDir;

    let db = Database::in_memory()
        .await
        .expect("Failed to create test database");
    let temp_dir = TempDir::new().expect("Failed to create temp directory");

    // First export to create the sync directory
    sync::flush(db.pool(), temp_dir.path())
        .await
        .expect("Export should succeed");

    // Then import
    let result = sync::import(db.pool(), temp_dir.path()).await;
    assert!(result.is_ok());
}

#[tokio::test]
async fn test_sync_status_structure() {
    let status = SyncStatus {
        dirty: false,
        last_sync_at: Some("2024-01-01T00:00:00Z".to_string()),
        pending_changes: 10,
        message: "Test status".to_string(),
    };

    assert_eq!(status.pending_changes, 10);
    assert!(status.last_sync_at.is_some());
    assert!(!status.last_sync_at.clone().unwrap().is_empty());
    assert!(!status.message.is_empty());
}

#[tokio::test]
async fn test_sync_status_returns_result() {
    use crate::storage::Database;
    use tempfile::TempDir;

    let db = Database::in_memory()
        .await
        .expect("Failed to create test database");
    let temp_dir = TempDir::new().expect("Failed to create temp directory");

    // Before any export, status should show dirty
    let result = sync::status(db.pool(), temp_dir.path()).await;
    assert!(result.is_ok());
    let status = result.unwrap();
    assert!(status.dirty); // No previous export, so dirty

    // After export, should be clean
    sync::flush(db.pool(), temp_dir.path())
        .await
        .expect("Export should succeed");

    let status_after = sync::status(db.pool(), temp_dir.path())
        .await
        .expect("Status check should succeed");
    assert!(!status_after.dirty);
    assert_eq!(status_after.pending_changes, 0);
}
