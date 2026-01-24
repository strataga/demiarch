//! Decomposition strategies
//!
//! Defines how features are decomposed into executable tasks.

use super::task::{ExecutionPlan, PlanTask};

/// Trait for feature decomposition strategies
pub trait DecompositionStrategy: Send + Sync {
    /// Check if the input should be decomposed
    fn should_decompose(&self, input: &str) -> bool;

    /// Create an execution plan from a feature description
    fn create_plan(&self, description: &str) -> ExecutionPlan;
}

/// Simple keyword-based decomposition strategy (heuristic fallback)
pub struct KeywordDecompositionStrategy;

impl KeywordDecompositionStrategy {
    /// Create a new keyword decomposition strategy
    pub fn new() -> Self {
        Self
    }

    /// Check if text indicates implementation work is needed
    fn needs_implementation(&self, text: &str) -> bool {
        let lower = text.to_lowercase();
        lower.contains("implement")
            || lower.contains("create")
            || lower.contains("code")
            || lower.contains("build")
            || lower.contains("add")
            || lower.contains("write")
    }

    /// Check if text indicates testing is needed
    fn needs_testing(&self, text: &str) -> bool {
        let lower = text.to_lowercase();
        lower.contains("test")
            || lower.contains("verify")
            || lower.contains("validate")
            || lower.contains("check")
    }

    /// Check if text indicates review is needed
    fn needs_review(&self, text: &str) -> bool {
        let lower = text.to_lowercase();
        lower.contains("review")
            || lower.contains("audit")
            || lower.contains("inspect")
            || lower.contains("quality")
    }
}

impl Default for KeywordDecompositionStrategy {
    fn default() -> Self {
        Self::new()
    }
}

impl DecompositionStrategy for KeywordDecompositionStrategy {
    fn should_decompose(&self, input: &str) -> bool {
        self.needs_implementation(input) || self.needs_testing(input) || self.needs_review(input)
    }

    fn create_plan(&self, description: &str) -> ExecutionPlan {
        let mut plan = ExecutionPlan::new(description);
        let mut task_id = 0;

        // Add coding task if implementation needed
        if self.needs_implementation(description) {
            task_id += 1;
            plan = plan.with_task(PlanTask::coding(
                format!("task-{}", task_id),
                "Implement the feature as described",
            ));
        }

        // Add review task if coding was added or review explicitly requested
        let coding_task_ids: Vec<String> =
            plan.coding_tasks().iter().map(|t| t.id.clone()).collect();

        if !coding_task_ids.is_empty() && plan.requires_review {
            task_id += 1;
            plan = plan.with_task(
                PlanTask::review(format!("task-{}", task_id), "Review the generated code")
                    .with_dependencies(coding_task_ids.clone())
                    .with_priority(2),
            );
        }

        // Add test task if coding was added or tests explicitly requested
        if !coding_task_ids.is_empty() && plan.requires_tests {
            task_id += 1;
            plan = plan.with_task(
                PlanTask::test(
                    format!("task-{}", task_id),
                    "Generate tests for the implementation",
                )
                .with_dependencies(coding_task_ids)
                .with_priority(3),
            );
        }

        plan
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_keyword_strategy_should_decompose() {
        let strategy = KeywordDecompositionStrategy::new();

        assert!(strategy.should_decompose("implement user login"));
        assert!(strategy.should_decompose("Create a new API endpoint"));
        assert!(strategy.should_decompose("test the authentication"));
        assert!(!strategy.should_decompose("explain how this works"));
    }

    #[test]
    fn test_keyword_strategy_create_plan() {
        let strategy = KeywordDecompositionStrategy::new();
        let plan = strategy.create_plan("Implement user authentication");

        assert!(!plan.tasks.is_empty());
        assert!(!plan.coding_tasks().is_empty());
        assert!(!plan.review_tasks().is_empty());
        assert!(!plan.test_tasks().is_empty());
    }

    #[test]
    fn test_plan_task_dependencies() {
        let strategy = KeywordDecompositionStrategy::new();
        let plan = strategy.create_plan("Create login form");

        // Review and test tasks should depend on coding tasks
        for review in plan.review_tasks() {
            assert!(!review.depends_on.is_empty());
        }

        for test in plan.test_tasks() {
            assert!(!test.depends_on.is_empty());
        }
    }
}
