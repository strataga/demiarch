//! Costs API
//!
//! Provides high-level operations for cost tracking from GUI.

use crate::cost::{CostTracker, ModelCostSummary};
use crate::Result;
use chrono::NaiveDate;
use serde::{Deserialize, Serialize};

/// Cost summary for GUI display
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CostSummaryDto {
    pub date: String,
    pub total_cost_usd: f64,
    pub total_input_tokens: u64,
    pub total_output_tokens: u64,
    pub call_count: u32,
    pub by_model: Vec<ModelCostDto>,
    pub daily_limit_usd: f64,
    pub remaining_budget_usd: f64,
    pub is_over_limit: bool,
    pub is_approaching_limit: bool,
}

/// Model cost breakdown for GUI display
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelCostDto {
    pub model: String,
    pub total_cost_usd: f64,
    pub total_input_tokens: u64,
    pub total_output_tokens: u64,
    pub call_count: u32,
}

impl From<ModelCostSummary> for ModelCostDto {
    fn from(m: ModelCostSummary) -> Self {
        Self {
            model: m.model,
            total_cost_usd: m.total_cost_usd,
            total_input_tokens: m.total_input_tokens,
            total_output_tokens: m.total_output_tokens,
            call_count: m.call_count,
        }
    }
}

/// Get today's cost summary
pub fn get_today_summary(tracker: &CostTracker) -> CostSummaryDto {
    let today = chrono::Utc::now().date_naive();
    let summary = tracker.today_summary();

    match summary {
        Some(s) => CostSummaryDto {
            date: s.date.to_string(),
            total_cost_usd: s.total_cost_usd,
            total_input_tokens: s.total_input_tokens,
            total_output_tokens: s.total_output_tokens,
            call_count: s.call_count,
            by_model: s.by_model.into_values().map(ModelCostDto::from).collect(),
            daily_limit_usd: tracker.daily_limit(),
            remaining_budget_usd: tracker.remaining_budget(),
            is_over_limit: tracker.is_over_limit(),
            is_approaching_limit: tracker.is_approaching_limit(),
        },
        None => CostSummaryDto {
            date: today.to_string(),
            total_cost_usd: 0.0,
            total_input_tokens: 0,
            total_output_tokens: 0,
            call_count: 0,
            by_model: vec![],
            daily_limit_usd: tracker.daily_limit(),
            remaining_budget_usd: tracker.remaining_budget(),
            is_over_limit: false,
            is_approaching_limit: false,
        },
    }
}

/// Get cost summary for a specific date
pub fn get_summary_for_date(tracker: &CostTracker, date: &str) -> Result<Option<CostSummaryDto>> {
    let date = NaiveDate::parse_from_str(date, "%Y-%m-%d")
        .map_err(|_| crate::Error::InvalidInput(format!("Invalid date: {}", date)))?;

    let summary = tracker.summary_for_date(date);

    Ok(summary.map(|s| CostSummaryDto {
        date: s.date.to_string(),
        total_cost_usd: s.total_cost_usd,
        total_input_tokens: s.total_input_tokens,
        total_output_tokens: s.total_output_tokens,
        call_count: s.call_count,
        by_model: s.by_model.into_values().map(ModelCostDto::from).collect(),
        daily_limit_usd: tracker.daily_limit(),
        remaining_budget_usd: tracker.remaining_budget(),
        is_over_limit: tracker.is_over_limit(),
        is_approaching_limit: tracker.is_approaching_limit(),
    }))
}

/// Cost history entry
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CostHistoryEntry {
    pub id: String,
    pub model: String,
    pub input_tokens: u32,
    pub output_tokens: u32,
    pub input_cost_usd: f64,
    pub output_cost_usd: f64,
    pub total_cost_usd: f64,
    pub timestamp: String,
    pub context: Option<String>,
}

/// Get recent cost records
pub fn get_recent_records(tracker: &CostTracker, limit: usize) -> Vec<CostHistoryEntry> {
    tracker
        .records()
        .into_iter()
        .take(limit)
        .map(|r| {
            let total = r.total_cost_usd();
            CostHistoryEntry {
                id: r.id,
                model: r.model,
                input_tokens: r.tokens.input_tokens,
                output_tokens: r.tokens.output_tokens,
                input_cost_usd: r.input_cost_usd,
                output_cost_usd: r.output_cost_usd,
                total_cost_usd: total,
                timestamp: r.timestamp.to_rfc3339(),
                context: r.context,
            }
        })
        .collect()
}

/// Budget status for quick checks
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BudgetStatus {
    pub today_spent_usd: f64,
    pub daily_limit_usd: f64,
    pub remaining_usd: f64,
    pub usage_percent: f64,
    pub is_over_limit: bool,
    pub is_approaching_limit: bool,
}

/// Get current budget status
pub fn get_budget_status(tracker: &CostTracker) -> BudgetStatus {
    let today_spent = tracker.today_total();
    let limit = tracker.daily_limit();

    BudgetStatus {
        today_spent_usd: today_spent,
        daily_limit_usd: limit,
        remaining_usd: tracker.remaining_budget(),
        usage_percent: if limit > 0.0 {
            (today_spent / limit) * 100.0
        } else {
            0.0
        },
        is_over_limit: tracker.is_over_limit(),
        is_approaching_limit: tracker.is_approaching_limit(),
    }
}
