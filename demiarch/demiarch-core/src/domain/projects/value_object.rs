//! Project value objects

/// Project name value object
#[derive(Debug, Clone, PartialEq)]
pub struct ProjectName(String);

impl ProjectName {
    pub fn new(name: String) -> Result<Self, String> {
        if name.trim().is_empty() {
            return Err("Project name cannot be empty".to_string());
        }
        if name.len() > 100 {
            return Err("Project name too long (max 100 characters)".to_string());
        }
        Ok(ProjectName(name.trim().to_string()))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}
