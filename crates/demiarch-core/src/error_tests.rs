//! Error module tests

use crate::error::{Error, Result};

#[tokio::test]
async fn test_feature_not_found_error() {
    let error = Error::FeatureNotFound("test-feature".to_string());
    assert_eq!(error.code(), "E001");
    assert_eq!(
        error.suggestion(),
        Some("demiarch features list".to_string())
    );
    assert!(error.to_string().contains("test-feature"));
}

#[tokio::test]
async fn test_project_not_found_error() {
    let error = Error::ProjectNotFound("my-project".to_string());
    assert_eq!(error.code(), "E002");
    assert_eq!(
        error.suggestion(),
        Some("demiarch projects list".to_string())
    );
    assert!(error.to_string().contains("my-project"));
}

#[tokio::test]
async fn test_phase_not_found_error() {
    let error = Error::PhaseNotFound("planning".to_string());
    assert_eq!(error.code(), "E003");
    assert_eq!(error.suggestion(), None);
    assert!(error.to_string().contains("planning"));
}

#[tokio::test]
async fn test_llm_error() {
    let error = Error::LLMError("Model not found".to_string());
    assert_eq!(error.code(), "E101");
    assert_eq!(
        error.suggestion(),
        Some("demiarch config get openrouter_api_key".to_string())
    );
}

#[tokio::test]
async fn test_rate_limited_error() {
    let error = Error::RateLimited(30);
    assert_eq!(error.code(), "E103");
    assert_eq!(error.suggestion(), None);
    assert!(error.to_string().contains("30"));
}

#[tokio::test]
async fn test_budget_exceeded_error() {
    let error = Error::BudgetExceeded(15.0, 10.0, 20.0);
    assert_eq!(error.code(), "E200");
    assert_eq!(
        error.suggestion(),
        Some("demiarch config set cost_daily_limit_usd 20".to_string())
    );
    assert!(error.to_string().contains("15.00"));
    assert!(error.to_string().contains("10.00"));
}

#[tokio::test]
async fn test_lock_timeout_error() {
    let error = Error::LockTimeout("file.txt".to_string());
    assert_eq!(error.code(), "E300");
    assert_eq!(error.suggestion(), None);
    assert!(error.to_string().contains("file.txt"));
}

#[tokio::test]
async fn test_plugin_not_found_error() {
    let error = Error::PluginNotFound("nextjs-plugin".to_string());
    assert_eq!(error.code(), "E500");
    assert_eq!(
        error.suggestion(),
        Some("demiarch plugin install nextjs-plugin".to_string())
    );
    assert!(error.to_string().contains("nextjs-plugin"));
}

#[tokio::test]
async fn test_plugin_validation_failed_error() {
    let error = Error::PluginValidationFailed("Invalid manifest".to_string());
    assert_eq!(error.code(), "E501");
    assert_eq!(error.suggestion(), None);
}

#[tokio::test]
async fn test_license_expired_error() {
    let error = Error::LicenseExpired("premium-plugin".to_string(), "2024-01-01".to_string());
    assert_eq!(error.code(), "E502");
    assert_eq!(error.suggestion(), None);
    assert!(error.to_string().contains("premium-plugin"));
}

#[tokio::test]
async fn test_config_error() {
    let error = Error::ConfigError("Missing API key".to_string());
    assert_eq!(error.code(), "E600");
    assert_eq!(error.suggestion(), None);
}

#[tokio::test]
async fn test_user_cancelled_error() {
    let error = Error::UserCancelled;
    assert_eq!(error.code(), "E700");
    assert_eq!(error.suggestion(), None);
}

#[tokio::test]
async fn test_invalid_input_error() {
    let error = Error::InvalidInput("Empty name".to_string());
    assert_eq!(error.code(), "E800");
    assert_eq!(error.suggestion(), None);
    assert!(error.to_string().contains("Empty name"));
}

#[tokio::test]
async fn test_skill_not_found_error() {
    let error = Error::SkillNotFound("debug-react".to_string());
    assert_eq!(error.code(), "E900");
    assert_eq!(error.suggestion(), Some("demiarch skills list".to_string()));
    assert!(error.to_string().contains("debug-react"));
}

#[tokio::test]
async fn test_skill_extraction_failed_error() {
    let error = Error::SkillExtractionFailed("Empty solution".to_string());
    assert_eq!(error.code(), "E901");
    assert_eq!(error.suggestion(), None);
}

#[tokio::test]
async fn test_hook_failed_error() {
    let error = Error::HookFailed("pre-generation hook".to_string());
    assert_eq!(error.code(), "E1000");
    assert_eq!(error.suggestion(), None);
}

#[tokio::test]
async fn test_hook_timeout_error() {
    let error = Error::HookTimeout(60);
    assert_eq!(error.code(), "E1001");
    assert_eq!(error.suggestion(), None);
    assert!(error.to_string().contains("60"));
}

#[tokio::test]
async fn test_routing_failed_error() {
    let error = Error::RoutingFailed("No models available".to_string());
    assert_eq!(error.code(), "E1100");
    assert_eq!(error.suggestion(), None);
}

#[tokio::test]
async fn test_no_suitable_model_error() {
    let error = Error::NoSuitableModel("math".to_string());
    assert_eq!(error.code(), "E1101");
    assert_eq!(error.suggestion(), None);
    assert!(error.to_string().contains("math"));
}

#[tokio::test]
async fn test_context_retrieval_failed_error() {
    let error = Error::ContextRetrievalFailed("Index corrupted".to_string());
    assert_eq!(error.code(), "E1200");
    assert_eq!(
        error.suggestion(),
        Some("demiarch context rebuild".to_string())
    );
}

#[tokio::test]
async fn test_embedding_failed_error() {
    let error = Error::EmbeddingFailed("API error".to_string());
    assert_eq!(error.code(), "E1201");
    assert_eq!(error.suggestion(), None);
}

#[tokio::test]
async fn test_other_error() {
    let error = Error::Other("Unknown error".to_string());
    assert_eq!(error.code(), "E9999");
    assert_eq!(error.suggestion(), None);
}

#[tokio::test]
async fn test_io_error() {
    let error = Error::from(std::io::Error::new(
        std::io::ErrorKind::NotFound,
        "file not found",
    ));
    assert_eq!(error.code(), "E9999");
}

#[tokio::test]
async fn test_network_error() {
    let error = Error::LLMError("Network timeout".to_string());
    assert_eq!(error.code(), "E101");
    assert_eq!(
        error.suggestion(),
        Some("demiarch config get openrouter_api_key".to_string())
    );
}

#[tokio::test]
async fn test_database_error() {
    let error = Error::DatabaseError(sqlx::Error::RowNotFound);
    assert_eq!(error.code(), "E400");
}

#[tokio::test]
async fn test_phase_not_found_error_suggestion() {
    let error = Error::PhaseNotFound("planning".to_string());
    assert_eq!(error.code(), "E003");
    assert_eq!(error.suggestion(), None);
}

#[tokio::test]
async fn test_lock_timeout_error_suggestion() {
    let error = Error::LockTimeout("file.txt".to_string());
    assert_eq!(error.code(), "E300");
    assert_eq!(error.suggestion(), None);
}

#[tokio::test]
async fn test_plugin_validation_failed_error_suggestion() {
    let error = Error::PluginValidationFailed("Invalid manifest".to_string());
    assert_eq!(error.code(), "E501");
    assert_eq!(error.suggestion(), None);
}

#[tokio::test]
async fn test_config_error_suggestion() {
    let error = Error::ConfigError("Missing API key".to_string());
    assert_eq!(error.code(), "E600");
    assert_eq!(error.suggestion(), None);
}

#[tokio::test]
async fn test_invalid_input_error_suggestion() {
    let error = Error::InvalidInput("Empty name".to_string());
    assert_eq!(error.code(), "E800");
    assert_eq!(error.suggestion(), None);
}

#[tokio::test]
async fn test_skill_extraction_failed_error_suggestion() {
    let error = Error::SkillExtractionFailed("Empty solution".to_string());
    assert_eq!(error.code(), "E901");
    assert_eq!(error.suggestion(), None);
}

#[tokio::test]
async fn test_hook_failed_error_suggestion() {
    let error = Error::HookFailed("pre-generation hook".to_string());
    assert_eq!(error.code(), "E1000");
    assert_eq!(error.suggestion(), None);
}

#[tokio::test]
async fn test_hook_timeout_error_suggestion() {
    let error = Error::HookTimeout(60);
    assert_eq!(error.code(), "E1001");
    assert_eq!(error.suggestion(), None);
}

#[tokio::test]
async fn test_routing_failed_error_suggestion() {
    let error = Error::RoutingFailed("No models available".to_string());
    assert_eq!(error.code(), "E1100");
    assert_eq!(error.suggestion(), None);
}

#[tokio::test]
async fn test_no_suitable_model_error_suggestion() {
    let error = Error::NoSuitableModel("math".to_string());
    assert_eq!(error.code(), "E1101");
    assert_eq!(error.suggestion(), None);
}

#[tokio::test]
async fn test_embedding_failed_error_suggestion() {
    let error = Error::EmbeddingFailed("API error".to_string());
    assert_eq!(error.code(), "E1201");
    assert_eq!(error.suggestion(), None);
}

#[tokio::test]
async fn test_other_error_suggestion() {
    let error = Error::Other("Unknown error".to_string());
    assert_eq!(error.code(), "E9999");
    assert_eq!(error.suggestion(), None);
}

#[tokio::test]
async fn test_result_type_alias() {
    let success: Result<i32> = Ok(42);
    assert!(success.is_ok());
    assert_eq!(success.ok(), Some(42));

    let failure: Result<i32> = Err(Error::ProjectNotFound("test".to_string()));
    assert!(failure.is_err());
}

#[tokio::test]
async fn test_all_error_codes_unique() {
    let errors = vec![
        Error::FeatureNotFound("test".to_string()).code(),
        Error::ProjectNotFound("test".to_string()).code(),
        Error::PhaseNotFound("test".to_string()).code(),
        Error::LLMError("test".to_string()).code(),
        Error::RateLimited(30).code(),
        Error::BudgetExceeded(10.0, 15.0, 20.0).code(),
        Error::LockTimeout("test".to_string()).code(),
        Error::DatabaseError(sqlx::Error::RowNotFound).code(),
        Error::PluginNotFound("test".to_string()).code(),
        Error::PluginValidationFailed("test".to_string()).code(),
        Error::LicenseExpired("test".to_string(), "2024".to_string()).code(),
        Error::ConfigError("test".to_string()).code(),
        Error::UserCancelled.code(),
        Error::InvalidInput("test".to_string()).code(),
        Error::SkillNotFound("test".to_string()).code(),
        Error::SkillExtractionFailed("test".to_string()).code(),
        Error::HookFailed("test".to_string()).code(),
        Error::HookTimeout(60).code(),
        Error::RoutingFailed("test".to_string()).code(),
        Error::NoSuitableModel("test".to_string()).code(),
        Error::ContextRetrievalFailed("test".to_string()).code(),
        Error::EmbeddingFailed("test".to_string()).code(),
        Error::Other("test".to_string()).code(),
    ];

    let unique_codes: std::collections::HashSet<_> = errors.into_iter().collect();
    assert_eq!(unique_codes.len(), 23);
}
