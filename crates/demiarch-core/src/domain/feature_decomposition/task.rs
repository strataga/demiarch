//! Task entities for feature decomposition
//!
//! Defines the core entities for representing tasks and execution plans.

use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Status of a plan task
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TaskStatus {
    /// Task is pending execution
    #[default]
    Pending,
    /// Task is currently being executed
    InProgress,
    /// Task completed successfully
    Completed,
    /// Task failed
    Failed,
    /// Task was skipped
    Skipped,
}

impl std::fmt::Display for TaskStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Pending => write!(f, "pending"),
            Self::InProgress => write!(f, "in_progress"),
            Self::Completed => write!(f, "completed"),
            Self::Failed => write!(f, "failed"),
            Self::Skipped => write!(f, "skipped"),
        }
    }
}

/// A task identified by the planner
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlanTask {
    /// Unique identifier for this task
    pub id: String,
    /// Type of agent that should execute this task
    pub agent_type: String,
    /// Description of what needs to be done
    pub description: String,
    /// Dependencies on other tasks (by id)
    pub depends_on: Vec<String>,
    /// Priority (1 = highest)
    pub priority: u8,
    /// Current status
    #[serde(default)]
    pub status: TaskStatus,
}

impl PlanTask {
    /// Create a new task
    pub fn new(id: impl Into<String>, agent_type: impl Into<String>, description: impl Into<String>) -> Self {
        Self {
            id: id.into(),
            agent_type: agent_type.into(),
            description: description.into(),
            depends_on: Vec::new(),
            priority: 1,
            status: TaskStatus::Pending,
        }
    }

    /// Create a coding task
    pub fn coding(id: impl Into<String>, description: impl Into<String>) -> Self {
        Self::new(id, "coder", description)
    }

    /// Create a review task
    pub fn review(id: impl Into<String>, description: impl Into<String>) -> Self {
        Self::new(id, "reviewer", description)
    }

    /// Create a test task
    pub fn test(id: impl Into<String>, description: impl Into<String>) -> Self {
        Self::new(id, "tester", description)
    }

    /// Add a dependency
    pub fn with_dependency(mut self, task_id: impl Into<String>) -> Self {
        self.depends_on.push(task_id.into());
        self
    }

    /// Add multiple dependencies
    pub fn with_dependencies(mut self, task_ids: Vec<String>) -> Self {
        self.depends_on.extend(task_ids);
        self
    }

    /// Set priority
    pub fn with_priority(mut self, priority: u8) -> Self {
        self.priority = priority;
        self
    }

    /// Check if this task is ready to execute (all dependencies met)
    pub fn is_ready(&self, completed_tasks: &[String]) -> bool {
        self.status == TaskStatus::Pending
            && self.depends_on.iter().all(|dep| completed_tasks.contains(dep))
    }

    /// Mark as in progress
    pub fn start(&mut self) {
        self.status = TaskStatus::InProgress;
    }

    /// Mark as completed
    pub fn complete(&mut self) {
        self.status = TaskStatus::Completed;
    }

    /// Mark as failed
    pub fn fail(&mut self) {
        self.status = TaskStatus::Failed;
    }

    /// Check if task is for a specific agent type
    pub fn is_for_agent(&self, agent_type: &str) -> bool {
        self.agent_type.eq_ignore_ascii_case(agent_type)
    }
}

/// The execution plan created by the planner
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecutionPlan {
    /// Unique plan ID
    pub id: Uuid,
    /// Feature being implemented
    pub feature_description: String,
    /// Ordered list of tasks
    pub tasks: Vec<PlanTask>,
    /// Whether code review is required
    pub requires_review: bool,
    /// Whether tests should be generated
    pub requires_tests: bool,
}

impl ExecutionPlan {
    /// Create a new execution plan
    pub fn new(description: impl Into<String>) -> Self {
        Self {
            id: Uuid::new_v4(),
            feature_description: description.into(),
            tasks: Vec::new(),
            requires_review: true,
            requires_tests: true,
        }
    }

    /// Add a task to the plan
    pub fn with_task(mut self, task: PlanTask) -> Self {
        self.tasks.push(task);
        self
    }

    /// Add multiple tasks
    pub fn with_tasks(mut self, tasks: Vec<PlanTask>) -> Self {
        self.tasks.extend(tasks);
        self
    }

    /// Set whether review is required
    pub fn with_review(mut self, required: bool) -> Self {
        self.requires_review = required;
        self
    }

    /// Set whether tests are required
    pub fn with_tests(mut self, required: bool) -> Self {
        self.requires_tests = required;
        self
    }

    /// Get coding tasks
    pub fn coding_tasks(&self) -> Vec<&PlanTask> {
        self.tasks.iter().filter(|t| t.is_for_agent("coder")).collect()
    }

    /// Get review tasks
    pub fn review_tasks(&self) -> Vec<&PlanTask> {
        self.tasks.iter().filter(|t| t.is_for_agent("reviewer")).collect()
    }

    /// Get test tasks
    pub fn test_tasks(&self) -> Vec<&PlanTask> {
        self.tasks.iter().filter(|t| t.is_for_agent("tester")).collect()
    }

    /// Get tasks ready for execution
    pub fn ready_tasks(&self) -> Vec<&PlanTask> {
        let completed: Vec<String> = self
            .tasks
            .iter()
            .filter(|t| t.status == TaskStatus::Completed)
            .map(|t| t.id.clone())
            .collect();

        self.tasks.iter().filter(|t| t.is_ready(&completed)).collect()
    }

    /// Get all pending tasks
    pub fn pending_tasks(&self) -> Vec<&PlanTask> {
        self.tasks
            .iter()
            .filter(|t| t.status == TaskStatus::Pending)
            .collect()
    }

    /// Check if plan is complete (all tasks done)
    pub fn is_complete(&self) -> bool {
        self.tasks.iter().all(|t| {
            t.status == TaskStatus::Completed || t.status == TaskStatus::Skipped
        })
    }

    /// Check if plan has failed (any task failed)
    pub fn has_failed(&self) -> bool {
        self.tasks.iter().any(|t| t.status == TaskStatus::Failed)
    }

    /// Get task by ID
    pub fn get_task(&self, id: &str) -> Option<&PlanTask> {
        self.tasks.iter().find(|t| t.id == id)
    }

    /// Get mutable task by ID
    pub fn get_task_mut(&mut self, id: &str) -> Option<&mut PlanTask> {
        self.tasks.iter_mut().find(|t| t.id == id)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_plan_task_creation() {
        let task = PlanTask::coding("task-1", "Implement login form");
        assert_eq!(task.id, "task-1");
        assert_eq!(task.agent_type, "coder");
        assert_eq!(task.status, TaskStatus::Pending);
    }

    #[test]
    fn test_plan_task_with_dependencies() {
        let task = PlanTask::review("review-1", "Review code")
            .with_dependency("task-1")
            .with_dependency("task-2")
            .with_priority(2);

        assert_eq!(task.depends_on.len(), 2);
        assert_eq!(task.priority, 2);
    }

    #[test]
    fn test_plan_task_is_ready() {
        let task = PlanTask::review("review-1", "Review")
            .with_dependency("task-1");

        assert!(!task.is_ready(&[]));
        assert!(task.is_ready(&["task-1".to_string()]));
    }

    #[test]
    fn test_execution_plan_creation() {
        let plan = ExecutionPlan::new("User authentication")
            .with_task(PlanTask::coding("task-1", "Implement login"))
            .with_task(PlanTask::review("review-1", "Review login").with_dependency("task-1"))
            .with_task(PlanTask::test("test-1", "Test login").with_dependency("task-1"));

        assert_eq!(plan.tasks.len(), 3);
        assert_eq!(plan.coding_tasks().len(), 1);
        assert_eq!(plan.review_tasks().len(), 1);
        assert_eq!(plan.test_tasks().len(), 1);
    }

    #[test]
    fn test_execution_plan_ready_tasks() {
        let mut plan = ExecutionPlan::new("Feature")
            .with_task(PlanTask::coding("task-1", "Code"))
            .with_task(PlanTask::review("review-1", "Review").with_dependency("task-1"));

        // Initially only task-1 is ready
        assert_eq!(plan.ready_tasks().len(), 1);
        assert_eq!(plan.ready_tasks()[0].id, "task-1");

        // After completing task-1, review-1 becomes ready
        plan.get_task_mut("task-1").unwrap().complete();
        assert_eq!(plan.ready_tasks().len(), 1);
        assert_eq!(plan.ready_tasks()[0].id, "review-1");
    }

    #[test]
    fn test_execution_plan_completion() {
        let mut plan = ExecutionPlan::new("Feature")
            .with_task(PlanTask::coding("task-1", "Code"));

        assert!(!plan.is_complete());
        plan.get_task_mut("task-1").unwrap().complete();
        assert!(plan.is_complete());
    }
}
