//! Config module tests

use crate::config::{Config, CostConfig, LlmConfig, RoutingConfig};

#[test]
fn test_llm_config_fallback_models() {
    let config = Config::default();
    assert!(
        config
            .llm
            .fallback_models
            .contains(&"anthropic/claude-3-5-haiku-latest".to_string())
    );
    assert!(
        config
            .llm
            .fallback_models
            .contains(&"openai/gpt-4o".to_string())
    );
    assert_eq!(config.llm.fallback_models.len(), 2);
}

#[test]
fn test_cost_config_values() {
    let cost = CostConfig {
        daily_limit_usd: 50.0,
        alert_threshold: 0.9,
    };

    assert_eq!(cost.daily_limit_usd, 50.0);
    assert_eq!(cost.alert_threshold, 0.9);
}

#[test]
fn test_routing_config_preference() {
    let routing = RoutingConfig {
        preference: "performance".to_string(),
    };

    assert_eq!(routing.preference, "performance");
}

#[test]
fn test_config_default() {
    let config = Config::default();

    // LLM config defaults
    assert!(config.llm.api_key.is_none());
    assert_eq!(
        config.llm.default_model,
        "anthropic/claude-sonnet-4-20250514"
    );
    assert_eq!(config.llm.fallback_models.len(), 2);
    assert_eq!(config.llm.temperature, 0.7);
    assert_eq!(config.llm.max_tokens, 8192);
    assert_eq!(config.llm.timeout_secs, 120);

    // Cost config defaults
    assert_eq!(config.cost.daily_limit_usd, 10.0);
    assert_eq!(config.cost.alert_threshold, 0.8);

    // Routing config defaults
    assert_eq!(config.routing.preference, "balanced");
}

#[test]
fn test_config_serialize_deserialize() {
    use serde::{Deserialize, Serialize};

    #[derive(Debug, Clone, Serialize, Deserialize)]
    struct Wrapper {
        config: Config,
    }

    let config = Config::default();
    let wrapper = Wrapper {
        config: config.clone(),
    };

    let serialized = serde_json::to_string(&wrapper).expect("Should serialize");
    let deserialized: Wrapper = serde_json::from_str(&serialized).expect("Should deserialize");

    assert_eq!(
        deserialized.config.llm.default_model,
        config.llm.default_model
    );
    assert_eq!(
        deserialized.config.cost.daily_limit_usd,
        config.cost.daily_limit_usd
    );
}

#[test]
fn test_config_with_api_key() {
    let mut config = Config::default();
    config.llm.api_key = Some("sk-test-key".to_string());

    assert_eq!(config.llm.api_key, Some("sk-test-key".to_string()));
}

#[test]
fn test_config_custom_fallback_models() {
    let mut config = Config::default();
    config.llm.fallback_models = vec![
        "anthropic/claude-3-opus-latest".to_string(),
        "google/gemini-pro".to_string(),
    ];

    assert_eq!(config.llm.fallback_models.len(), 2);
}
