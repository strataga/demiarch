//! Demiarch Core Integration Tests

use demiarch_core::{
    Error, Result,
    agents::{AgentType, CoderAgent, OrchestratorAgent, PlannerAgent, ReviewerAgent, TesterAgent},
    commands::chat,
    commands::feature,
    commands::generate,
    commands::project,
    commands::sync::{self, SyncStatus},
};

#[tokio::test]
async fn test_full_agent_workflow() {
    let orchestrator = OrchestratorAgent::new();
    let planner = PlannerAgent::new();
    let coder = CoderAgent::new();
    let reviewer = ReviewerAgent::new();
    let tester = TesterAgent::new();

    let request = "Create a user authentication system";
    let orchestrated = orchestrator.process(request).await;

    let tasks = planner.plan(&orchestrated).await;
    assert!(!tasks.is_empty());

    for task in tasks {
        let code = coder.generate(&task).await;
        let review = reviewer.review(&code).await;
        let test = tester.test(&code).await;

        assert!(!code.is_empty());
        assert!(!review.is_empty());
        assert!(!test.is_empty());
    }
}

#[tokio::test]
async fn test_project_and_feature_workflow() {
    let project_id = "test-project-1".to_string();
    let _create_result =
        project::create("test-project", "nextjs", "https://github.com/test/repo").await;

    let list_result = project::list().await;
    assert!(list_result.is_ok());

    let get_result = project::get(&project_id).await;
    assert!(get_result.is_ok());

    let feature_id = "test-feature-1";
    let _create_feature = feature::create(&project_id, "Add login", None).await;

    let features = feature::list(&project_id, None).await;
    assert!(features.is_ok());

    let _update_feature = feature::update(feature_id, Some("in-progress"), None).await;
    let _delete_feature = feature::delete(feature_id).await;
}

#[tokio::test]
async fn test_generate_workflow() {
    let feature_id = "test-feature-2";
    let generate_result = generate::generate(feature_id, true).await;
    assert!(generate_result.is_ok());

    let result = generate_result.unwrap();
    assert_eq!(result.files_created, 0);
    assert_eq!(result.files_modified, 0);
}

#[tokio::test]
async fn test_sync_workflow() {
    let _flush_result = sync::flush().await;
    let _import_result = sync::import().await;

    let status = sync::status().await;
    assert!(status.is_ok());

    let sync_status = status.unwrap();
    assert!(!sync_status.dirty);
    assert_eq!(sync_status.pending_changes, 0);
}

#[tokio::test]
async fn test_chat_workflow() {
    let project_id = "test-project-chat";
    let message = "How do I add user authentication?";

    let send_result = chat::send(project_id, message).await;
    assert!(send_result.is_ok());

    let history = chat::history(project_id, 10).await;
    assert!(history.is_ok());

    let messages = history.unwrap();
    assert!(messages.is_empty());
}

#[test]
fn test_error_codes() {
    let errors = [
        Error::FeatureNotFound("test".to_string()),
        Error::ProjectNotFound("test".to_string()),
        Error::PhaseNotFound("test".to_string()),
        Error::LLMError("test".to_string()),
        Error::RateLimited(30),
        Error::BudgetExceeded(10.0, 15.0, 20.0),
        Error::LockTimeout("test".to_string()),
        Error::PluginNotFound("test".to_string()),
        Error::PluginValidationFailed("test".to_string()),
        Error::LicenseExpired("test".to_string(), "2024-01-01".to_string()),
        Error::ConfigError("test".to_string()),
        Error::InvalidInput("test".to_string()),
        Error::SkillNotFound("test".to_string()),
        Error::SkillExtractionFailed("test".to_string()),
        Error::HookFailed("test".to_string()),
        Error::HookTimeout(60),
        Error::RoutingFailed("test".to_string()),
        Error::NoSuitableModel("test".to_string()),
        Error::ContextRetrievalFailed("test".to_string()),
        Error::EmbeddingFailed("test".to_string()),
    ];

    for error in errors.iter() {
        let code = error.code();
        assert!(!code.is_empty());
    }
}

#[test]
fn test_result_types() {
    let ok_result: Result<i32> = Ok(42);
    let err_result: Result<i32> = Err(Error::ProjectNotFound("test".to_string()));

    assert!(ok_result.is_ok());
    assert!(err_result.is_err());
}

#[test]
fn test_agent_types_all_covered() {
    let types = [
        AgentType::Orchestrator,
        AgentType::Planner,
        AgentType::Coder,
        AgentType::Reviewer,
        AgentType::Tester,
    ];

    for agent_type in types.iter() {
        let string = agent_type.to_string();
        assert!(!string.is_empty());
    }
}

#[tokio::test]
async fn test_multiple_projects() {
    let project_ids = vec!["proj-1", "proj-2", "proj-3"];

    for id in project_ids.iter() {
        let _create = project::create(id, "nextjs", "").await;
    }

    let list = project::list().await;
    assert!(list.is_ok());
}

#[tokio::test]
async fn test_multiple_features() {
    let project_id = "multi-feature-project";
    let features = vec!["Feature 1", "Feature 2", "Feature 3"];

    for feature_name in features.iter() {
        let _create = feature::create(project_id, feature_name, None).await;
    }

    let list = feature::list(project_id, None).await;
    assert!(list.is_ok());
}

#[tokio::test]
async fn test_concurrent_operations() {
    let project_id = "concurrent-project";

    let create_proj = tokio::spawn(async move { project::create(project_id, "nextjs", "").await });

    let create_feat1 =
        tokio::spawn(async move { feature::create(project_id, "Feature 1", None).await });

    let create_feat2 =
        tokio::spawn(async move { feature::create(project_id, "Feature 2", None).await });

    let (r1, r2, r3) = tokio::join!(create_proj, create_feat1, create_feat2);

    assert!(r1.unwrap().is_ok());
    assert!(r2.unwrap().is_ok());
    assert!(r3.unwrap().is_ok());
}

#[test]
fn test_error_display() {
    let error = Error::FeatureNotFound("test-feature".to_string());
    let display = format!("{}", error);
    assert!(display.contains("test-feature"));
}

#[test]
fn test_error_debug() {
    let error = Error::ProjectNotFound("test-project".to_string());
    let debug = format!("{:?}", error);
    assert!(debug.contains("ProjectNotFound"));
}

#[test]
fn test_sync_status_clone() {
    let status = SyncStatus {
        dirty: false,
        last_sync_at: Some("2024-01-01".to_string()),
        pending_changes: 5,
    };

    let cloned = status.clone();
    assert_eq!(status.dirty, cloned.dirty);
    assert_eq!(status.pending_changes, cloned.pending_changes);
}

#[tokio::test]
async fn test_all_agent_async_methods() {
    let orchestrator = OrchestratorAgent::new();
    let planner = PlannerAgent::new();
    let coder = CoderAgent::new();
    let reviewer = ReviewerAgent::new();
    let tester = TesterAgent::new();

    let o = orchestrator.process("test").await;
    let p = planner.plan("test").await;
    let c = coder.generate("test").await;
    let r = reviewer.review("test").await;
    let t = tester.test("test").await;

    assert!(!o.is_empty());
    assert!(!p.is_empty());
    assert!(!c.is_empty());
    assert!(!r.is_empty());
    assert!(!t.is_empty());
}
