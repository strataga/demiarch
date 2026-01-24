//! Dynamic model routing with RL optimization
//!
//! This module provides intelligent model selection using reinforcement learning.
//! The key components are:
//!
//! - **Thompson Sampling Bandit**: Multi-armed bandit algorithm that balances
//!   exploration (trying less-tested models) with exploitation (using proven models).
//!
//! - **Task Context**: Information about the current task (agent type, complexity,
//!   constraints) used to match tasks with appropriate models.
//!
//! - **Model Registry**: Catalog of available models with their capabilities,
//!   pricing, and performance characteristics.
//!
//! - **Routing Store**: SQLite persistence for learning across sessions.
//!
//! ## How It Works
//!
//! 1. When a task needs to select a model, the router examines the task context
//! 2. Candidates are filtered by capability requirements and budget constraints
//! 3. Thompson Sampling selects from candidates, balancing exploration/exploitation
//! 4. After task completion, the outcome is recorded to improve future selections
//!
//! ## Example
//!
//! ```rust,ignore
//! use demiarch_core::routing::{ModelRouter, TaskContext, RoutingReward};
//! use demiarch_core::agents::AgentType;
//!
//! // Create a router
//! let router = ModelRouter::new();
//!
//! // Select a model for a task
//! let context = TaskContext::new(AgentType::Coder);
//! let decision = router.select(&context).await?;
//!
//! // Use the selected model...
//! // let response = llm_client.complete(messages, Some(&decision.model_id)).await?;
//!
//! // Record the outcome for learning
//! let reward = RoutingReward::new(
//!     context.routing_key(),
//!     decision.model_id,
//!     true, // success
//! )
//! .with_cost(actual_cost)
//! .with_latency(latency_ms);
//!
//! router.record_outcome(reward).await?;
//! ```

mod bandit;
mod router;
mod store;
mod types;

pub use bandit::ThompsonSamplingBandit;
pub use router::{ModelRouter, ModelRouterBuilder, RouterConfig};
pub use store::{RoutingStore, RoutingStoreSummary, CREATE_ROUTING_STATS_TABLE_SQL};
pub use types::{
    ModelCandidate, ModelRegistry, ModelStats, RoutingDecision, RoutingPreference, RoutingReason,
    RoutingReward, TaskComplexity, TaskContext,
};
