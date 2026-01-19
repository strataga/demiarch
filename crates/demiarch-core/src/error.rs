//! Error types for Demiarch

use thiserror::Error;

/// Result type alias using Demiarch's Error
pub type Result<T> = std::result::Result<T, Error>;

/// Demiarch error types with helpful messages and suggestions
#[derive(Error, Debug)]
pub enum Error {
    // Entity errors (E001-E099)
    #[error("Feature '{0}' not found. Run `demiarch features list` to see all features.")]
    FeatureNotFound(String),

    #[error("Project '{0}' not found. Run `demiarch projects list` to see all projects.")]
    ProjectNotFound(String),

    #[error("Phase '{0}' not found.")]
    PhaseNotFound(String),

    // Network errors (E100-E199)
    #[error("Network error: {0}. Check your internet connection.")]
    NetworkError(#[from] reqwest::Error),

    #[error("LLM API error: {0}. Check your API key with `demiarch config get openrouter_api_key`.")]
    LLMError(String),

    #[error("Rate limited. Waiting {0} seconds before retry.")]
    RateLimited(u64),

    // Cost errors (E200-E299)
    #[error("Daily budget exceeded (${0:.2}/${1:.2}). Increase limit with `demiarch config set cost_daily_limit_usd {2}`.")]
    BudgetExceeded(f64, f64, f64),

    // Lock errors (E300-E399)
    #[error("Lock timeout: Resource '{0}' is held by another agent. Try again later.")]
    LockTimeout(String),

    // Database errors (E400-E499)
    #[error("Database error: {0}")]
    DatabaseError(#[from] sqlx::Error),

    // Plugin errors (E500-E599)
    #[error("Plugin '{0}' not found. Install with `demiarch plugin install {0}`.")]
    PluginNotFound(String),

    #[error("Plugin validation failed: {0}")]
    PluginValidationFailed(String),

    #[error("License expired for plugin '{0}'. Renew at {1}")]
    LicenseExpired(String, String),

    // Config errors (E600-E699)
    #[error("Configuration error: {0}")]
    ConfigError(String),

    // User errors (E700-E799)
    #[error("User cancelled operation")]
    UserCancelled,

    // Input errors (E800-E899)
    #[error("Invalid input: {0}")]
    InvalidInput(String),

    // Skill errors (E900-E999)
    #[error("Skill '{0}' not found. Run `demiarch skills list` to see all skills.")]
    SkillNotFound(String),

    #[error("Skill extraction failed: {0}")]
    SkillExtractionFailed(String),

    // Hook errors (E1000-E1099)
    #[error("Hook execution failed: {0}")]
    HookFailed(String),

    #[error("Hook timeout after {0} seconds")]
    HookTimeout(u64),

    // Routing errors (E1100-E1199)
    #[error("Model routing failed: {0}")]
    RoutingFailed(String),

    #[error("No suitable model found for task type '{0}'")]
    NoSuitableModel(String),

    // Context errors (E1200-E1299)
    #[error("Context retrieval failed: {0}")]
    ContextRetrievalFailed(String),

    #[error("Embedding generation failed: {0}")]
    EmbeddingFailed(String),

    // Generic errors
    #[error("{0}")]
    Other(String),

    #[error(transparent)]
    Io(#[from] std::io::Error),
}

impl Error {
    /// Get error code for this error type
    pub fn code(&self) -> &'static str {
        match self {
            Self::FeatureNotFound(_) => "E001",
            Self::ProjectNotFound(_) => "E002",
            Self::PhaseNotFound(_) => "E003",
            Self::NetworkError(_) => "E100",
            Self::LLMError(_) => "E101",
            Self::RateLimited(_) => "E102",
            Self::BudgetExceeded(..) => "E200",
            Self::LockTimeout(_) => "E300",
            Self::DatabaseError(_) => "E400",
            Self::PluginNotFound(_) => "E500",
            Self::PluginValidationFailed(_) => "E501",
            Self::LicenseExpired(..) => "E502",
            Self::ConfigError(_) => "E600",
            Self::UserCancelled => "E700",
            Self::InvalidInput(_) => "E800",
            Self::SkillNotFound(_) => "E900",
            Self::SkillExtractionFailed(_) => "E901",
            Self::HookFailed(_) => "E1000",
            Self::HookTimeout(_) => "E1001",
            Self::RoutingFailed(_) => "E1100",
            Self::NoSuitableModel(_) => "E1101",
            Self::ContextRetrievalFailed(_) => "E1200",
            Self::EmbeddingFailed(_) => "E1201",
            Self::Other(_) | Self::Io(_) => "E9999",
        }
    }

    /// Get suggestion for how to fix this error
    pub fn suggestion(&self) -> Option<String> {
        match self {
            Self::FeatureNotFound(_) => Some("demiarch features list".to_string()),
            Self::ProjectNotFound(_) => Some("demiarch projects list".to_string()),
            Self::NetworkError(_) => Some("Check internet connection".to_string()),
            Self::LLMError(_) => Some("demiarch config get openrouter_api_key".to_string()),
            Self::BudgetExceeded(_, _, suggested) => {
                Some(format!("demiarch config set cost_daily_limit_usd {}", suggested))
            }
            Self::PluginNotFound(name) => Some(format!("demiarch plugin install {}", name)),
            Self::SkillNotFound(_) => Some("demiarch skills list".to_string()),
            Self::ContextRetrievalFailed(_) => Some("demiarch context rebuild".to_string()),
            _ => None,
        }
    }
}
