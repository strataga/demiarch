//! Thompson Sampling Multi-Armed Bandit for model selection
//!
//! This module implements Thompson Sampling, a Bayesian approach to the
//! explore/exploit dilemma in reinforcement learning. Each model (arm)
//! maintains a Beta distribution over its expected reward, and we sample
//! from these distributions to make selection decisions.
//!
//! ## Algorithm Overview
//!
//! 1. For each model, maintain Beta(α, β) distribution of success probability
//! 2. Sample from each model's distribution: θ ~ Beta(α, β)
//! 3. Select the model with highest sampled value
//! 4. Observe reward and update: α += reward, β += (1 - reward)
//!
//! This naturally balances exploration (uncertain arms get explored due to
//! high variance in samples) and exploitation (arms with high expected
//! value are more likely to have high samples).

use rand::prelude::*;
use rand_distr::{Beta, Distribution};
use std::collections::HashMap;

use super::types::{ModelCandidate, ModelStats, RoutingPreference};

/// Thompson Sampling bandit for model selection
#[derive(Debug)]
pub struct ThompsonSamplingBandit {
    /// Statistics for each (routing_key, model_id) pair
    stats: HashMap<String, HashMap<String, ModelStats>>,
    /// Exploration bonus factor (higher = more exploration)
    exploration_factor: f64,
    /// Minimum samples before we trust the statistics
    min_samples_for_trust: u64,
    /// Random number generator
    rng: StdRng,
}

impl Default for ThompsonSamplingBandit {
    fn default() -> Self {
        Self::new()
    }
}

impl ThompsonSamplingBandit {
    /// Create a new bandit with default parameters
    pub fn new() -> Self {
        Self {
            stats: HashMap::new(),
            exploration_factor: 1.0,
            min_samples_for_trust: 5,
            rng: StdRng::from_entropy(),
        }
    }

    /// Create a bandit with a fixed seed (for reproducibility in tests)
    pub fn with_seed(seed: u64) -> Self {
        Self {
            stats: HashMap::new(),
            exploration_factor: 1.0,
            min_samples_for_trust: 5,
            rng: StdRng::seed_from_u64(seed),
        }
    }

    /// Set the exploration factor
    pub fn with_exploration_factor(mut self, factor: f64) -> Self {
        self.exploration_factor = factor.max(0.1);
        self
    }

    /// Set minimum samples for trust
    pub fn with_min_samples(mut self, min_samples: u64) -> Self {
        self.min_samples_for_trust = min_samples;
        self
    }

    /// Initialize or get stats for a (routing_key, model) pair
    pub fn get_or_create_stats(
        &mut self,
        routing_key: &str,
        model: &ModelCandidate,
    ) -> &mut ModelStats {
        let model_stats = self.stats.entry(routing_key.to_string()).or_default();

        model_stats
            .entry(model.model_id.clone())
            .or_insert_with(|| {
                // Use model quality tier as prior
                let quality_prior = model.quality_tier as f64 / 5.0;
                ModelStats::new(routing_key.to_string(), model.model_id.clone())
                    .with_prior(quality_prior)
            })
    }

    /// Get stats for a (routing_key, model) pair if it exists
    pub fn get_stats(&self, routing_key: &str, model_id: &str) -> Option<&ModelStats> {
        self.stats.get(routing_key).and_then(|m| m.get(model_id))
    }

    /// Sample from the Beta distribution for a model
    fn sample_beta(&mut self, alpha: f64, beta: f64) -> f64 {
        // Handle edge cases
        if alpha <= 0.0 || beta <= 0.0 {
            return 0.5;
        }

        // Beta distribution
        match Beta::new(alpha, beta) {
            Ok(dist) => dist.sample(&mut self.rng),
            Err(_) => 0.5, // Fallback to mean of uniform
        }
    }

    /// Select the best model using Thompson Sampling
    ///
    /// Returns (selected_model_id, sampled_value, is_exploration)
    pub fn select<'a>(
        &mut self,
        routing_key: &str,
        candidates: &[&'a ModelCandidate],
        preference: RoutingPreference,
    ) -> Option<(&'a ModelCandidate, f64, bool)> {
        if candidates.is_empty() {
            return None;
        }

        // If only one candidate, return it
        if candidates.len() == 1 {
            return Some((candidates[0], 1.0, false));
        }

        let mut best_model: Option<&ModelCandidate> = None;
        let mut best_sample = f64::NEG_INFINITY;
        let mut is_exploration = false;

        for &candidate in candidates {
            // Get or create stats (we need to clone the key since we can't borrow self mutably)
            let (alpha, beta, total_uses) = {
                let stats = self.get_or_create_stats(routing_key, candidate);
                (stats.alpha, stats.beta, stats.total_uses)
            };

            // Apply exploration bonus
            let adjusted_alpha = alpha * self.exploration_factor;
            let adjusted_beta = beta * self.exploration_factor;

            // Sample from Beta distribution
            let mut sample = self.sample_beta(adjusted_alpha, adjusted_beta);

            // Apply preference-based adjustments
            sample = self.apply_preference_adjustment(sample, candidate, preference);

            // Track if this is likely an exploration choice
            let uncertainty =
                ((alpha * beta) / ((alpha + beta).powi(2) * (alpha + beta + 1.0))).sqrt();
            let is_high_uncertainty = uncertainty > 0.15 || total_uses < self.min_samples_for_trust;

            if sample > best_sample {
                best_sample = sample;
                best_model = Some(candidate);
                is_exploration = is_high_uncertainty;
            }
        }

        best_model.map(|m| (m, best_sample, is_exploration))
    }

    /// Apply preference-based adjustments to the sampled value
    fn apply_preference_adjustment(
        &self,
        sample: f64,
        candidate: &ModelCandidate,
        preference: RoutingPreference,
    ) -> f64 {
        match preference {
            RoutingPreference::Balanced => sample,
            RoutingPreference::Fast => {
                // Boost fast models (stronger bonus for tier 5)
                let speed_bonus = (candidate.speed_tier as f64 / 5.0).powi(2) * 0.4;
                sample + speed_bonus
            }
            RoutingPreference::Quality => {
                // Boost high-quality models (stronger bonus for tier 5)
                let quality_bonus = (candidate.quality_tier as f64 / 5.0).powi(2) * 0.4;
                sample + quality_bonus
            }
            RoutingPreference::Cost => {
                // Boost cheap models (inverse of cost)
                // Normalize: cheap models (< $1/M) get bonus, expensive (> $10/M) get penalty
                let cost_factor = 1.0 / (1.0 + candidate.input_cost_per_million / 5.0);
                let cost_bonus = cost_factor * 0.4;
                sample + cost_bonus
            }
        }
    }

    /// Update the bandit with observed reward
    ///
    /// Creates stats for the model if they don't exist yet.
    pub fn update(&mut self, routing_key: &str, model_id: &str, reward: f64) {
        let clamped_reward = reward.clamp(0.0, 1.0);

        // Get or create the routing key entry
        let model_stats = self.stats.entry(routing_key.to_string()).or_default();

        // Get or create the model stats entry
        let stats = model_stats
            .entry(model_id.to_string())
            .or_insert_with(|| ModelStats::new(routing_key.to_string(), model_id.to_string()));

        // Update Beta distribution parameters
        stats.alpha += clamped_reward;
        stats.beta += 1.0 - clamped_reward;

        // Update summary statistics
        stats.total_uses += 1;
        if clamped_reward > 0.5 {
            stats.successes += 1;
        } else {
            stats.failures += 1;
        }
        stats.reward_sum += clamped_reward;
        stats.reward_sum_sq += clamped_reward * clamped_reward;
    }

    /// Get the expected values for all models for a routing key
    pub fn expected_values(&self, routing_key: &str) -> HashMap<String, f64> {
        self.stats
            .get(routing_key)
            .map(|model_stats| {
                model_stats
                    .iter()
                    .map(|(model_id, stats)| (model_id.clone(), stats.expected_value()))
                    .collect()
            })
            .unwrap_or_default()
    }

    /// Get all statistics
    pub fn all_stats(&self) -> &HashMap<String, HashMap<String, ModelStats>> {
        &self.stats
    }

    /// Import statistics (e.g., from persistence)
    pub fn import_stats(&mut self, stats: HashMap<String, HashMap<String, ModelStats>>) {
        self.stats = stats;
    }

    /// Get statistics for a specific routing key
    pub fn stats_for_key(&self, routing_key: &str) -> Option<&HashMap<String, ModelStats>> {
        self.stats.get(routing_key)
    }

    /// Reset all statistics (useful for testing)
    pub fn reset(&mut self) {
        self.stats.clear();
    }

    /// Compute Upper Confidence Bound (UCB) for a model
    ///
    /// UCB = mean + c * sqrt(ln(N) / n)
    /// where N is total selections across all models, n is this model's selections
    pub fn ucb(&self, routing_key: &str, model_id: &str, total_selections: u64) -> f64 {
        let c = 2.0_f64.sqrt(); // Exploration constant

        if let Some(stats) = self.get_stats(routing_key, model_id) {
            if stats.total_uses == 0 {
                return f64::INFINITY; // Unexplored arm gets infinite UCB
            }
            let mean = stats.expected_value();
            let exploration_term =
                c * ((total_selections as f64).ln() / stats.total_uses as f64).sqrt();
            mean + exploration_term
        } else {
            f64::INFINITY // Unknown model gets explored
        }
    }
}

/// Result of a bandit selection (for future use in detailed analytics)
#[derive(Debug, Clone)]
#[allow(dead_code)]
pub struct BanditSelection {
    /// Selected model ID
    pub model_id: String,
    /// Sampled value that led to selection
    pub sampled_value: f64,
    /// Whether this was an exploration choice
    pub is_exploration: bool,
    /// Expected value (mean of Beta distribution)
    pub expected_value: f64,
    /// Uncertainty (std dev of Beta distribution)
    pub uncertainty: f64,
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_candidates() -> Vec<ModelCandidate> {
        vec![
            ModelCandidate::new("model-a")
                .with_quality_tier(4)
                .with_speed_tier(3)
                .with_pricing(3.0, 15.0),
            ModelCandidate::new("model-b")
                .with_quality_tier(3)
                .with_speed_tier(5)
                .with_pricing(0.5, 2.0),
            ModelCandidate::new("model-c")
                .with_quality_tier(5)
                .with_speed_tier(2)
                .with_pricing(15.0, 75.0),
        ]
    }

    #[test]
    fn test_bandit_new() {
        let bandit = ThompsonSamplingBandit::new();
        assert!(bandit.stats.is_empty());
    }

    #[test]
    fn test_bandit_select_single_candidate() {
        let mut bandit = ThompsonSamplingBandit::with_seed(42);
        let candidates = vec![ModelCandidate::new("only-model")];
        let refs: Vec<&ModelCandidate> = candidates.iter().collect();

        let result = bandit.select("test:simple", &refs, RoutingPreference::Balanced);
        assert!(result.is_some());

        let (model, _, _) = result.unwrap();
        assert_eq!(model.model_id, "only-model");
    }

    #[test]
    fn test_bandit_select_multiple_candidates() {
        let mut bandit = ThompsonSamplingBandit::with_seed(42);
        let candidates = test_candidates();
        let refs: Vec<&ModelCandidate> = candidates.iter().collect();

        // Run multiple selections to verify it's working
        let mut selections: HashMap<String, u32> = HashMap::new();

        for _ in 0..100 {
            if let Some((model, _, _)) =
                bandit.select("test:simple", &refs, RoutingPreference::Balanced)
            {
                *selections.entry(model.model_id.clone()).or_default() += 1;
            }
        }

        // All models should be selected at least once (exploration)
        assert!(
            selections.len() >= 2,
            "Expected exploration of multiple models"
        );
    }

    #[test]
    fn test_bandit_update_affects_selection() {
        let mut bandit = ThompsonSamplingBandit::with_seed(42);
        let candidates = test_candidates();
        let refs: Vec<&ModelCandidate> = candidates.iter().collect();

        // Initialize stats
        for candidate in &candidates {
            bandit.get_or_create_stats("test:simple", candidate);
        }

        // Strongly reward model-a
        for _ in 0..20 {
            bandit.update("test:simple", "model-a", 1.0);
        }

        // Strongly penalize model-b
        for _ in 0..20 {
            bandit.update("test:simple", "model-b", 0.0);
        }

        // Now model-a should be selected much more often
        let mut a_count = 0;
        for _ in 0..50 {
            if let Some((model, _, _)) =
                bandit.select("test:simple", &refs, RoutingPreference::Balanced)
            {
                if model.model_id == "model-a" {
                    a_count += 1;
                }
            }
        }

        assert!(
            a_count > 30,
            "Expected model-a to be selected most often, got {} out of 50",
            a_count
        );
    }

    #[test]
    fn test_bandit_preference_fast() {
        let mut bandit = ThompsonSamplingBandit::with_seed(42);
        let candidates = test_candidates();
        let refs: Vec<&ModelCandidate> = candidates.iter().collect();

        // Initialize with equal stats
        for candidate in &candidates {
            let stats = bandit.get_or_create_stats("test:simple", candidate);
            stats.alpha = 10.0;
            stats.beta = 10.0;
        }

        // With FAST preference, model-b (speed_tier=5) should be favored
        let mut b_count = 0;
        for _ in 0..100 {
            if let Some((model, _, _)) =
                bandit.select("test:simple", &refs, RoutingPreference::Fast)
            {
                if model.model_id == "model-b" {
                    b_count += 1;
                }
            }
        }

        assert!(
            b_count > 40,
            "Expected fast model-b to be preferred, got {} out of 100",
            b_count
        );
    }

    #[test]
    fn test_bandit_preference_quality() {
        let mut bandit = ThompsonSamplingBandit::with_seed(42);
        let candidates = test_candidates();
        let refs: Vec<&ModelCandidate> = candidates.iter().collect();

        // Initialize with equal stats
        for candidate in &candidates {
            let stats = bandit.get_or_create_stats("test:simple", candidate);
            stats.alpha = 10.0;
            stats.beta = 10.0;
        }

        // With QUALITY preference, model-c (quality_tier=5) should be favored
        let mut c_count = 0;
        for _ in 0..100 {
            if let Some((model, _, _)) =
                bandit.select("test:simple", &refs, RoutingPreference::Quality)
            {
                if model.model_id == "model-c" {
                    c_count += 1;
                }
            }
        }

        assert!(
            c_count > 40,
            "Expected quality model-c to be preferred, got {} out of 100",
            c_count
        );
    }

    #[test]
    fn test_bandit_preference_cost() {
        let mut bandit = ThompsonSamplingBandit::with_seed(42);
        let candidates = test_candidates();
        let refs: Vec<&ModelCandidate> = candidates.iter().collect();

        // Initialize with equal stats
        for candidate in &candidates {
            let stats = bandit.get_or_create_stats("test:simple", candidate);
            stats.alpha = 10.0;
            stats.beta = 10.0;
        }

        // With COST preference, model-b (cheapest) should be favored
        let mut b_count = 0;
        for _ in 0..100 {
            if let Some((model, _, _)) =
                bandit.select("test:simple", &refs, RoutingPreference::Cost)
            {
                if model.model_id == "model-b" {
                    b_count += 1;
                }
            }
        }

        assert!(
            b_count > 40,
            "Expected cheap model-b to be preferred, got {} out of 100",
            b_count
        );
    }

    #[test]
    fn test_expected_values() {
        let mut bandit = ThompsonSamplingBandit::with_seed(42);
        let candidates = test_candidates();

        // Initialize stats
        for candidate in &candidates {
            bandit.get_or_create_stats("test:simple", candidate);
        }

        // Update with different rewards
        bandit.update("test:simple", "model-a", 0.9);
        bandit.update("test:simple", "model-b", 0.5);
        bandit.update("test:simple", "model-c", 0.3);

        let expected = bandit.expected_values("test:simple");

        // model-a should have highest expected value
        assert!(expected.get("model-a").unwrap() > expected.get("model-b").unwrap());
    }

    #[test]
    fn test_ucb() {
        let mut bandit = ThompsonSamplingBandit::with_seed(42);
        let candidates = test_candidates();

        // Initialize stats
        for candidate in &candidates {
            bandit.get_or_create_stats("test:simple", candidate);
        }

        // Update model-a more times
        for _ in 0..10 {
            bandit.update("test:simple", "model-a", 0.8);
        }

        // UCB for unexplored model should be infinite
        let ucb_unknown = bandit.ucb("test:simple", "unknown-model", 100);
        assert!(ucb_unknown.is_infinite());

        // UCB for explored model should be finite
        let ucb_a = bandit.ucb("test:simple", "model-a", 100);
        assert!(ucb_a.is_finite());
    }

    #[test]
    fn test_import_stats() {
        let mut bandit = ThompsonSamplingBandit::new();

        let mut imported: HashMap<String, HashMap<String, ModelStats>> = HashMap::new();
        let mut model_stats: HashMap<String, ModelStats> = HashMap::new();

        let mut stats = ModelStats::new("test:simple".to_string(), "model-a".to_string());
        stats.alpha = 10.0;
        stats.beta = 5.0;
        stats.total_uses = 15;
        model_stats.insert("model-a".to_string(), stats);

        imported.insert("test:simple".to_string(), model_stats);

        bandit.import_stats(imported);

        let retrieved = bandit.get_stats("test:simple", "model-a");
        assert!(retrieved.is_some());
        assert_eq!(retrieved.unwrap().alpha, 10.0);
    }
}
