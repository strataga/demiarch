//! Configuration management

use serde::{Deserialize, Serialize};

/// Demiarch configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    pub llm: LlmConfig,
    pub cost: CostConfig,
    pub routing: RoutingConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LlmConfig {
    pub api_key: Option<String>,
    pub default_model: String,
    pub fallback_models: Vec<String>,
    pub temperature: f32,
    pub max_tokens: usize,
    pub timeout_secs: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CostConfig {
    pub daily_limit_usd: f64,
    pub alert_threshold: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RoutingConfig {
    pub preference: String,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            llm: LlmConfig {
                api_key: None,
                default_model: "anthropic/claude-sonnet-4-20250514".to_string(),
                fallback_models: vec![
                    "anthropic/claude-3-5-haiku-latest".to_string(),
                    "openai/gpt-4o".to_string(),
                ],
                temperature: 0.7,
                max_tokens: 8192,
                timeout_secs: 120,
            },
            cost: CostConfig {
                daily_limit_usd: 10.0,
                alert_threshold: 0.8,
            },
            routing: RoutingConfig {
                preference: "balanced".to_string(),
            },
        }
    }
}
