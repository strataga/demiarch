//! Model Router - Dynamic model selection with RL optimization
//!
//! This module provides the main ModelRouter that combines:
//! - Thompson Sampling bandit for learning optimal model selection
//! - Cost-aware routing to stay within budget
//! - Task context-based filtering
//! - Persistent learning across sessions

use std::collections::HashMap;
use std::sync::Arc;
use std::time::Instant;

use tokio::sync::RwLock;
use tracing::{debug, info};

use super::bandit::ThompsonSamplingBandit;
use super::store::RoutingStore;
use super::types::{
    ModelCandidate, ModelRegistry, ModelStats, RoutingDecision, RoutingPreference, RoutingReason,
    RoutingReward, TaskContext,
};
use crate::cost::CostTracker;
use crate::error::{Error, Result};

/// Configuration for the model router
#[derive(Debug, Clone)]
pub struct RouterConfig {
    /// Routing preference (balanced, fast, quality, cost)
    pub preference: RoutingPreference,
    /// Exploration factor (higher = more exploration)
    pub exploration_factor: f64,
    /// Minimum samples before trusting statistics
    pub min_samples: u64,
    /// Whether to persist statistics
    pub persist_stats: bool,
    /// Default model when no suitable candidate found
    pub default_model: String,
    /// Fallback models in order of preference
    pub fallback_models: Vec<String>,
}

impl Default for RouterConfig {
    fn default() -> Self {
        Self {
            preference: RoutingPreference::Balanced,
            exploration_factor: 1.0,
            min_samples: 5,
            persist_stats: true,
            default_model: "anthropic/claude-sonnet-4-20250514".to_string(),
            fallback_models: vec![
                "anthropic/claude-3-5-haiku-latest".to_string(),
                "openai/gpt-4o".to_string(),
            ],
        }
    }
}

impl RouterConfig {
    /// Create config from routing preference string
    pub fn from_preference(pref: &str) -> Self {
        let preference = pref.parse().unwrap_or(RoutingPreference::Balanced);
        Self {
            preference,
            ..Default::default()
        }
    }
}

/// Model Router with RL-based selection
///
/// The router maintains a Thompson Sampling bandit to learn which models
/// perform best for different task types. It also considers cost constraints
/// and user preferences when making routing decisions.
pub struct ModelRouter {
    /// Configuration
    config: RouterConfig,
    /// Model registry
    registry: ModelRegistry,
    /// Thompson Sampling bandit
    bandit: Arc<RwLock<ThompsonSamplingBandit>>,
    /// Optional persistent store
    store: Option<Arc<RoutingStore>>,
    /// Optional cost tracker for budget awareness
    cost_tracker: Option<Arc<CostTracker>>,
    /// Cache of pending decisions (for reward attribution)
    pending_decisions: Arc<RwLock<HashMap<String, PendingDecision>>>,
}

/// A pending routing decision awaiting reward feedback
#[derive(Debug, Clone)]
#[allow(dead_code)]
struct PendingDecision {
    /// The routing key
    routing_key: String,
    /// Selected model
    model_id: String,
    /// Start time for latency calculation (for future auto-latency calculation)
    start_time: Instant,
    /// Estimated cost for comparison
    estimated_cost: f64,
    /// Estimated latency for comparison
    estimated_latency_ms: u64,
}

impl ModelRouter {
    /// Create a new router with default configuration
    pub fn new() -> Self {
        Self {
            config: RouterConfig::default(),
            registry: ModelRegistry::with_defaults(),
            bandit: Arc::new(RwLock::new(ThompsonSamplingBandit::new())),
            store: None,
            cost_tracker: None,
            pending_decisions: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Create a router with custom configuration
    pub fn with_config(config: RouterConfig) -> Self {
        let bandit = ThompsonSamplingBandit::new()
            .with_exploration_factor(config.exploration_factor)
            .with_min_samples(config.min_samples);

        Self {
            config,
            registry: ModelRegistry::with_defaults(),
            bandit: Arc::new(RwLock::new(bandit)),
            store: None,
            cost_tracker: None,
            pending_decisions: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Set the routing store for persistence
    pub fn with_store(mut self, store: Arc<RoutingStore>) -> Self {
        self.store = Some(store);
        self
    }

    /// Set the cost tracker for budget awareness
    pub fn with_cost_tracker(mut self, tracker: Arc<CostTracker>) -> Self {
        self.cost_tracker = Some(tracker);
        self
    }

    /// Set a custom model registry
    pub fn with_registry(mut self, registry: ModelRegistry) -> Self {
        self.registry = registry;
        self
    }

    /// Load statistics from the persistent store
    pub async fn load_stats(&self) -> Result<()> {
        if let Some(store) = &self.store {
            let stats = store.load_all_stats().await?;
            let mut bandit = self.bandit.write().await;
            bandit.import_stats(stats);
            info!("Loaded routing statistics from store");
        }
        Ok(())
    }

    /// Save current statistics to the persistent store
    pub async fn save_stats(&self) -> Result<()> {
        if let Some(store) = &self.store {
            let bandit = self.bandit.read().await;
            let all_stats = bandit.all_stats();

            // Flatten to a list
            let stats_list: Vec<ModelStats> = all_stats
                .values()
                .flat_map(|m| m.values().cloned())
                .collect();

            if !stats_list.is_empty() {
                store.save_all_stats(&stats_list).await?;
                info!(
                    count = stats_list.len(),
                    "Saved routing statistics to store"
                );
            }
        }
        Ok(())
    }

    /// Select a model for a task
    ///
    /// Returns a routing decision with the selected model and reasoning.
    pub async fn select(&self, context: &TaskContext) -> Result<RoutingDecision> {
        let routing_key = context.routing_key();

        // Get candidates that can handle this task
        let candidates = self.registry.candidates_for(context);

        if candidates.is_empty() {
            // No candidates - use fallback
            return self.create_fallback_decision(context).await;
        }

        // Apply budget constraints if we have a cost tracker
        let filtered_candidates = self.apply_budget_filter(candidates, context).await;

        if filtered_candidates.is_empty() {
            // Budget too tight - use cheapest available
            return self.create_budget_constrained_decision(context).await;
        }

        // Use Thompson Sampling to select
        let mut bandit = self.bandit.write().await;
        let selection = bandit.select(&routing_key, &filtered_candidates, self.config.preference);

        match selection {
            Some((model, sampled_value, is_exploration)) => {
                // Record pending decision for reward attribution
                let estimated_cost = model.estimate_cost(
                    context.estimated_input_tokens,
                    context.estimated_input_tokens, // Assume similar output
                );
                let estimated_latency = self.estimate_latency(model);

                let pending = PendingDecision {
                    routing_key: routing_key.clone(),
                    model_id: model.model_id.clone(),
                    start_time: Instant::now(),
                    estimated_cost,
                    estimated_latency_ms: estimated_latency,
                };

                let decision_id = uuid::Uuid::new_v4().to_string();
                {
                    let mut pending_map = self.pending_decisions.write().await;
                    pending_map.insert(decision_id.clone(), pending);
                }

                let reason = if is_exploration {
                    // Get uncertainty for this model
                    let uncertainty = bandit
                        .get_stats(&routing_key, &model.model_id)
                        .map(|s| s.uncertainty())
                        .unwrap_or(0.5);
                    RoutingReason::Exploration { uncertainty }
                } else {
                    RoutingReason::Performance {
                        expected_reward: sampled_value,
                    }
                };

                let alternatives: Vec<String> = filtered_candidates
                    .iter()
                    .filter(|c| c.model_id != model.model_id)
                    .map(|c| c.model_id.clone())
                    .collect();

                let mut decision =
                    RoutingDecision::new(model.model_id.clone(), sampled_value, reason)
                        .with_alternatives(alternatives);

                if is_exploration {
                    decision = decision.as_exploration();
                }

                debug!(
                    routing_key = %routing_key,
                    model = %model.model_id,
                    sampled_value = sampled_value,
                    is_exploration = is_exploration,
                    "Selected model for task"
                );

                Ok(decision)
            }
            None => self.create_fallback_decision(context).await,
        }
    }

    /// Record the outcome of a routing decision
    ///
    /// This updates the bandit with the observed reward.
    pub async fn record_outcome(&self, reward: RoutingReward) -> Result<()> {
        let routing_key = reward.routing_key.clone();
        let model_id = reward.model_id.clone();

        // Look up the pending decision for expected values
        let pending = {
            let pending_map = self.pending_decisions.read().await;
            pending_map
                .values()
                .find(|p| p.routing_key == routing_key && p.model_id == model_id)
                .cloned()
        };

        let (expected_cost, expected_latency) = pending
            .map(|p| (p.estimated_cost, p.estimated_latency_ms))
            .unwrap_or((reward.actual_cost_usd, reward.latency_ms));

        // Compute the reward value
        let computed_reward = reward.compute_reward(expected_cost, expected_latency);

        // Update the bandit
        {
            let mut bandit = self.bandit.write().await;
            bandit.update(&routing_key, &model_id, computed_reward);
        }

        // Persist if configured
        if self.config.persist_stats {
            // We could batch this for efficiency, but for now save immediately
            if let Some(store) = &self.store {
                let bandit = self.bandit.read().await;
                if let Some(stats) = bandit.get_stats(&routing_key, &model_id) {
                    store.save_stats(stats).await?;
                }
            }
        }

        debug!(
            routing_key = %routing_key,
            model = %model_id,
            success = reward.success,
            computed_reward = computed_reward,
            "Recorded routing outcome"
        );

        Ok(())
    }

    /// Get the expected performance for each model for a task type
    pub async fn get_expected_values(&self, routing_key: &str) -> HashMap<String, f64> {
        let bandit = self.bandit.read().await;
        bandit.expected_values(routing_key)
    }

    /// Get statistics for a specific routing key
    pub async fn get_stats(&self, routing_key: &str) -> Option<HashMap<String, ModelStats>> {
        let bandit = self.bandit.read().await;
        bandit.stats_for_key(routing_key).cloned()
    }

    /// Get the model registry
    pub fn registry(&self) -> &ModelRegistry {
        &self.registry
    }

    /// Get the current configuration
    pub fn config(&self) -> &RouterConfig {
        &self.config
    }

    /// Apply budget filtering to candidates
    async fn apply_budget_filter<'a>(
        &self,
        candidates: Vec<&'a ModelCandidate>,
        context: &TaskContext,
    ) -> Vec<&'a ModelCandidate> {
        // If we have a cost tracker, check remaining budget
        if let Some(tracker) = &self.cost_tracker {
            let remaining = tracker.remaining_budget();

            if remaining < 0.001 {
                // Essentially no budget - return only cheapest models
                return candidates
                    .into_iter()
                    .filter(|c| c.input_cost_per_million < 1.0)
                    .collect();
            }

            // Filter to models that won't likely exceed remaining budget
            candidates
                .into_iter()
                .filter(|c| {
                    let estimated = c.estimate_cost(
                        context.estimated_input_tokens,
                        context.estimated_input_tokens,
                    );
                    estimated < remaining * 0.5 // Be conservative
                })
                .collect()
        } else {
            candidates
        }
    }

    /// Create a fallback decision when no candidates are available
    async fn create_fallback_decision(&self, context: &TaskContext) -> Result<RoutingDecision> {
        // Try default model
        if self.registry.get(&self.config.default_model).is_some() {
            return Ok(RoutingDecision::new(
                self.config.default_model.clone(),
                0.5,
                RoutingReason::Default,
            ));
        }

        // Try fallback models
        for fallback in &self.config.fallback_models {
            if self.registry.get(fallback).is_some() {
                return Ok(RoutingDecision::new(
                    fallback.clone(),
                    0.3,
                    RoutingReason::FallbackOnly,
                ));
            }
        }

        Err(Error::NoSuitableModel(format!(
            "No model available for task: {}",
            context.routing_key()
        )))
    }

    /// Create a decision when budget is too constrained
    async fn create_budget_constrained_decision(
        &self,
        _context: &TaskContext,
    ) -> Result<RoutingDecision> {
        // Find the cheapest model
        let cheapest = self.registry.all().min_by(|a, b| {
            a.input_cost_per_million
                .partial_cmp(&b.input_cost_per_million)
                .unwrap()
        });

        match cheapest {
            Some(model) => {
                let remaining = self
                    .cost_tracker
                    .as_ref()
                    .map(|t| t.remaining_budget())
                    .unwrap_or(0.0);

                Ok(RoutingDecision::new(
                    model.model_id.clone(),
                    0.4,
                    RoutingReason::CostConstraint {
                        within_budget: remaining,
                    },
                ))
            }
            None => Err(Error::NoSuitableModel(
                "No models available in registry".to_string(),
            )),
        }
    }

    /// Estimate latency for a model (simplified heuristic)
    fn estimate_latency(&self, model: &ModelCandidate) -> u64 {
        // Base latency estimate based on speed tier
        // Speed tier 5 = ~500ms, tier 1 = ~5000ms
        match model.speed_tier {
            5 => 500,
            4 => 1000,
            3 => 2000,
            2 => 3500,
            _ => 5000,
        }
    }
}

impl Default for ModelRouter {
    fn default() -> Self {
        Self::new()
    }
}

/// Builder for ModelRouter
pub struct ModelRouterBuilder {
    config: RouterConfig,
    registry: Option<ModelRegistry>,
    store: Option<Arc<RoutingStore>>,
    cost_tracker: Option<Arc<CostTracker>>,
}

impl Default for ModelRouterBuilder {
    fn default() -> Self {
        Self::new()
    }
}

impl ModelRouterBuilder {
    /// Create a new builder
    pub fn new() -> Self {
        Self {
            config: RouterConfig::default(),
            registry: None,
            store: None,
            cost_tracker: None,
        }
    }

    /// Set configuration
    pub fn config(mut self, config: RouterConfig) -> Self {
        self.config = config;
        self
    }

    /// Set routing preference
    pub fn preference(mut self, preference: RoutingPreference) -> Self {
        self.config.preference = preference;
        self
    }

    /// Set model registry
    pub fn registry(mut self, registry: ModelRegistry) -> Self {
        self.registry = Some(registry);
        self
    }

    /// Set routing store
    pub fn store(mut self, store: Arc<RoutingStore>) -> Self {
        self.store = Some(store);
        self
    }

    /// Set cost tracker
    pub fn cost_tracker(mut self, tracker: Arc<CostTracker>) -> Self {
        self.cost_tracker = Some(tracker);
        self
    }

    /// Build the router
    pub fn build(self) -> ModelRouter {
        let bandit = ThompsonSamplingBandit::new()
            .with_exploration_factor(self.config.exploration_factor)
            .with_min_samples(self.config.min_samples);

        ModelRouter {
            config: self.config,
            registry: self.registry.unwrap_or_else(ModelRegistry::with_defaults),
            bandit: Arc::new(RwLock::new(bandit)),
            store: self.store,
            cost_tracker: self.cost_tracker,
            pending_decisions: Arc::new(RwLock::new(HashMap::new())),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::agents::AgentType;
    use crate::routing::types::TaskComplexity;

    #[tokio::test]
    async fn test_router_select_basic() {
        let router = ModelRouter::new();

        let context = TaskContext::new(AgentType::Coder).with_complexity(TaskComplexity::Medium);

        let decision = router.select(&context).await.unwrap();

        assert!(!decision.model_id.is_empty());
        assert!(decision.confidence > 0.0);
    }

    #[tokio::test]
    async fn test_router_select_with_preference() {
        let config = RouterConfig {
            preference: RoutingPreference::Fast,
            ..Default::default()
        };
        let router = ModelRouter::with_config(config);

        let context = TaskContext::new(AgentType::Coder).with_complexity(TaskComplexity::Simple);

        // Run multiple selections and track fast vs slow model counts
        let mut fast_count = 0;
        let mut slow_count = 0;
        for _ in 0..100 {
            let decision = router.select(&context).await.unwrap();
            // Speed tier 5 models: haiku, mini
            // Speed tier 2: opus
            if decision.model_id.contains("haiku") || decision.model_id.contains("mini") {
                fast_count += 1;
            } else if decision.model_id.contains("opus") {
                slow_count += 1;
            }
        }

        // Fast preference should favor faster models more than slow ones
        // This is more forgiving but still validates the preference works
        assert!(
            fast_count > slow_count,
            "Expected fast models to be preferred over slow ones, got fast={} slow={}",
            fast_count,
            slow_count
        );
    }

    #[tokio::test]
    async fn test_router_record_outcome() {
        let router = ModelRouter::new();

        let context = TaskContext::new(AgentType::Coder);
        let decision = router.select(&context).await.unwrap();

        // Record a successful outcome
        let reward = RoutingReward::new(context.routing_key(), decision.model_id.clone(), true)
            .with_cost(0.01)
            .with_latency(500)
            .with_quality(0.9);

        router.record_outcome(reward).await.unwrap();

        // Verify stats were updated
        let stats = router.get_stats(&context.routing_key()).await;
        assert!(stats.is_some());

        let model_stats = stats.unwrap();
        assert!(model_stats.contains_key(&decision.model_id));
    }

    #[tokio::test]
    async fn test_router_learning() {
        let router = ModelRouter::new();
        let context = TaskContext::new(AgentType::Coder).with_complexity(TaskComplexity::Simple);

        // Record many good outcomes for haiku
        for _ in 0..50 {
            let reward = RoutingReward::new(
                context.routing_key(),
                "anthropic/claude-3-5-haiku-latest".to_string(),
                true,
            )
            .with_cost(0.001)
            .with_latency(300)
            .with_quality(0.95);

            router.record_outcome(reward).await.unwrap();
        }

        // Record poor outcomes for sonnet
        for _ in 0..50 {
            let reward = RoutingReward::new(
                context.routing_key(),
                "anthropic/claude-sonnet-4-20250514".to_string(),
                false,
            )
            .with_cost(0.05)
            .with_latency(2000);

            router.record_outcome(reward).await.unwrap();
        }

        // Verify the expected values reflect our training
        let expected = router.get_expected_values(&context.routing_key()).await;
        let haiku_expected = expected
            .get("anthropic/claude-3-5-haiku-latest")
            .unwrap_or(&0.0);
        let sonnet_expected = expected
            .get("anthropic/claude-sonnet-4-20250514")
            .unwrap_or(&1.0);

        // Haiku should have higher expected value than sonnet after training
        assert!(
            haiku_expected > sonnet_expected,
            "Expected haiku to have higher expected value than sonnet after training, haiku={:.3} sonnet={:.3}",
            haiku_expected,
            sonnet_expected
        );

        // Now haiku should be preferred more often than sonnet
        let mut haiku_count = 0;
        let mut sonnet_count = 0;
        for _ in 0..50 {
            let decision = router.select(&context).await.unwrap();
            if decision.model_id.contains("haiku") {
                haiku_count += 1;
            } else if decision.model_id.contains("sonnet") {
                sonnet_count += 1;
            }
        }

        assert!(
            haiku_count > sonnet_count,
            "Expected haiku to be selected more often than sonnet after training, haiku={} sonnet={}",
            haiku_count,
            sonnet_count
        );
    }

    #[tokio::test]
    async fn test_router_budget_constraint() {
        let tracker = Arc::new(CostTracker::new(0.001, 0.8)); // Very tight budget

        // Use up almost all the budget
        tracker.record(
            "anthropic/claude-sonnet-4-20250514",
            crate::cost::TokenUsage::new(300, 100),
            None,
        );

        let router = ModelRouter::new().with_cost_tracker(tracker);

        let context = TaskContext::new(AgentType::Coder);
        let decision = router.select(&context).await.unwrap();

        // Should select a cheaper model due to budget constraint
        assert!(
            decision.model_id.contains("haiku") || decision.model_id.contains("mini"),
            "Expected cheap model due to budget, got {}",
            decision.model_id
        );
    }

    #[tokio::test]
    async fn test_router_complex_task() {
        let config = RouterConfig {
            preference: RoutingPreference::Quality,
            ..Default::default()
        };
        let router = ModelRouter::with_config(config);

        let context = TaskContext::new(AgentType::Planner)
            .with_complexity(TaskComplexity::Expert)
            .with_requires_accuracy(true);

        // Quality preference + expert task should favor high-quality models
        let mut quality_count = 0;
        for _ in 0..30 {
            let decision = router.select(&context).await.unwrap();
            if decision.model_id.contains("opus")
                || decision.model_id.contains("sonnet")
                || decision.model_id.contains("gpt-4o")
            {
                quality_count += 1;
            }
        }

        assert!(
            quality_count > 15,
            "Expected quality models for expert task, got {} out of 30",
            quality_count
        );
    }

    #[tokio::test]
    async fn test_router_builder() {
        let router = ModelRouterBuilder::new()
            .preference(RoutingPreference::Cost)
            .build();

        assert_eq!(router.config().preference, RoutingPreference::Cost);
    }

    #[tokio::test]
    async fn test_get_expected_values() {
        let router = ModelRouter::new();
        let context = TaskContext::new(AgentType::Coder);

        // Record some outcomes to populate stats
        let reward = RoutingReward::new(
            context.routing_key(),
            "anthropic/claude-sonnet-4-20250514".to_string(),
            true,
        );
        router.record_outcome(reward).await.unwrap();

        let values = router.get_expected_values(&context.routing_key()).await;
        assert!(!values.is_empty());
    }
}
