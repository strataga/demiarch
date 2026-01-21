//! Plan validation
//!
//! Validates execution plans and tasks.

use crate::application::errors::{AppResult, ApplicationError};

/// Validator for planning-related operations
pub struct PlanValidator;

impl PlanValidator {
    /// Validate a plan description/task
    ///
    /// Rules:
    /// - Must not be empty
    /// - Must be at least 10 characters (meaningful description)
    /// - Must be 10000 characters or less
    pub fn validate_description(description: &str) -> AppResult<()> {
        let description = description.trim();

        if description.is_empty() {
            return Err(ApplicationError::validation(
                "description",
                "Plan description cannot be empty",
            ));
        }

        if description.len() < 10 {
            return Err(ApplicationError::validation(
                "description",
                "Plan description must be at least 10 characters",
            ));
        }

        if description.len() > 10000 {
            return Err(ApplicationError::validation(
                "description",
                "Plan description must be 10000 characters or less",
            ));
        }

        Ok(())
    }

    /// Validate task ID format
    ///
    /// Rules:
    /// - Must not be empty
    /// - Must be alphanumeric with hyphens/underscores
    /// - Must be 50 characters or less
    pub fn validate_task_id(task_id: &str) -> AppResult<()> {
        if task_id.is_empty() {
            return Err(ApplicationError::validation("task_id", "Task ID cannot be empty"));
        }

        if task_id.len() > 50 {
            return Err(ApplicationError::validation(
                "task_id",
                "Task ID must be 50 characters or less",
            ));
        }

        if !task_id
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_')
        {
            return Err(ApplicationError::validation(
                "task_id",
                "Task ID must contain only letters, numbers, hyphens, and underscores",
            ));
        }

        Ok(())
    }

    /// Validate agent type
    ///
    /// Rules:
    /// - Must be one of the valid agent types
    pub fn validate_agent_type(agent_type: &str) -> AppResult<()> {
        let valid_types = ["orchestrator", "planner", "coder", "reviewer", "tester"];

        if !valid_types.contains(&agent_type.to_lowercase().as_str()) {
            return Err(ApplicationError::validation(
                "agent_type",
                format!(
                    "Invalid agent type '{}'. Valid types: {}",
                    agent_type,
                    valid_types.join(", ")
                ),
            ));
        }

        Ok(())
    }

    /// Validate task priority
    ///
    /// Rules:
    /// - Must be between 1 and 10
    pub fn validate_priority(priority: u8) -> AppResult<()> {
        if priority == 0 || priority > 10 {
            return Err(ApplicationError::validation(
                "priority",
                "Task priority must be between 1 and 10",
            ));
        }

        Ok(())
    }

    /// Validate task dependencies don't have cycles
    ///
    /// Simple check: a task cannot depend on itself
    pub fn validate_no_self_dependency(task_id: &str, dependencies: &[String]) -> AppResult<()> {
        if dependencies.iter().any(|d| d == task_id) {
            return Err(ApplicationError::validation(
                "dependencies",
                format!("Task '{}' cannot depend on itself", task_id),
            ));
        }

        Ok(())
    }

    /// Validate a complete task
    pub fn validate_task(
        task_id: &str,
        agent_type: &str,
        description: &str,
        priority: u8,
        dependencies: &[String],
    ) -> AppResult<()> {
        Self::validate_task_id(task_id)?;
        Self::validate_agent_type(agent_type)?;
        Self::validate_description(description)?;
        Self::validate_priority(priority)?;
        Self::validate_no_self_dependency(task_id, dependencies)?;
        Ok(())
    }

    /// Validate that a plan has at least one task
    pub fn validate_plan_not_empty(task_count: usize) -> AppResult<()> {
        if task_count == 0 {
            return Err(ApplicationError::validation(
                "tasks",
                "Plan must contain at least one task",
            ));
        }

        Ok(())
    }

    /// Validate maximum plan size
    pub fn validate_plan_size(task_count: usize) -> AppResult<()> {
        const MAX_TASKS: usize = 100;

        if task_count > MAX_TASKS {
            return Err(ApplicationError::validation(
                "tasks",
                format!("Plan cannot contain more than {} tasks", MAX_TASKS),
            ));
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_validate_description_valid() {
        assert!(PlanValidator::validate_description("Implement user authentication feature").is_ok());
    }

    #[test]
    fn test_validate_description_empty() {
        assert!(PlanValidator::validate_description("").is_err());
        assert!(PlanValidator::validate_description("   ").is_err());
    }

    #[test]
    fn test_validate_description_too_short() {
        assert!(PlanValidator::validate_description("short").is_err());
    }

    #[test]
    fn test_validate_description_too_long() {
        let long_desc = "a".repeat(10001);
        assert!(PlanValidator::validate_description(&long_desc).is_err());
    }

    #[test]
    fn test_validate_task_id_valid() {
        assert!(PlanValidator::validate_task_id("task-1").is_ok());
        assert!(PlanValidator::validate_task_id("task_123").is_ok());
        assert!(PlanValidator::validate_task_id("TaskOne").is_ok());
    }

    #[test]
    fn test_validate_task_id_invalid() {
        assert!(PlanValidator::validate_task_id("").is_err());
        assert!(PlanValidator::validate_task_id("task with spaces").is_err());
        assert!(PlanValidator::validate_task_id("task.one").is_err());
    }

    #[test]
    fn test_validate_agent_type() {
        assert!(PlanValidator::validate_agent_type("coder").is_ok());
        assert!(PlanValidator::validate_agent_type("Coder").is_ok());
        assert!(PlanValidator::validate_agent_type("reviewer").is_ok());
        assert!(PlanValidator::validate_agent_type("invalid").is_err());
    }

    #[test]
    fn test_validate_priority() {
        assert!(PlanValidator::validate_priority(1).is_ok());
        assert!(PlanValidator::validate_priority(10).is_ok());
        assert!(PlanValidator::validate_priority(0).is_err());
        assert!(PlanValidator::validate_priority(11).is_err());
    }

    #[test]
    fn test_validate_no_self_dependency() {
        assert!(PlanValidator::validate_no_self_dependency(
            "task-1",
            &["task-2".to_string(), "task-3".to_string()]
        ).is_ok());

        assert!(PlanValidator::validate_no_self_dependency(
            "task-1",
            &["task-1".to_string()]
        ).is_err());
    }

    #[test]
    fn test_validate_plan_size() {
        assert!(PlanValidator::validate_plan_not_empty(1).is_ok());
        assert!(PlanValidator::validate_plan_not_empty(0).is_err());

        assert!(PlanValidator::validate_plan_size(50).is_ok());
        assert!(PlanValidator::validate_plan_size(100).is_ok());
        assert!(PlanValidator::validate_plan_size(101).is_err());
    }
}
