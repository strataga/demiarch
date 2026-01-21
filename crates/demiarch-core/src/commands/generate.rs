//! Code generation commands

use crate::Result;

/// Generate code for a feature
pub async fn generate(_feature_id: &str, _dry_run: bool) -> Result<GenerationResult> {
    Ok(GenerationResult {
        files_created: 0,
        files_modified: 0,
        tokens_used: 0,
        cost_usd: 0.0,
    })
}

/// Generation result
#[derive(Debug, Clone)]
pub struct GenerationResult {
    pub files_created: usize,
    pub files_modified: usize,
    pub tokens_used: u32,
    pub cost_usd: f64,
}
