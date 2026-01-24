//! Specification pattern for composable business rules
//!
//! The Specification pattern encapsulates business rules that can be
//! combined using boolean logic (and, or, not).

use std::sync::Arc;

/// Core specification trait for business rules
///
/// Specifications are predicate objects that can be composed
/// to form complex business rules.
pub trait Specification<T>: Send + Sync {
    /// Check if the entity satisfies this specification
    fn is_satisfied_by(&self, entity: &T) -> bool;

    /// Combine with another specification using AND
    fn and<S: Specification<T> + 'static>(self, other: S) -> AndSpecification<T>
    where
        Self: Sized + 'static,
    {
        AndSpecification {
            left: Arc::new(self),
            right: Arc::new(other),
        }
    }

    /// Combine with another specification using OR
    fn or<S: Specification<T> + 'static>(self, other: S) -> OrSpecification<T>
    where
        Self: Sized + 'static,
    {
        OrSpecification {
            left: Arc::new(self),
            right: Arc::new(other),
        }
    }

    /// Negate this specification
    fn not(self) -> NotSpecification<T>
    where
        Self: Sized + 'static,
    {
        NotSpecification {
            spec: Arc::new(self),
        }
    }
}

/// AND composite specification
pub struct AndSpecification<T> {
    left: Arc<dyn Specification<T>>,
    right: Arc<dyn Specification<T>>,
}

impl<T> Specification<T> for AndSpecification<T>
where
    T: Send + Sync,
{
    fn is_satisfied_by(&self, entity: &T) -> bool {
        self.left.is_satisfied_by(entity) && self.right.is_satisfied_by(entity)
    }
}

/// OR composite specification
pub struct OrSpecification<T> {
    left: Arc<dyn Specification<T>>,
    right: Arc<dyn Specification<T>>,
}

impl<T> Specification<T> for OrSpecification<T>
where
    T: Send + Sync,
{
    fn is_satisfied_by(&self, entity: &T) -> bool {
        self.left.is_satisfied_by(entity) || self.right.is_satisfied_by(entity)
    }
}

/// NOT specification wrapper
pub struct NotSpecification<T> {
    spec: Arc<dyn Specification<T>>,
}

impl<T> Specification<T> for NotSpecification<T>
where
    T: Send + Sync,
{
    fn is_satisfied_by(&self, entity: &T) -> bool {
        !self.spec.is_satisfied_by(entity)
    }
}

/// Always true specification (identity for AND)
pub struct TrueSpec<T>(std::marker::PhantomData<T>);

impl<T> TrueSpec<T> {
    pub fn new() -> Self {
        Self(std::marker::PhantomData)
    }
}

impl<T> Default for TrueSpec<T> {
    fn default() -> Self {
        Self::new()
    }
}

impl<T: Send + Sync> Specification<T> for TrueSpec<T> {
    fn is_satisfied_by(&self, _entity: &T) -> bool {
        true
    }
}

/// Always false specification (identity for OR)
pub struct FalseSpec<T>(std::marker::PhantomData<T>);

impl<T> FalseSpec<T> {
    pub fn new() -> Self {
        Self(std::marker::PhantomData)
    }
}

impl<T> Default for FalseSpec<T> {
    fn default() -> Self {
        Self::new()
    }
}

impl<T: Send + Sync> Specification<T> for FalseSpec<T> {
    fn is_satisfied_by(&self, _entity: &T) -> bool {
        false
    }
}

/// A specification that uses a closure
pub struct PredicateSpec<T, F>
where
    F: Fn(&T) -> bool + Send + Sync,
{
    predicate: F,
    _phantom: std::marker::PhantomData<T>,
}

impl<T, F> PredicateSpec<T, F>
where
    F: Fn(&T) -> bool + Send + Sync,
{
    pub fn new(predicate: F) -> Self {
        Self {
            predicate,
            _phantom: std::marker::PhantomData,
        }
    }
}

impl<T, F> Specification<T> for PredicateSpec<T, F>
where
    T: Send + Sync,
    F: Fn(&T) -> bool + Send + Sync,
{
    fn is_satisfied_by(&self, entity: &T) -> bool {
        (self.predicate)(entity)
    }
}

/// Helper function to create a specification from a closure
pub fn spec<T, F>(predicate: F) -> PredicateSpec<T, F>
where
    F: Fn(&T) -> bool + Send + Sync,
{
    PredicateSpec::new(predicate)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Debug)]
    struct User {
        age: u32,
        is_active: bool,
        role: String,
    }

    struct AgeSpec {
        min_age: u32,
    }

    impl Specification<User> for AgeSpec {
        fn is_satisfied_by(&self, user: &User) -> bool {
            user.age >= self.min_age
        }
    }

    struct ActiveSpec;

    impl Specification<User> for ActiveSpec {
        fn is_satisfied_by(&self, user: &User) -> bool {
            user.is_active
        }
    }

    struct RoleSpec {
        role: String,
    }

    impl Specification<User> for RoleSpec {
        fn is_satisfied_by(&self, user: &User) -> bool {
            user.role == self.role
        }
    }

    #[test]
    fn test_single_specification() {
        let age_spec = AgeSpec { min_age: 18 };

        let adult = User {
            age: 25,
            is_active: true,
            role: "user".to_string(),
        };
        let minor = User {
            age: 15,
            is_active: true,
            role: "user".to_string(),
        };

        assert!(age_spec.is_satisfied_by(&adult));
        assert!(!age_spec.is_satisfied_by(&minor));
    }

    #[test]
    fn test_and_specification() {
        let age_spec = AgeSpec { min_age: 18 };
        let active_spec = ActiveSpec;

        let combined = age_spec.and(active_spec);

        let active_adult = User {
            age: 25,
            is_active: true,
            role: "user".to_string(),
        };
        let inactive_adult = User {
            age: 25,
            is_active: false,
            role: "user".to_string(),
        };
        let active_minor = User {
            age: 15,
            is_active: true,
            role: "user".to_string(),
        };

        assert!(combined.is_satisfied_by(&active_adult));
        assert!(!combined.is_satisfied_by(&inactive_adult));
        assert!(!combined.is_satisfied_by(&active_minor));
    }

    #[test]
    fn test_or_specification() {
        let admin_spec = RoleSpec {
            role: "admin".to_string(),
        };
        let age_spec = AgeSpec { min_age: 21 };

        let combined = admin_spec.or(age_spec);

        let young_admin = User {
            age: 18,
            is_active: true,
            role: "admin".to_string(),
        };
        let old_user = User {
            age: 25,
            is_active: true,
            role: "user".to_string(),
        };
        let young_user = User {
            age: 18,
            is_active: true,
            role: "user".to_string(),
        };

        assert!(combined.is_satisfied_by(&young_admin)); // admin
        assert!(combined.is_satisfied_by(&old_user)); // 21+
        assert!(!combined.is_satisfied_by(&young_user)); // neither
    }

    #[test]
    fn test_not_specification() {
        let active_spec = ActiveSpec;
        let inactive_spec = active_spec.not();

        let active_user = User {
            age: 25,
            is_active: true,
            role: "user".to_string(),
        };
        let inactive_user = User {
            age: 25,
            is_active: false,
            role: "user".to_string(),
        };

        assert!(!inactive_spec.is_satisfied_by(&active_user));
        assert!(inactive_spec.is_satisfied_by(&inactive_user));
    }

    #[test]
    fn test_complex_composition() {
        // (age >= 18 AND active) OR role == admin
        let age_and_active = AgeSpec { min_age: 18 }.and(ActiveSpec);
        let admin = RoleSpec {
            role: "admin".to_string(),
        };
        let combined = age_and_active.or(admin);

        let active_adult = User {
            age: 25,
            is_active: true,
            role: "user".to_string(),
        };
        let inactive_admin = User {
            age: 15,
            is_active: false,
            role: "admin".to_string(),
        };
        let inactive_minor = User {
            age: 15,
            is_active: false,
            role: "user".to_string(),
        };

        assert!(combined.is_satisfied_by(&active_adult));
        assert!(combined.is_satisfied_by(&inactive_admin));
        assert!(!combined.is_satisfied_by(&inactive_minor));
    }

    #[test]
    fn test_predicate_spec() {
        let adult_spec = spec(|u: &User| u.age >= 18);

        let adult = User {
            age: 25,
            is_active: true,
            role: "user".to_string(),
        };
        let minor = User {
            age: 15,
            is_active: true,
            role: "user".to_string(),
        };

        assert!(adult_spec.is_satisfied_by(&adult));
        assert!(!adult_spec.is_satisfied_by(&minor));
    }

    #[test]
    fn test_true_false_specs() {
        let user = User {
            age: 25,
            is_active: true,
            role: "user".to_string(),
        };

        assert!(TrueSpec::<User>::new().is_satisfied_by(&user));
        assert!(!FalseSpec::<User>::new().is_satisfied_by(&user));
    }
}
