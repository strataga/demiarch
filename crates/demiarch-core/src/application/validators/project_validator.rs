//! Project validation
//!
//! Validates project-related inputs and business rules.

use crate::application::errors::{AppResult, ApplicationError};

/// Validator for project-related operations
pub struct ProjectValidator;

impl ProjectValidator {
    /// Validate a project name
    ///
    /// Rules:
    /// - Must not be empty
    /// - Must be between 1 and 100 characters
    /// - Must contain only alphanumeric characters, hyphens, and underscores
    /// - Must start with a letter
    pub fn validate_name(name: &str) -> AppResult<()> {
        let name = name.trim();

        if name.is_empty() {
            return Err(ApplicationError::validation("name", "Project name cannot be empty"));
        }

        if name.len() > 100 {
            return Err(ApplicationError::validation(
                "name",
                "Project name must be 100 characters or less",
            ));
        }

        let first_char = name.chars().next().unwrap();
        if !first_char.is_ascii_alphabetic() {
            return Err(ApplicationError::validation(
                "name",
                "Project name must start with a letter",
            ));
        }

        if !name.chars().all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_') {
            return Err(ApplicationError::validation(
                "name",
                "Project name can only contain letters, numbers, hyphens, and underscores",
            ));
        }

        Ok(())
    }

    /// Validate a project description
    ///
    /// Rules:
    /// - Can be empty
    /// - Must be 1000 characters or less
    pub fn validate_description(description: &str) -> AppResult<()> {
        if description.len() > 1000 {
            return Err(ApplicationError::validation(
                "description",
                "Description must be 1000 characters or less",
            ));
        }

        Ok(())
    }

    /// Validate a framework name
    ///
    /// Rules:
    /// - Must be a recognized framework or empty
    pub fn validate_framework(framework: &str) -> AppResult<()> {
        if framework.is_empty() {
            return Ok(());
        }

        let allowed_frameworks = [
            "rust",
            "python",
            "nodejs",
            "react",
            "vue",
            "angular",
            "django",
            "flask",
            "fastapi",
            "rails",
            "spring",
            "express",
            "nextjs",
            "svelte",
            "go",
            "actix",
            "axum",
            "rocket",
            "other",
        ];

        let framework_lower = framework.to_lowercase();
        if !allowed_frameworks.contains(&framework_lower.as_str()) {
            return Err(ApplicationError::validation(
                "framework",
                format!(
                    "Unknown framework '{}'. Allowed: {}",
                    framework,
                    allowed_frameworks.join(", ")
                ),
            ));
        }

        Ok(())
    }

    /// Validate project path
    ///
    /// Rules:
    /// - Must not be empty
    /// - Must be a valid path format
    pub fn validate_path(path: &str) -> AppResult<()> {
        if path.trim().is_empty() {
            return Err(ApplicationError::validation("path", "Project path cannot be empty"));
        }

        // Check for invalid path characters (basic validation)
        let invalid_chars = ['<', '>', '"', '|', '?', '*'];
        if path.chars().any(|c| invalid_chars.contains(&c)) {
            return Err(ApplicationError::validation(
                "path",
                "Path contains invalid characters",
            ));
        }

        Ok(())
    }

    /// Validate all project fields at once
    pub fn validate_create(
        name: &str,
        description: &str,
        framework: &str,
        path: &str,
    ) -> AppResult<()> {
        Self::validate_name(name)?;
        Self::validate_description(description)?;
        Self::validate_framework(framework)?;
        Self::validate_path(path)?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_validate_name_valid() {
        assert!(ProjectValidator::validate_name("my-project").is_ok());
        assert!(ProjectValidator::validate_name("MyProject").is_ok());
        assert!(ProjectValidator::validate_name("project_123").is_ok());
        assert!(ProjectValidator::validate_name("a").is_ok());
    }

    #[test]
    fn test_validate_name_empty() {
        assert!(ProjectValidator::validate_name("").is_err());
        assert!(ProjectValidator::validate_name("   ").is_err());
    }

    #[test]
    fn test_validate_name_too_long() {
        let long_name = "a".repeat(101);
        assert!(ProjectValidator::validate_name(&long_name).is_err());
    }

    #[test]
    fn test_validate_name_must_start_with_letter() {
        assert!(ProjectValidator::validate_name("123project").is_err());
        assert!(ProjectValidator::validate_name("-project").is_err());
        assert!(ProjectValidator::validate_name("_project").is_err());
    }

    #[test]
    fn test_validate_name_invalid_chars() {
        assert!(ProjectValidator::validate_name("my project").is_err());
        assert!(ProjectValidator::validate_name("my.project").is_err());
        assert!(ProjectValidator::validate_name("my@project").is_err());
    }

    #[test]
    fn test_validate_description() {
        assert!(ProjectValidator::validate_description("").is_ok());
        assert!(ProjectValidator::validate_description("A short description").is_ok());

        let long_desc = "a".repeat(1001);
        assert!(ProjectValidator::validate_description(&long_desc).is_err());
    }

    #[test]
    fn test_validate_framework() {
        assert!(ProjectValidator::validate_framework("").is_ok());
        assert!(ProjectValidator::validate_framework("rust").is_ok());
        assert!(ProjectValidator::validate_framework("Rust").is_ok());
        assert!(ProjectValidator::validate_framework("python").is_ok());
        assert!(ProjectValidator::validate_framework("other").is_ok());

        assert!(ProjectValidator::validate_framework("unknown-framework").is_err());
    }

    #[test]
    fn test_validate_path() {
        assert!(ProjectValidator::validate_path("/home/user/projects").is_ok());
        assert!(ProjectValidator::validate_path("./relative/path").is_ok());
        assert!(ProjectValidator::validate_path("C:\\Users\\project").is_ok());

        assert!(ProjectValidator::validate_path("").is_err());
        assert!(ProjectValidator::validate_path("path<invalid>").is_err());
    }

    #[test]
    fn test_validate_create() {
        assert!(ProjectValidator::validate_create(
            "my-project",
            "A description",
            "rust",
            "/path/to/project"
        ).is_ok());

        assert!(ProjectValidator::validate_create(
            "",
            "A description",
            "rust",
            "/path/to/project"
        ).is_err());
    }
}
