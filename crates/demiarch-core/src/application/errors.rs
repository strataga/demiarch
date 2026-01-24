//! Application layer errors
//!
//! Error types for application-level operations.

use std::fmt;

use crate::error::Error;

/// Application layer error types
#[derive(Debug)]
pub enum ApplicationError {
    /// Validation error with field and message
    Validation { field: String, message: String },
    /// Entity not found
    NotFound { entity: String, id: String },
    /// Duplicate entity
    Duplicate {
        entity: String,
        field: String,
        value: String,
    },
    /// Operation not allowed in current state
    InvalidState {
        entity: String,
        current: String,
        operation: String,
    },
    /// Authorization error
    Unauthorized { resource: String, action: String },
    /// Domain error wrapper
    Domain(Error),
    /// Database error
    Database(String),
}

impl ApplicationError {
    /// Create a validation error
    pub fn validation(field: impl Into<String>, message: impl Into<String>) -> Self {
        Self::Validation {
            field: field.into(),
            message: message.into(),
        }
    }

    /// Create a not found error
    pub fn not_found(entity: impl Into<String>, id: impl Into<String>) -> Self {
        Self::NotFound {
            entity: entity.into(),
            id: id.into(),
        }
    }

    /// Create a duplicate error
    pub fn duplicate(
        entity: impl Into<String>,
        field: impl Into<String>,
        value: impl Into<String>,
    ) -> Self {
        Self::Duplicate {
            entity: entity.into(),
            field: field.into(),
            value: value.into(),
        }
    }

    /// Create an invalid state error
    pub fn invalid_state(
        entity: impl Into<String>,
        current: impl Into<String>,
        operation: impl Into<String>,
    ) -> Self {
        Self::InvalidState {
            entity: entity.into(),
            current: current.into(),
            operation: operation.into(),
        }
    }

    /// Create an unauthorized error
    pub fn unauthorized(resource: impl Into<String>, action: impl Into<String>) -> Self {
        Self::Unauthorized {
            resource: resource.into(),
            action: action.into(),
        }
    }

    /// Wrap a domain error
    pub fn domain(error: Error) -> Self {
        Self::Domain(error)
    }

    /// Create a database error
    pub fn database(message: impl Into<String>) -> Self {
        Self::Database(message.into())
    }
}

impl fmt::Display for ApplicationError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Validation { field, message } => {
                write!(f, "Validation error for '{}': {}", field, message)
            }
            Self::NotFound { entity, id } => {
                write!(f, "{} with id '{}' not found", entity, id)
            }
            Self::Duplicate {
                entity,
                field,
                value,
            } => {
                write!(f, "{} with {} '{}' already exists", entity, field, value)
            }
            Self::InvalidState {
                entity,
                current,
                operation,
            } => {
                write!(f, "Cannot {} {} in '{}' state", operation, entity, current)
            }
            Self::Unauthorized { resource, action } => {
                write!(f, "Not authorized to {} {}", action, resource)
            }
            Self::Domain(e) => write!(f, "Domain error: {}", e),
            Self::Database(msg) => write!(f, "Database error: {}", msg),
        }
    }
}

impl std::error::Error for ApplicationError {}

impl From<Error> for ApplicationError {
    fn from(error: Error) -> Self {
        Self::Domain(error)
    }
}

/// Result type for application operations
pub type AppResult<T> = Result<T, ApplicationError>;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_validation_error() {
        let err = ApplicationError::validation("email", "must be a valid email address");
        assert!(err.to_string().contains("email"));
        assert!(err.to_string().contains("valid email"));
    }

    #[test]
    fn test_not_found_error() {
        let err = ApplicationError::not_found("Project", "123");
        assert!(err.to_string().contains("Project"));
        assert!(err.to_string().contains("123"));
        assert!(err.to_string().contains("not found"));
    }

    #[test]
    fn test_duplicate_error() {
        let err = ApplicationError::duplicate("User", "email", "test@example.com");
        assert!(err.to_string().contains("already exists"));
    }

    #[test]
    fn test_invalid_state_error() {
        let err = ApplicationError::invalid_state("Session", "completed", "pause");
        assert!(err.to_string().contains("Cannot pause"));
        assert!(err.to_string().contains("completed"));
    }
}
