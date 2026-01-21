//! Feature Decomposition Domain
//!
//! This domain handles the decomposition of features into executable tasks
//! and the strategies for planning and assigning work to agents.

pub mod parser;
pub mod strategy;
pub mod task;

pub use parser::PlanParser;
pub use strategy::{DecompositionStrategy, KeywordDecompositionStrategy};
pub use task::{ExecutionPlan, PlanTask, TaskStatus};
