//! Types for dynamic model routing with reinforcement learning
//!
//! This module defines the core types used by the model router to make
//! intelligent decisions about which LLM to use for each task.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use crate::agents::AgentType;

/// Context about a task to inform model selection
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskContext {
    /// Type of agent making the request
    pub agent_type: AgentType,
    /// Classification of the task complexity
    pub complexity: TaskComplexity,
    /// Whether this task requires high accuracy (e.g., code generation)
    pub requires_accuracy: bool,
    /// Whether speed is prioritized over quality
    pub prioritize_speed: bool,
    /// Estimated input token count
    pub estimated_input_tokens: usize,
    /// Maximum allowed cost for this task (optional budget constraint)
    pub max_cost_usd: Option<f64>,
    /// Tags for more specific task classification
    pub tags: Vec<String>,
}

impl TaskContext {
    /// Create a new task context with defaults
    pub fn new(agent_type: AgentType) -> Self {
        Self {
            agent_type,
            complexity: TaskComplexity::Medium,
            requires_accuracy: true,
            prioritize_speed: false,
            estimated_input_tokens: 1000,
            max_cost_usd: None,
            tags: Vec::new(),
        }
    }

    /// Set complexity level
    pub fn with_complexity(mut self, complexity: TaskComplexity) -> Self {
        self.complexity = complexity;
        self
    }

    /// Set accuracy requirement
    pub fn with_requires_accuracy(mut self, requires: bool) -> Self {
        self.requires_accuracy = requires;
        self
    }

    /// Set speed priority
    pub fn with_prioritize_speed(mut self, prioritize: bool) -> Self {
        self.prioritize_speed = prioritize;
        self
    }

    /// Set estimated input tokens
    pub fn with_estimated_tokens(mut self, tokens: usize) -> Self {
        self.estimated_input_tokens = tokens;
        self
    }

    /// Set maximum cost constraint
    pub fn with_max_cost(mut self, max_cost: f64) -> Self {
        self.max_cost_usd = Some(max_cost);
        self
    }

    /// Add tags for task classification
    pub fn with_tags(mut self, tags: Vec<String>) -> Self {
        self.tags = tags;
        self
    }

    /// Get a routing key for this context (used to track statistics)
    pub fn routing_key(&self) -> String {
        format!("{}:{}", self.agent_type, self.complexity)
    }
}

/// Classification of task complexity
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TaskComplexity {
    /// Simple tasks: formatting, simple edits, summaries
    Simple,
    /// Medium tasks: standard code generation, reviews
    Medium,
    /// Complex tasks: architecture decisions, multi-file refactoring
    Complex,
    /// Expert tasks: security analysis, performance optimization
    Expert,
}

impl std::fmt::Display for TaskComplexity {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Simple => write!(f, "simple"),
            Self::Medium => write!(f, "medium"),
            Self::Complex => write!(f, "complex"),
            Self::Expert => write!(f, "expert"),
        }
    }
}

impl std::str::FromStr for TaskComplexity {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "simple" => Ok(Self::Simple),
            "medium" => Ok(Self::Medium),
            "complex" => Ok(Self::Complex),
            "expert" => Ok(Self::Expert),
            _ => Err(format!("Unknown complexity level: {}", s)),
        }
    }
}

/// A candidate model for selection
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelCandidate {
    /// Model identifier (e.g., "anthropic/claude-sonnet-4-20250514")
    pub model_id: String,
    /// Cost per million input tokens
    pub input_cost_per_million: f64,
    /// Cost per million output tokens
    pub output_cost_per_million: f64,
    /// Maximum context window size
    pub context_window: usize,
    /// Whether this model is suitable for complex reasoning
    pub supports_complex_reasoning: bool,
    /// Whether this model excels at code generation
    pub optimized_for_code: bool,
    /// Base quality tier (1-5, higher is better)
    pub quality_tier: u8,
    /// Relative speed tier (1-5, higher is faster)
    pub speed_tier: u8,
}

impl ModelCandidate {
    /// Create a new model candidate
    pub fn new(model_id: impl Into<String>) -> Self {
        Self {
            model_id: model_id.into(),
            input_cost_per_million: 1.0,
            output_cost_per_million: 5.0,
            context_window: 128_000,
            supports_complex_reasoning: true,
            optimized_for_code: true,
            quality_tier: 3,
            speed_tier: 3,
        }
    }

    /// Set pricing
    pub fn with_pricing(mut self, input: f64, output: f64) -> Self {
        self.input_cost_per_million = input;
        self.output_cost_per_million = output;
        self
    }

    /// Set context window
    pub fn with_context_window(mut self, size: usize) -> Self {
        self.context_window = size;
        self
    }

    /// Set capabilities
    pub fn with_capabilities(mut self, complex_reasoning: bool, code_optimized: bool) -> Self {
        self.supports_complex_reasoning = complex_reasoning;
        self.optimized_for_code = code_optimized;
        self
    }

    /// Set quality tier
    pub fn with_quality_tier(mut self, tier: u8) -> Self {
        self.quality_tier = tier.min(5);
        self
    }

    /// Set speed tier
    pub fn with_speed_tier(mut self, tier: u8) -> Self {
        self.speed_tier = tier.min(5);
        self
    }

    /// Estimate cost for a given token count
    pub fn estimate_cost(&self, input_tokens: usize, output_tokens: usize) -> f64 {
        let input_cost = (input_tokens as f64 / 1_000_000.0) * self.input_cost_per_million;
        let output_cost = (output_tokens as f64 / 1_000_000.0) * self.output_cost_per_million;
        input_cost + output_cost
    }
}

/// Result of a routing decision
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RoutingDecision {
    /// Selected model ID
    pub model_id: String,
    /// Confidence score (0.0 to 1.0)
    pub confidence: f64,
    /// Reason for selection
    pub reason: RoutingReason,
    /// Alternative models considered
    pub alternatives: Vec<String>,
    /// Whether this was an exploration (vs exploitation) choice
    pub is_exploration: bool,
}

impl RoutingDecision {
    /// Create a new routing decision
    pub fn new(model_id: impl Into<String>, confidence: f64, reason: RoutingReason) -> Self {
        Self {
            model_id: model_id.into(),
            confidence,
            reason,
            alternatives: Vec::new(),
            is_exploration: false,
        }
    }

    /// Add alternative models
    pub fn with_alternatives(mut self, alternatives: Vec<String>) -> Self {
        self.alternatives = alternatives;
        self
    }

    /// Mark as exploration choice
    pub fn as_exploration(mut self) -> Self {
        self.is_exploration = true;
        self
    }
}

/// Reason for a routing decision
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RoutingReason {
    /// Selected based on historical performance (Thompson sampling)
    Performance { expected_reward: f64 },
    /// Selected due to cost constraints
    CostConstraint { within_budget: f64 },
    /// Selected for quality on complex task
    QualityRequired,
    /// Selected for speed on time-sensitive task
    SpeedRequired,
    /// Exploration to gather more data
    Exploration { uncertainty: f64 },
    /// Default fallback selection
    Default,
    /// No suitable model found, using best available
    FallbackOnly,
}

/// Reward signal from a completed task
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RoutingReward {
    /// The routing key (task type identifier)
    pub routing_key: String,
    /// The model that was used
    pub model_id: String,
    /// Whether the task succeeded
    pub success: bool,
    /// Actual cost in USD
    pub actual_cost_usd: f64,
    /// Latency in milliseconds
    pub latency_ms: u64,
    /// Quality score (0.0 to 1.0, optional - from reviews/tests)
    pub quality_score: Option<f64>,
    /// Token efficiency (output tokens / input tokens)
    pub token_efficiency: f64,
}

impl RoutingReward {
    /// Create a new reward signal
    pub fn new(routing_key: String, model_id: String, success: bool) -> Self {
        Self {
            routing_key,
            model_id,
            success,
            actual_cost_usd: 0.0,
            latency_ms: 0,
            quality_score: None,
            token_efficiency: 1.0,
        }
    }

    /// Set cost
    pub fn with_cost(mut self, cost: f64) -> Self {
        self.actual_cost_usd = cost;
        self
    }

    /// Set latency
    pub fn with_latency(mut self, ms: u64) -> Self {
        self.latency_ms = ms;
        self
    }

    /// Set quality score
    pub fn with_quality(mut self, score: f64) -> Self {
        self.quality_score = Some(score.clamp(0.0, 1.0));
        self
    }

    /// Set token efficiency
    pub fn with_token_efficiency(mut self, efficiency: f64) -> Self {
        self.token_efficiency = efficiency;
        self
    }

    /// Compute overall reward value (0.0 to 1.0)
    ///
    /// Reward combines:
    /// - Success (40% weight)
    /// - Quality (30% weight)
    /// - Cost efficiency (20% weight)
    /// - Speed (10% weight)
    pub fn compute_reward(&self, expected_cost: f64, expected_latency_ms: u64) -> f64 {
        // Base success reward
        let success_reward = if self.success { 1.0 } else { 0.0 };

        // Quality reward (use 0.5 as default if not provided)
        let quality_reward = self.quality_score.unwrap_or(0.5);

        // Cost efficiency: ratio of expected to actual (capped at 2x for very cheap)
        let cost_efficiency = if self.actual_cost_usd > 0.0 {
            (expected_cost / self.actual_cost_usd).min(2.0) / 2.0
        } else {
            1.0
        };

        // Speed efficiency: ratio of expected to actual (capped at 2x for very fast)
        let speed_efficiency = if self.latency_ms > 0 {
            (expected_latency_ms as f64 / self.latency_ms as f64).min(2.0) / 2.0
        } else {
            1.0
        };

        // Weighted combination
        let reward = (success_reward * 0.4)
            + (quality_reward * 0.3)
            + (cost_efficiency * 0.2)
            + (speed_efficiency * 0.1);

        reward.clamp(0.0, 1.0)
    }
}

/// Statistics for a (task_type, model) pair
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelStats {
    /// The routing key (task type)
    pub routing_key: String,
    /// The model ID
    pub model_id: String,
    /// Total number of uses
    pub total_uses: u64,
    /// Number of successes
    pub successes: u64,
    /// Number of failures
    pub failures: u64,
    /// Sum of rewards (for computing average)
    pub reward_sum: f64,
    /// Sum of squared rewards (for variance calculation)
    pub reward_sum_sq: f64,
    /// Average cost in USD
    pub avg_cost_usd: f64,
    /// Average latency in ms
    pub avg_latency_ms: f64,
    /// Beta distribution alpha parameter (for Thompson sampling)
    pub alpha: f64,
    /// Beta distribution beta parameter (for Thompson sampling)
    pub beta: f64,
}

impl ModelStats {
    /// Create new stats with prior
    pub fn new(routing_key: String, model_id: String) -> Self {
        Self {
            routing_key,
            model_id,
            total_uses: 0,
            successes: 0,
            failures: 0,
            reward_sum: 0.0,
            reward_sum_sq: 0.0,
            avg_cost_usd: 0.0,
            avg_latency_ms: 0.0,
            // Uninformed prior (Beta(1,1) = uniform distribution)
            alpha: 1.0,
            beta: 1.0,
        }
    }

    /// Create stats with an informed prior (e.g., from model quality tier)
    pub fn with_prior(mut self, quality_prior: f64) -> Self {
        // Higher quality prior = more initial successes assumed
        // E.g., quality 0.8 -> alpha=4, beta=1 (expected success rate ~0.8)
        self.alpha = 1.0 + (quality_prior * 5.0);
        self.beta = 1.0 + ((1.0 - quality_prior) * 5.0);
        self
    }

    /// Update stats with a new observation
    pub fn update(&mut self, reward: &RoutingReward, computed_reward: f64) {
        self.total_uses += 1;

        if reward.success {
            self.successes += 1;
        } else {
            self.failures += 1;
        }

        // Update reward statistics
        self.reward_sum += computed_reward;
        self.reward_sum_sq += computed_reward * computed_reward;

        // Update averages using online mean formula
        let n = self.total_uses as f64;
        self.avg_cost_usd += (reward.actual_cost_usd - self.avg_cost_usd) / n;
        self.avg_latency_ms += (reward.latency_ms as f64 - self.avg_latency_ms) / n;

        // Update Beta distribution parameters
        // Using the computed reward as a "soft" success indicator
        self.alpha += computed_reward;
        self.beta += 1.0 - computed_reward;
    }

    /// Get the mean reward
    pub fn mean_reward(&self) -> f64 {
        if self.total_uses == 0 {
            return 0.5; // Prior expectation
        }
        self.reward_sum / self.total_uses as f64
    }

    /// Get the reward variance
    pub fn reward_variance(&self) -> f64 {
        if self.total_uses < 2 {
            return 0.25; // Prior variance (uniform)
        }
        let n = self.total_uses as f64;
        let mean = self.mean_reward();
        (self.reward_sum_sq / n) - (mean * mean)
    }

    /// Get the success rate
    pub fn success_rate(&self) -> f64 {
        if self.total_uses == 0 {
            return 0.5;
        }
        self.successes as f64 / self.total_uses as f64
    }

    /// Get the expected value from Beta distribution (Thompson sampling)
    pub fn expected_value(&self) -> f64 {
        self.alpha / (self.alpha + self.beta)
    }

    /// Get uncertainty (standard deviation of Beta distribution)
    pub fn uncertainty(&self) -> f64 {
        let ab = self.alpha + self.beta;
        ((self.alpha * self.beta) / (ab * ab * (ab + 1.0))).sqrt()
    }
}

/// Configuration for the routing preference
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RoutingPreference {
    /// Balance cost, quality, and speed
    #[default]
    Balanced,
    /// Prioritize speed over quality
    Fast,
    /// Prioritize quality over cost
    Quality,
    /// Prioritize cost savings
    Cost,
}

impl std::str::FromStr for RoutingPreference {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "balanced" => Ok(Self::Balanced),
            "fast" => Ok(Self::Fast),
            "quality" => Ok(Self::Quality),
            "cost" => Ok(Self::Cost),
            _ => Err(format!("Unknown routing preference: {}", s)),
        }
    }
}

impl std::fmt::Display for RoutingPreference {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Balanced => write!(f, "balanced"),
            Self::Fast => write!(f, "fast"),
            Self::Quality => write!(f, "quality"),
            Self::Cost => write!(f, "cost"),
        }
    }
}

/// Registry of available models with their capabilities
#[derive(Debug, Clone, Default)]
pub struct ModelRegistry {
    models: HashMap<String, ModelCandidate>,
}

impl ModelRegistry {
    /// Create a new empty registry
    pub fn new() -> Self {
        Self {
            models: HashMap::new(),
        }
    }

    /// Create a registry with default models
    pub fn with_defaults() -> Self {
        let mut registry = Self::new();

        // Claude Sonnet 4 - balanced quality/cost
        registry.register(
            ModelCandidate::new("anthropic/claude-sonnet-4-20250514")
                .with_pricing(3.0, 15.0)
                .with_context_window(200_000)
                .with_capabilities(true, true)
                .with_quality_tier(4)
                .with_speed_tier(4),
        );

        // Claude Haiku - fast and cheap
        registry.register(
            ModelCandidate::new("anthropic/claude-3-5-haiku-latest")
                .with_pricing(0.80, 4.0)
                .with_context_window(200_000)
                .with_capabilities(false, true)
                .with_quality_tier(3)
                .with_speed_tier(5),
        );

        // Claude Opus - highest quality
        registry.register(
            ModelCandidate::new("anthropic/claude-opus-4-20250514")
                .with_pricing(15.0, 75.0)
                .with_context_window(200_000)
                .with_capabilities(true, true)
                .with_quality_tier(5)
                .with_speed_tier(2),
        );

        // GPT-4o - general purpose
        registry.register(
            ModelCandidate::new("openai/gpt-4o")
                .with_pricing(2.50, 10.0)
                .with_context_window(128_000)
                .with_capabilities(true, true)
                .with_quality_tier(4)
                .with_speed_tier(4),
        );

        // GPT-4o-mini - fast and cheap
        registry.register(
            ModelCandidate::new("openai/gpt-4o-mini")
                .with_pricing(0.15, 0.60)
                .with_context_window(128_000)
                .with_capabilities(false, true)
                .with_quality_tier(3)
                .with_speed_tier(5),
        );

        registry
    }

    /// Register a model
    pub fn register(&mut self, model: ModelCandidate) {
        self.models.insert(model.model_id.clone(), model);
    }

    /// Get a model by ID
    pub fn get(&self, model_id: &str) -> Option<&ModelCandidate> {
        self.models.get(model_id)
    }

    /// Get all models
    pub fn all(&self) -> impl Iterator<Item = &ModelCandidate> {
        self.models.values()
    }

    /// Get models suitable for a task context
    pub fn candidates_for(&self, context: &TaskContext) -> Vec<&ModelCandidate> {
        self.models
            .values()
            .filter(|m| {
                // Filter out models that can't handle the context size
                if m.context_window < context.estimated_input_tokens * 2 {
                    return false;
                }

                // Filter out models that can't meet cost constraints
                if let Some(max_cost) = context.max_cost_usd {
                    let estimated_cost = m.estimate_cost(
                        context.estimated_input_tokens,
                        context.estimated_input_tokens, // Assume similar output
                    );
                    if estimated_cost > max_cost {
                        return false;
                    }
                }

                // For complex tasks, require complex reasoning support
                if matches!(
                    context.complexity,
                    TaskComplexity::Complex | TaskComplexity::Expert
                ) && context.requires_accuracy
                    && !m.supports_complex_reasoning
                {
                    return false;
                }

                true
            })
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_task_context_routing_key() {
        let context = TaskContext::new(AgentType::Coder).with_complexity(TaskComplexity::Complex);

        assert_eq!(context.routing_key(), "coder:complex");
    }

    #[test]
    fn test_model_candidate_estimate_cost() {
        let model = ModelCandidate::new("test/model").with_pricing(3.0, 15.0);

        let cost = model.estimate_cost(1_000_000, 500_000);
        assert!((cost - 10.5).abs() < 0.001); // 3.0 + 7.5
    }

    #[test]
    fn test_routing_reward_compute() {
        let reward = RoutingReward::new("test:simple".to_string(), "test/model".to_string(), true)
            .with_cost(0.01)
            .with_latency(1000)
            .with_quality(0.9);

        let computed = reward.compute_reward(0.02, 2000); // Expected was 2x actual
        assert!(computed > 0.5); // Should be decent reward
    }

    #[test]
    fn test_model_stats_update() {
        let mut stats = ModelStats::new("test:simple".to_string(), "test/model".to_string());

        let reward = RoutingReward::new("test:simple".to_string(), "test/model".to_string(), true)
            .with_cost(0.01)
            .with_latency(1000);

        stats.update(&reward, 0.8);

        assert_eq!(stats.total_uses, 1);
        assert_eq!(stats.successes, 1);
        assert!((stats.avg_cost_usd - 0.01).abs() < 0.0001);
    }

    #[test]
    fn test_model_registry_candidates() {
        let registry = ModelRegistry::with_defaults();

        // Simple task should allow all models
        let simple_context =
            TaskContext::new(AgentType::Coder).with_complexity(TaskComplexity::Simple);
        let candidates = registry.candidates_for(&simple_context);
        assert!(!candidates.is_empty());

        // Complex task with tight budget should filter expensive models
        let constrained_context = TaskContext::new(AgentType::Planner)
            .with_complexity(TaskComplexity::Complex)
            .with_max_cost(0.001);
        let constrained_candidates = registry.candidates_for(&constrained_context);
        assert!(constrained_candidates.len() < candidates.len());
    }

    #[test]
    fn test_routing_preference_parse() {
        assert_eq!(
            "balanced".parse::<RoutingPreference>().unwrap(),
            RoutingPreference::Balanced
        );
        assert_eq!(
            "FAST".parse::<RoutingPreference>().unwrap(),
            RoutingPreference::Fast
        );
        assert!("unknown".parse::<RoutingPreference>().is_err());
    }
}
