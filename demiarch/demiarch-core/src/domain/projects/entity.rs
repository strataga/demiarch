//! Project Entity

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Project entity representing a development project
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Project {
    /// Unique identifier for the project
    pub id: Uuid,
    /// Human-readable project name
    pub name: String,
    /// Project description
    pub description: Option<String>,
    /// Project configuration
    pub config: ProjectConfig,
    /// Project creation timestamp
    pub created_at: DateTime<Utc>,
    /// Project last modified timestamp
    pub updated_at: DateTime<Utc>,
}

/// Project configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProjectConfig {
    /// Technology stack used in the project
    pub tech_stack: Vec<String>,
    /// Project-specific settings
    pub settings: serde_json::Value,
}

impl Project {
    /// Create a new project
    pub fn new(name: String, description: Option<String>) -> Self {
        let now = Utc::now();
        Self {
            id: Uuid::new_v4(),
            name,
            description,
            config: ProjectConfig {
                tech_stack: vec![],
                settings: serde_json::Value::Object(serde_json::Map::new()),
            },
            created_at: now,
            updated_at: now,
        }
    }

    /// Update project metadata
    pub fn update_metadata(&mut self, name: Option<String>, description: Option<String>) {
        if let Some(name) = name {
            self.name = name;
        }
        if let Some(description) = description {
            self.description = Some(description);
        }
        self.updated_at = Utc::now();
    }
}
