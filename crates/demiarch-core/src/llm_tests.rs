//! LLM module tests

use std::sync::Arc;

use crate::config::LlmConfig;
use crate::cost::CostTracker;
use crate::llm::{LlmClient, Message, MessageRole};

fn test_config() -> LlmConfig {
    LlmConfig {
        api_key: None,
        default_model: "anthropic/claude-sonnet-4-20250514".to_string(),
        fallback_models: vec![
            "anthropic/claude-3-5-haiku-latest".to_string(),
            "openai/gpt-4o".to_string(),
        ],
        temperature: 0.7,
        max_tokens: 8192,
        timeout_secs: 120,
    }
}

#[test]
fn test_llm_client_new() {
    let client = LlmClient::new(test_config(), "test-api-key").unwrap();
    assert!(format!("{:?}", client).contains("LlmClient"));
}

#[test]
fn test_llm_client_clone() {
    let client = LlmClient::new(test_config(), "test-api-key").unwrap();
    let cloned = client.clone();
    assert_eq!(format!("{:?}", client), format!("{:?}", cloned));
}

#[test]
fn test_llm_client_debug() {
    let client = LlmClient::new(test_config(), "test-api-key").unwrap();
    let debug = format!("{:?}", client);
    assert!(debug.contains("LlmClient"));
    assert!(debug.contains("claude-sonnet"));
}

#[test]
fn test_llm_client_default() {
    let client = LlmClient::new(test_config(), "test-api-key").unwrap();
    assert_eq!(client.default_model(), "anthropic/claude-sonnet-4-20250514");
}

#[test]
fn test_llm_client_send_sync() {
    fn assert_send_sync<T: Send + Sync>() {}
    assert_send_sync::<LlmClient>();
}

#[test]
fn test_llm_client_builder() {
    let tracker = Arc::new(CostTracker::new(10.0, 0.8));

    let client = LlmClient::builder()
        .config(test_config())
        .api_key("test-key")
        .cost_tracker(tracker.clone())
        .timeout_secs(60)
        .build()
        .unwrap();

    assert_eq!(client.default_model(), "anthropic/claude-sonnet-4-20250514");
}

#[test]
fn test_llm_client_builder_missing_key() {
    let result = LlmClient::builder().config(test_config()).build();
    assert!(result.is_err());
}

#[test]
fn test_message_creation() {
    let system = Message::system("You are helpful");
    assert_eq!(system.role, MessageRole::System);

    let user = Message::user("Hello");
    assert_eq!(user.role, MessageRole::User);

    let assistant = Message::assistant("Hi there");
    assert_eq!(assistant.role, MessageRole::Assistant);
}

#[test]
fn test_token_estimation() {
    let client = LlmClient::new(test_config(), "test-api-key").unwrap();
    let messages = vec![
        Message::system("You are a helpful assistant"),
        Message::user("What is 2+2?"),
    ];

    let estimate = client.estimate_tokens(&messages);
    // Should be non-zero
    assert!(estimate > 0);
    // Should be reasonable (not too high)
    assert!(estimate < 100);
}

#[tokio::test]
async fn test_llm_request() {
    let client = LlmClient::new(test_config(), "test-api-key").unwrap();
    // Just verify client can be used in async context
    let _model = client.default_model();
}

#[tokio::test]
async fn test_model_fallback() {
    let client = LlmClient::new(test_config(), "test-api-key").unwrap();
    assert_eq!(
        client.fallback_models(),
        &["anthropic/claude-3-5-haiku-latest", "openai/gpt-4o"]
    );
}

#[tokio::test]
async fn test_cost_tracking_integration() {
    let tracker = Arc::new(CostTracker::new(10.0, 0.8));
    let client = LlmClient::new(test_config(), "test-api-key")
        .unwrap()
        .with_cost_tracker(tracker.clone());

    // Verify cost tracker is attached
    assert!(format!("{:?}", client).contains("cost_tracker: true"));

    // Verify tracker starts at zero
    assert_eq!(tracker.today_total(), 0.0);
}

#[tokio::test]
async fn test_timeout_configuration() {
    let mut config = test_config();
    config.timeout_secs = 30;

    let client = LlmClient::new(config, "test-api-key").unwrap();
    // Client should be created with custom timeout
    assert!(format!("{:?}", client).contains("LlmClient"));
}

// Integration tests that require actual API access are marked with feature flag
#[cfg(feature = "integration-tests")]
mod integration {
    use super::*;
    use crate::config::Config;

    #[tokio::test]
    async fn test_actual_api_call() {
        let config = Config::load().unwrap();
        let api_key = config
            .llm
            .resolved_api_key()
            .unwrap()
            .expect("API key required for integration tests");

        let tracker = Arc::new(CostTracker::from_config(&config.cost));
        let client = LlmClient::new(config.llm, api_key)
            .unwrap()
            .with_cost_tracker(tracker.clone());

        let messages = vec![Message::user("Say 'Hello' and nothing else.")];

        let response = client.complete(messages, None).await.unwrap();

        assert!(response.content.to_lowercase().contains("hello"));
        assert!(response.tokens_used > 0);
        assert!(tracker.today_total() > 0.0);
    }

    #[tokio::test]
    async fn test_streaming_api_call() {
        use futures_util::StreamExt;

        let config = Config::load().unwrap();
        let api_key = config
            .llm
            .resolved_api_key()
            .unwrap()
            .expect("API key required for integration tests");

        let client = LlmClient::new(config.llm, api_key).unwrap();

        let messages = vec![Message::user("Count from 1 to 3.")];

        let mut stream = client.complete_streaming(messages, None).await.unwrap();
        let mut content = String::new();

        while let Some(event) = stream.next().await {
            match event.unwrap() {
                crate::llm::StreamEvent::Chunk(chunk) => {
                    if let Some(text) = chunk.content() {
                        content.push_str(text);
                    }
                }
                crate::llm::StreamEvent::Done => break,
                crate::llm::StreamEvent::Error(e) => panic!("Stream error: {}", e),
            }
        }

        assert!(content.contains('1'));
        assert!(content.contains('2'));
        assert!(content.contains('3'));
    }
}
