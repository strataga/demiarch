//! Configuration management with file persistence

use anyhow::{Context, anyhow};
use serde::{Deserialize, Serialize};
use std::env;
use std::fs;
use std::path::PathBuf;

/// Demiarch configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    pub llm: LlmConfig,
    pub cost: CostConfig,
    pub routing: RoutingConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LlmConfig {
    #[serde(skip)]
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

impl LlmConfig {
    pub fn resolved_api_key(&self) -> anyhow::Result<Option<String>> {
        self.enforce_env_only()?;

        Ok(env::var("DEMIARCH_API_KEY")
            .or_else(|_| env::var("OPENROUTER_API_KEY"))
            .ok())
    }

    pub fn redacted_api_key(&self) -> anyhow::Result<Option<String>> {
        self.resolved_api_key().map(|opt| {
            opt.map(|key| {
                if key.len() <= 4 {
                    "***".to_string()
                } else {
                    let suffix = &key[key.len() - 4..];
                    format!("***{}", suffix)
                }
            })
        })
    }

    pub fn enforce_env_only(&self) -> anyhow::Result<()> {
        if self.api_key.is_some() {
            return Err(anyhow!(
                "LLM API keys must be provided via environment variables, not stored in configuration"
            ));
        }
        Ok(())
    }
}

impl Config {
    /// Get the config directory path
    pub fn config_dir() -> anyhow::Result<PathBuf> {
        let dir = if let Ok(custom_dir) = env::var("DEMIARCH_CONFIG_DIR") {
            PathBuf::from(custom_dir)
        } else {
            dirs::config_dir()
                .ok_or_else(|| anyhow!("Could not determine config directory"))?
                .join("demiarch")
        };
        Ok(dir)
    }

    /// Get the config file path
    pub fn config_path() -> anyhow::Result<PathBuf> {
        Ok(Self::config_dir()?.join("config.toml"))
    }

    /// Load configuration from file, or create default if it doesn't exist
    pub fn load() -> anyhow::Result<Self> {
        let path = Self::config_path()?;

        if path.exists() {
            let contents = fs::read_to_string(&path)
                .with_context(|| format!("Failed to read config file: {}", path.display()))?;
            let config: Config = toml::from_str(&contents)
                .with_context(|| format!("Failed to parse config file: {}", path.display()))?;
            config.validate()?;
            Ok(config)
        } else {
            // Return default config without creating file
            Ok(Config::default())
        }
    }

    /// Save configuration to file
    pub fn save(&self) -> anyhow::Result<()> {
        self.validate()?;

        let dir = Self::config_dir()?;
        fs::create_dir_all(&dir)
            .with_context(|| format!("Failed to create config directory: {}", dir.display()))?;

        let path = Self::config_path()?;
        let contents = toml::to_string_pretty(self)
            .context("Failed to serialize config")?;

        fs::write(&path, contents)
            .with_context(|| format!("Failed to write config file: {}", path.display()))?;

        Ok(())
    }

    /// Validate configuration
    pub fn validate(&self) -> anyhow::Result<()> {
        self.llm.enforce_env_only()
    }

    /// Get a configuration value by key
    pub fn get(&self, key: &str) -> anyhow::Result<String> {
        match key {
            // LLM settings
            "llm.default_model" => Ok(self.llm.default_model.clone()),
            "llm.fallback_models" => Ok(self.llm.fallback_models.join(", ")),
            "llm.temperature" => Ok(self.llm.temperature.to_string()),
            "llm.max_tokens" => Ok(self.llm.max_tokens.to_string()),
            "llm.timeout_secs" => Ok(self.llm.timeout_secs.to_string()),

            // Cost settings
            "cost.daily_limit_usd" => Ok(self.cost.daily_limit_usd.to_string()),
            "cost.alert_threshold" => Ok(self.cost.alert_threshold.to_string()),

            // Routing settings
            "routing.preference" => Ok(self.routing.preference.clone()),

            // API key (special handling - show redacted)
            "llm.api_key" | "api_key" => {
                match self.llm.redacted_api_key()? {
                    Some(redacted) => Ok(redacted),
                    None => Ok("(not set - use DEMIARCH_API_KEY or OPENROUTER_API_KEY env var)".to_string()),
                }
            }

            _ => Err(anyhow!("Unknown configuration key: {}. Use `demiarch config list` to see available keys.", key)),
        }
    }

    /// Set a configuration value by key
    pub fn set(&mut self, key: &str, value: &str) -> anyhow::Result<()> {
        match key {
            // LLM settings
            "llm.default_model" => {
                self.llm.default_model = value.to_string();
            }
            "llm.fallback_models" => {
                self.llm.fallback_models = value
                    .split(',')
                    .map(|s| s.trim().to_string())
                    .filter(|s| !s.is_empty())
                    .collect();
            }
            "llm.temperature" => {
                let temp: f32 = value.parse()
                    .with_context(|| format!("Invalid temperature value: {}", value))?;
                if !(0.0..=2.0).contains(&temp) {
                    return Err(anyhow!("Temperature must be between 0.0 and 2.0"));
                }
                self.llm.temperature = temp;
            }
            "llm.max_tokens" => {
                self.llm.max_tokens = value.parse()
                    .with_context(|| format!("Invalid max_tokens value: {}", value))?;
            }
            "llm.timeout_secs" => {
                self.llm.timeout_secs = value.parse()
                    .with_context(|| format!("Invalid timeout_secs value: {}", value))?;
            }

            // Cost settings
            "cost.daily_limit_usd" => {
                let limit: f64 = value.parse()
                    .with_context(|| format!("Invalid daily_limit_usd value: {}", value))?;
                if limit < 0.0 {
                    return Err(anyhow!("Daily limit must be non-negative"));
                }
                self.cost.daily_limit_usd = limit;
            }
            "cost.alert_threshold" => {
                let threshold: f64 = value.parse()
                    .with_context(|| format!("Invalid alert_threshold value: {}", value))?;
                if !(0.0..=1.0).contains(&threshold) {
                    return Err(anyhow!("Alert threshold must be between 0.0 and 1.0"));
                }
                self.cost.alert_threshold = threshold;
            }

            // Routing settings
            "routing.preference" => {
                let valid_prefs = ["balanced", "fast", "quality", "cost"];
                if !valid_prefs.contains(&value) {
                    return Err(anyhow!(
                        "Invalid routing preference: {}. Valid options: {}",
                        value,
                        valid_prefs.join(", ")
                    ));
                }
                self.routing.preference = value.to_string();
            }

            // API key cannot be set via config
            "llm.api_key" | "api_key" => {
                return Err(anyhow!(
                    "API keys cannot be stored in configuration for security. \
                     Set the DEMIARCH_API_KEY or OPENROUTER_API_KEY environment variable instead."
                ));
            }

            _ => {
                return Err(anyhow!(
                    "Unknown configuration key: {}. Use `demiarch config list` to see available keys.",
                    key
                ));
            }
        }
        Ok(())
    }

    /// List all configuration keys and their values
    pub fn list(&self) -> anyhow::Result<Vec<(String, String)>> {
        let keys = vec![
            "llm.default_model",
            "llm.fallback_models",
            "llm.temperature",
            "llm.max_tokens",
            "llm.timeout_secs",
            "llm.api_key",
            "cost.daily_limit_usd",
            "cost.alert_threshold",
            "routing.preference",
        ];

        keys.into_iter()
            .map(|key| {
                let value = self.get(key)?;
                Ok((key.to_string(), value))
            })
            .collect()
    }

    /// Reset configuration to defaults
    pub fn reset() -> anyhow::Result<()> {
        let path = Self::config_path()?;
        if path.exists() {
            fs::remove_file(&path)
                .with_context(|| format!("Failed to remove config file: {}", path.display()))?;
        }
        Ok(())
    }
}
