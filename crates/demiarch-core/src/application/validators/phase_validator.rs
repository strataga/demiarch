//! Phase validation
//!
//! Validates phase-related inputs and business rules.

use crate::application::errors::{AppResult, ApplicationError};

/// Validator for phase-related operations
pub struct PhaseValidator;

impl PhaseValidator {
    /// Valid phase names in order
    pub const VALID_PHASES: &'static [&'static str] = &[
        "planning",
        "design",
        "implementation",
        "testing",
        "review",
        "deployment",
        "maintenance",
    ];

    /// Validate a phase name
    ///
    /// Rules:
    /// - Must be one of the predefined phases
    pub fn validate_name(name: &str) -> AppResult<()> {
        let name_lower = name.to_lowercase();

        if !Self::VALID_PHASES.contains(&name_lower.as_str()) {
            return Err(ApplicationError::validation(
                "phase",
                format!(
                    "Invalid phase '{}'. Valid phases: {}",
                    name,
                    Self::VALID_PHASES.join(", ")
                ),
            ));
        }

        Ok(())
    }

    /// Validate phase transition
    ///
    /// Rules:
    /// - Can move forward to any subsequent phase
    /// - Can move backward only one phase (for corrections)
    pub fn validate_transition(current: &str, target: &str) -> AppResult<()> {
        let current_lower = current.to_lowercase();
        let target_lower = target.to_lowercase();

        let current_idx = Self::VALID_PHASES
            .iter()
            .position(|&p| p == current_lower)
            .ok_or_else(|| {
                ApplicationError::validation("current_phase", "Invalid current phase")
            })?;

        let target_idx = Self::VALID_PHASES
            .iter()
            .position(|&p| p == target_lower)
            .ok_or_else(|| ApplicationError::validation("target_phase", "Invalid target phase"))?;

        // Can move forward any amount, or back exactly one
        if target_idx > current_idx || target_idx == current_idx.saturating_sub(1) {
            Ok(())
        } else {
            Err(ApplicationError::invalid_state(
                "Phase",
                current,
                format!("transition to {}", target),
            ))
        }
    }

    /// Get the next phase in sequence
    pub fn next_phase(current: &str) -> Option<&'static str> {
        let current_lower = current.to_lowercase();
        let current_idx = Self::VALID_PHASES
            .iter()
            .position(|&p| p == current_lower)?;

        Self::VALID_PHASES.get(current_idx + 1).copied()
    }

    /// Get the previous phase in sequence
    pub fn previous_phase(current: &str) -> Option<&'static str> {
        let current_lower = current.to_lowercase();
        let current_idx = Self::VALID_PHASES
            .iter()
            .position(|&p| p == current_lower)?;

        if current_idx > 0 {
            Self::VALID_PHASES.get(current_idx - 1).copied()
        } else {
            None
        }
    }

    /// Check if a phase is terminal
    pub fn is_terminal(phase: &str) -> bool {
        phase.to_lowercase() == "maintenance"
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_validate_name_valid() {
        for phase in PhaseValidator::VALID_PHASES {
            assert!(PhaseValidator::validate_name(phase).is_ok());
        }
        // Case insensitive
        assert!(PhaseValidator::validate_name("PLANNING").is_ok());
        assert!(PhaseValidator::validate_name("Design").is_ok());
    }

    #[test]
    fn test_validate_name_invalid() {
        assert!(PhaseValidator::validate_name("invalid").is_err());
        assert!(PhaseValidator::validate_name("").is_err());
    }

    #[test]
    fn test_validate_transition_forward() {
        assert!(PhaseValidator::validate_transition("planning", "design").is_ok());
        assert!(PhaseValidator::validate_transition("planning", "implementation").is_ok());
        assert!(PhaseValidator::validate_transition("planning", "deployment").is_ok());
    }

    #[test]
    fn test_validate_transition_backward_one() {
        assert!(PhaseValidator::validate_transition("design", "planning").is_ok());
        assert!(PhaseValidator::validate_transition("testing", "implementation").is_ok());
    }

    #[test]
    fn test_validate_transition_backward_multiple() {
        assert!(PhaseValidator::validate_transition("testing", "planning").is_err());
        assert!(PhaseValidator::validate_transition("deployment", "design").is_err());
    }

    #[test]
    fn test_next_phase() {
        assert_eq!(PhaseValidator::next_phase("planning"), Some("design"));
        assert_eq!(PhaseValidator::next_phase("review"), Some("deployment"));
        assert_eq!(PhaseValidator::next_phase("maintenance"), None);
    }

    #[test]
    fn test_previous_phase() {
        assert_eq!(PhaseValidator::previous_phase("design"), Some("planning"));
        assert_eq!(PhaseValidator::previous_phase("planning"), None);
    }

    #[test]
    fn test_is_terminal() {
        assert!(!PhaseValidator::is_terminal("planning"));
        assert!(!PhaseValidator::is_terminal("deployment"));
        assert!(PhaseValidator::is_terminal("maintenance"));
    }
}
