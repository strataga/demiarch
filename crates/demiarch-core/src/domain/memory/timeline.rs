use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use super::error::MemoryError;
use super::full::compress_sentences;

/// A mid-layer representation of context used for progressive disclosure
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TimelineEntry {
    /// Concise summary of the conversation or interaction
    pub summary: String,
    /// Key moments extracted from the full context
    pub highlights: Vec<String>,
    /// When the entry was created
    pub created_at: DateTime<Utc>,
}

impl TimelineEntry {
    /// Build a timeline entry from raw content
    pub fn from_content(content: &str) -> Result<Self, MemoryError> {
        if content.trim().is_empty() {
            return Err(MemoryError::invalid("empty content"));
        }

        let highlights = extract_highlights(content);
        let summary = compress_sentences(content, 3);

        Ok(Self {
            summary,
            highlights,
            created_at: Utc::now(),
        })
    }
}

fn extract_highlights(content: &str) -> Vec<String> {
    // Very lightweight highlight extraction: keep bullet-like lines and first sentence per paragraph
    let mut highlights = Vec::new();
    for line in content.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }
        if trimmed.starts_with(['-', '*', '•']) {
            highlights.push(
                trimmed
                    .trim_start_matches(['-', '*', '•'])
                    .trim()
                    .to_string(),
            );
            continue;
        }
        if trimmed.len() > 12 {
            // Take the first sentence-ish chunk
            let first_sentence = trimmed
                .split(['.', '!', '?'])
                .filter(|s| !s.trim().is_empty())
                .next()
                .unwrap_or(trimmed)
                .trim()
                .to_string();
            highlights.push(first_sentence);
        }
        if highlights.len() >= 10 {
            break;
        }
    }

    highlights
}
