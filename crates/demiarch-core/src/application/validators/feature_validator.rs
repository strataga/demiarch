//! Feature validation
//!
//! Validates feature-related inputs and business rules.

use crate::application::errors::{AppResult, ApplicationError};

/// Validator for feature-related operations
pub struct FeatureValidator;

impl FeatureValidator {
    /// Validate a feature name
    ///
    /// Rules:
    /// - Must not be empty
    /// - Must be between 1 and 200 characters
    pub fn validate_name(name: &str) -> AppResult<()> {
        let name = name.trim();

        if name.is_empty() {
            return Err(ApplicationError::validation(
                "name",
                "Feature name cannot be empty",
            ));
        }

        if name.len() > 200 {
            return Err(ApplicationError::validation(
                "name",
                "Feature name must be 200 characters or less",
            ));
        }

        Ok(())
    }

    /// Validate a feature description
    ///
    /// Rules:
    /// - Must not be empty (features need context)
    /// - Must be 5000 characters or less
    pub fn validate_description(description: &str) -> AppResult<()> {
        let description = description.trim();

        if description.is_empty() {
            return Err(ApplicationError::validation(
                "description",
                "Feature description cannot be empty",
            ));
        }

        if description.len() > 5000 {
            return Err(ApplicationError::validation(
                "description",
                "Description must be 5000 characters or less",
            ));
        }

        Ok(())
    }

    /// Validate feature priority
    ///
    /// Rules:
    /// - Must be between 1 and 5 (1 = highest)
    pub fn validate_priority(priority: u8) -> AppResult<()> {
        if priority == 0 || priority > 5 {
            return Err(ApplicationError::validation(
                "priority",
                "Priority must be between 1 and 5",
            ));
        }

        Ok(())
    }

    /// Validate feature status transition
    pub fn validate_status_transition(current: &str, target: &str) -> AppResult<()> {
        let valid_transitions = [
            ("pending", "in_progress"),
            ("pending", "cancelled"),
            ("in_progress", "completed"),
            ("in_progress", "pending"),
            ("in_progress", "cancelled"),
        ];

        if !valid_transitions.contains(&(current, target)) {
            return Err(ApplicationError::invalid_state(
                "Feature",
                current,
                format!("transition to {}", target),
            ));
        }

        Ok(())
    }

    /// Validate all feature fields at once
    pub fn validate_create(name: &str, description: &str, priority: u8) -> AppResult<()> {
        Self::validate_name(name)?;
        Self::validate_description(description)?;
        Self::validate_priority(priority)?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_validate_name_valid() {
        assert!(FeatureValidator::validate_name("User Authentication").is_ok());
        assert!(FeatureValidator::validate_name("Add login button").is_ok());
    }

    #[test]
    fn test_validate_name_empty() {
        assert!(FeatureValidator::validate_name("").is_err());
        assert!(FeatureValidator::validate_name("   ").is_err());
    }

    #[test]
    fn test_validate_name_too_long() {
        let long_name = "a".repeat(201);
        assert!(FeatureValidator::validate_name(&long_name).is_err());
    }

    #[test]
    fn test_validate_description_valid() {
        assert!(FeatureValidator::validate_description("Implement OAuth2 flow").is_ok());
    }

    #[test]
    fn test_validate_description_empty() {
        assert!(FeatureValidator::validate_description("").is_err());
    }

    #[test]
    fn test_validate_priority() {
        assert!(FeatureValidator::validate_priority(1).is_ok());
        assert!(FeatureValidator::validate_priority(5).is_ok());
        assert!(FeatureValidator::validate_priority(0).is_err());
        assert!(FeatureValidator::validate_priority(6).is_err());
    }

    #[test]
    fn test_validate_status_transition() {
        assert!(FeatureValidator::validate_status_transition("pending", "in_progress").is_ok());
        assert!(FeatureValidator::validate_status_transition("in_progress", "completed").is_ok());
        assert!(FeatureValidator::validate_status_transition("pending", "completed").is_err());
        assert!(FeatureValidator::validate_status_transition("completed", "pending").is_err());
    }
}
