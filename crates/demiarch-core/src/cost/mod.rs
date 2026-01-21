//! Cost management and budget enforcement
//!
//! This module provides:
//! - Token usage tracking per LLM call
//! - Cost calculation based on model pricing
//! - Daily cost aggregation and budget enforcement
//! - Cost history for reporting

use chrono::{DateTime, NaiveDate, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::{Arc, RwLock};

/// Token usage for a single LLM call
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TokenUsage {
    /// Number of input/prompt tokens
    pub input_tokens: u32,
    /// Number of output/completion tokens
    pub output_tokens: u32,
}

impl TokenUsage {
    /// Create a new token usage record
    pub fn new(input_tokens: u32, output_tokens: u32) -> Self {
        Self {
            input_tokens,
            output_tokens,
        }
    }

    /// Total tokens (input + output)
    pub fn total(&self) -> u32 {
        self.input_tokens + self.output_tokens
    }
}

/// Cost breakdown for a single LLM call
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LlmCost {
    /// Unique identifier for this cost record
    pub id: String,
    /// Model identifier (e.g., "anthropic/claude-sonnet-4-20250514")
    pub model: String,
    /// Token usage for this call
    pub tokens: TokenUsage,
    /// Cost for input tokens in USD
    pub input_cost_usd: f64,
    /// Cost for output tokens in USD
    pub output_cost_usd: f64,
    /// Timestamp of the LLM call
    pub timestamp: DateTime<Utc>,
    /// Optional context (e.g., feature ID, agent type)
    pub context: Option<String>,
}

impl LlmCost {
    /// Total cost in USD
    pub fn total_cost_usd(&self) -> f64 {
        self.input_cost_usd + self.output_cost_usd
    }
}

/// Pricing information for a model (per million tokens)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelPricing {
    /// Model identifier
    pub model: String,
    /// Cost per million input tokens in USD
    pub input_price_per_million: f64,
    /// Cost per million output tokens in USD
    pub output_price_per_million: f64,
}

impl ModelPricing {
    /// Create new model pricing
    pub fn new(model: impl Into<String>, input_price: f64, output_price: f64) -> Self {
        Self {
            model: model.into(),
            input_price_per_million: input_price,
            output_price_per_million: output_price,
        }
    }

    /// Calculate cost for given token usage
    pub fn calculate_cost(&self, tokens: &TokenUsage) -> (f64, f64) {
        let input_cost = (tokens.input_tokens as f64 / 1_000_000.0) * self.input_price_per_million;
        let output_cost =
            (tokens.output_tokens as f64 / 1_000_000.0) * self.output_price_per_million;
        (input_cost, output_cost)
    }
}

/// Daily cost summary
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DailyCostSummary {
    /// The date for this summary
    pub date: NaiveDate,
    /// Total cost in USD
    pub total_cost_usd: f64,
    /// Total input tokens
    pub total_input_tokens: u64,
    /// Total output tokens
    pub total_output_tokens: u64,
    /// Number of LLM calls
    pub call_count: u32,
    /// Breakdown by model
    pub by_model: HashMap<String, ModelCostSummary>,
}

impl DailyCostSummary {
    /// Create a new empty daily summary
    pub fn new(date: NaiveDate) -> Self {
        Self {
            date,
            total_cost_usd: 0.0,
            total_input_tokens: 0,
            total_output_tokens: 0,
            call_count: 0,
            by_model: HashMap::new(),
        }
    }

    /// Add a cost record to this summary
    pub fn add(&mut self, cost: &LlmCost) {
        self.total_cost_usd += cost.total_cost_usd();
        self.total_input_tokens += cost.tokens.input_tokens as u64;
        self.total_output_tokens += cost.tokens.output_tokens as u64;
        self.call_count += 1;

        let model_summary = self
            .by_model
            .entry(cost.model.clone())
            .or_insert_with(|| ModelCostSummary::new(cost.model.clone()));
        model_summary.add(cost);
    }
}

/// Cost summary for a specific model
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelCostSummary {
    /// Model identifier
    pub model: String,
    /// Total cost in USD
    pub total_cost_usd: f64,
    /// Total input tokens
    pub total_input_tokens: u64,
    /// Total output tokens
    pub total_output_tokens: u64,
    /// Number of calls
    pub call_count: u32,
}

impl ModelCostSummary {
    /// Create a new empty model summary
    pub fn new(model: String) -> Self {
        Self {
            model,
            total_cost_usd: 0.0,
            total_input_tokens: 0,
            total_output_tokens: 0,
            call_count: 0,
        }
    }

    /// Add a cost record to this summary
    pub fn add(&mut self, cost: &LlmCost) {
        self.total_cost_usd += cost.total_cost_usd();
        self.total_input_tokens += cost.tokens.input_tokens as u64;
        self.total_output_tokens += cost.tokens.output_tokens as u64;
        self.call_count += 1;
    }
}

/// Default pricing table for common OpenRouter models
/// Prices are per million tokens as of January 2025
fn default_pricing_table() -> HashMap<String, ModelPricing> {
    let mut table = HashMap::new();

    // Anthropic models
    table.insert(
        "anthropic/claude-sonnet-4-20250514".to_string(),
        ModelPricing::new("anthropic/claude-sonnet-4-20250514", 3.0, 15.0),
    );
    table.insert(
        "anthropic/claude-3-5-haiku-latest".to_string(),
        ModelPricing::new("anthropic/claude-3-5-haiku-latest", 0.80, 4.0),
    );
    table.insert(
        "anthropic/claude-opus-4-20250514".to_string(),
        ModelPricing::new("anthropic/claude-opus-4-20250514", 15.0, 75.0),
    );

    // OpenAI models
    table.insert(
        "openai/gpt-4o".to_string(),
        ModelPricing::new("openai/gpt-4o", 2.50, 10.0),
    );
    table.insert(
        "openai/gpt-4o-mini".to_string(),
        ModelPricing::new("openai/gpt-4o-mini", 0.15, 0.60),
    );

    table
}

/// Cost tracker for recording and aggregating LLM costs
#[derive(Debug)]
pub struct CostTracker {
    /// Model pricing table
    pricing: HashMap<String, ModelPricing>,
    /// Cost records (in-memory, most recent first)
    records: Arc<RwLock<Vec<LlmCost>>>,
    /// Daily summaries cache
    daily_summaries: Arc<RwLock<HashMap<NaiveDate, DailyCostSummary>>>,
    /// Daily budget limit in USD
    daily_limit_usd: f64,
    /// Alert threshold (0.0 to 1.0)
    alert_threshold: f64,
}

impl CostTracker {
    /// Create a new cost tracker with default pricing
    pub fn new(daily_limit_usd: f64, alert_threshold: f64) -> Self {
        Self {
            pricing: default_pricing_table(),
            records: Arc::new(RwLock::new(Vec::new())),
            daily_summaries: Arc::new(RwLock::new(HashMap::new())),
            daily_limit_usd,
            alert_threshold,
        }
    }

    /// Create a cost tracker from config
    pub fn from_config(config: &crate::config::CostConfig) -> Self {
        Self::new(config.daily_limit_usd, config.alert_threshold)
    }

    /// Add custom model pricing
    pub fn add_pricing(&mut self, pricing: ModelPricing) {
        self.pricing.insert(pricing.model.clone(), pricing);
    }

    /// Get pricing for a model, returns None if unknown
    pub fn get_pricing(&self, model: &str) -> Option<&ModelPricing> {
        self.pricing.get(model)
    }

    /// Record a new LLM cost
    pub fn record(&self, model: &str, tokens: TokenUsage, context: Option<String>) -> LlmCost {
        let (input_cost, output_cost) = self
            .pricing
            .get(model)
            .map(|p| p.calculate_cost(&tokens))
            .unwrap_or((0.0, 0.0));

        let cost = LlmCost {
            id: uuid::Uuid::new_v4().to_string(),
            model: model.to_string(),
            tokens,
            input_cost_usd: input_cost,
            output_cost_usd: output_cost,
            timestamp: Utc::now(),
            context,
        };

        // Add to records
        if let Ok(mut records) = self.records.write() {
            records.insert(0, cost.clone());
        }

        // Update daily summary
        let date = cost.timestamp.date_naive();
        if let Ok(mut summaries) = self.daily_summaries.write() {
            let summary = summaries
                .entry(date)
                .or_insert_with(|| DailyCostSummary::new(date));
            summary.add(&cost);
        }

        cost
    }

    /// Get today's total cost
    pub fn today_total(&self) -> f64 {
        let today = Utc::now().date_naive();
        self.daily_summaries
            .read()
            .ok()
            .and_then(|s| s.get(&today).map(|d| d.total_cost_usd))
            .unwrap_or(0.0)
    }

    /// Get today's summary
    pub fn today_summary(&self) -> Option<DailyCostSummary> {
        let today = Utc::now().date_naive();
        self.daily_summaries
            .read()
            .ok()
            .and_then(|s| s.get(&today).cloned())
    }

    /// Check if we're approaching the daily limit
    pub fn is_approaching_limit(&self) -> bool {
        self.today_total() >= self.daily_limit_usd * self.alert_threshold
    }

    /// Check if we've exceeded the daily limit
    pub fn is_over_limit(&self) -> bool {
        self.today_total() >= self.daily_limit_usd
    }

    /// Get remaining budget for today
    pub fn remaining_budget(&self) -> f64 {
        (self.daily_limit_usd - self.today_total()).max(0.0)
    }

    /// Get the daily limit
    pub fn daily_limit(&self) -> f64 {
        self.daily_limit_usd
    }

    /// Get all cost records (most recent first)
    pub fn records(&self) -> Vec<LlmCost> {
        self.records
            .read()
            .ok()
            .map(|r| r.clone())
            .unwrap_or_default()
    }

    /// Get records for a specific date
    pub fn records_for_date(&self, date: NaiveDate) -> Vec<LlmCost> {
        self.records
            .read()
            .ok()
            .map(|r| {
                r.iter()
                    .filter(|c| c.timestamp.date_naive() == date)
                    .cloned()
                    .collect()
            })
            .unwrap_or_default()
    }

    /// Get summary for a specific date
    pub fn summary_for_date(&self, date: NaiveDate) -> Option<DailyCostSummary> {
        self.daily_summaries
            .read()
            .ok()
            .and_then(|s| s.get(&date).cloned())
    }

    /// Clear all records and summaries (useful for testing)
    pub fn clear(&self) {
        if let Ok(mut records) = self.records.write() {
            records.clear();
        }
        if let Ok(mut summaries) = self.daily_summaries.write() {
            summaries.clear();
        }
    }
}

impl Clone for CostTracker {
    fn clone(&self) -> Self {
        Self {
            pricing: self.pricing.clone(),
            records: self.records.clone(),
            daily_summaries: self.daily_summaries.clone(),
            daily_limit_usd: self.daily_limit_usd,
            alert_threshold: self.alert_threshold,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_token_usage() {
        let usage = TokenUsage::new(100, 50);
        assert_eq!(usage.input_tokens, 100);
        assert_eq!(usage.output_tokens, 50);
        assert_eq!(usage.total(), 150);
    }

    #[test]
    fn test_model_pricing_calculation() {
        let pricing = ModelPricing::new("test-model", 3.0, 15.0);
        let tokens = TokenUsage::new(1_000_000, 500_000);

        let (input_cost, output_cost) = pricing.calculate_cost(&tokens);

        assert!((input_cost - 3.0).abs() < 0.001);
        assert!((output_cost - 7.5).abs() < 0.001);
    }

    #[test]
    fn test_model_pricing_small_usage() {
        let pricing = ModelPricing::new("test-model", 3.0, 15.0);
        let tokens = TokenUsage::new(1000, 500);

        let (input_cost, output_cost) = pricing.calculate_cost(&tokens);

        assert!((input_cost - 0.003).abs() < 0.0001);
        assert!((output_cost - 0.0075).abs() < 0.0001);
    }

    #[test]
    fn test_cost_tracker_record() {
        let tracker = CostTracker::new(10.0, 0.8);

        let cost = tracker.record(
            "anthropic/claude-sonnet-4-20250514",
            TokenUsage::new(1000, 500),
            Some("test-context".to_string()),
        );

        assert_eq!(cost.model, "anthropic/claude-sonnet-4-20250514");
        assert_eq!(cost.tokens.input_tokens, 1000);
        assert_eq!(cost.tokens.output_tokens, 500);
        assert!(cost.input_cost_usd > 0.0);
        assert!(cost.output_cost_usd > 0.0);
        assert_eq!(cost.context, Some("test-context".to_string()));
    }

    #[test]
    fn test_cost_tracker_today_total() {
        let tracker = CostTracker::new(10.0, 0.8);

        tracker.record(
            "anthropic/claude-sonnet-4-20250514",
            TokenUsage::new(1_000_000, 0),
            None,
        );

        let today_total = tracker.today_total();
        assert!((today_total - 3.0).abs() < 0.001);
    }

    #[test]
    fn test_cost_tracker_budget_checks() {
        let tracker = CostTracker::new(1.0, 0.8);

        // Start with no usage
        assert!(!tracker.is_over_limit());
        assert!(!tracker.is_approaching_limit());
        assert!((tracker.remaining_budget() - 1.0).abs() < 0.001);

        // Add usage that approaches limit (0.9 of 1.0)
        tracker.record(
            "anthropic/claude-sonnet-4-20250514",
            TokenUsage::new(300_000, 0),
            None,
        );

        assert!(tracker.is_approaching_limit());
        assert!(!tracker.is_over_limit());

        // Add more to exceed limit
        tracker.record(
            "anthropic/claude-sonnet-4-20250514",
            TokenUsage::new(100_000, 0),
            None,
        );

        assert!(tracker.is_over_limit());
    }

    #[test]
    fn test_daily_summary_aggregation() {
        let tracker = CostTracker::new(10.0, 0.8);

        tracker.record(
            "anthropic/claude-sonnet-4-20250514",
            TokenUsage::new(1000, 500),
            None,
        );
        tracker.record(
            "anthropic/claude-3-5-haiku-latest",
            TokenUsage::new(2000, 1000),
            None,
        );

        let summary = tracker
            .today_summary()
            .expect("Should have today's summary");

        assert_eq!(summary.call_count, 2);
        assert_eq!(summary.total_input_tokens, 3000);
        assert_eq!(summary.total_output_tokens, 1500);
        assert_eq!(summary.by_model.len(), 2);
    }

    #[test]
    fn test_unknown_model_pricing() {
        let tracker = CostTracker::new(10.0, 0.8);

        let cost = tracker.record("unknown/model", TokenUsage::new(1000, 500), None);

        // Unknown models should have zero cost
        assert_eq!(cost.input_cost_usd, 0.0);
        assert_eq!(cost.output_cost_usd, 0.0);
    }

    #[test]
    fn test_add_custom_pricing() {
        let mut tracker = CostTracker::new(10.0, 0.8);

        tracker.add_pricing(ModelPricing::new("custom/model", 1.0, 2.0));

        let cost = tracker.record("custom/model", TokenUsage::new(1_000_000, 500_000), None);

        assert!((cost.input_cost_usd - 1.0).abs() < 0.001);
        assert!((cost.output_cost_usd - 1.0).abs() < 0.001);
    }

    #[test]
    fn test_llm_cost_total() {
        let cost = LlmCost {
            id: "test".to_string(),
            model: "test".to_string(),
            tokens: TokenUsage::new(100, 50),
            input_cost_usd: 0.5,
            output_cost_usd: 0.25,
            timestamp: Utc::now(),
            context: None,
        };

        assert!((cost.total_cost_usd() - 0.75).abs() < 0.001);
    }

    #[test]
    fn test_cost_tracker_clear() {
        let tracker = CostTracker::new(10.0, 0.8);

        tracker.record(
            "anthropic/claude-sonnet-4-20250514",
            TokenUsage::new(1000, 500),
            None,
        );

        assert!(!tracker.records().is_empty());
        assert!(tracker.today_summary().is_some());

        tracker.clear();

        assert!(tracker.records().is_empty());
        assert!(tracker.today_summary().is_none());
    }
}
