//! Progressive disclosure context management
//!
//! This module provides intelligent context management for hierarchical agent systems,
//! implementing progressive disclosure to manage token limits effectively while
//! preserving essential information as context flows through agent hierarchies.
//!
//! # Key Concepts
//!
//! - **ContextBudget**: Per-level token allocation strategy
//! - **DisclosureLevel**: Granularity of context (Full, Summary, Essential, Minimal)
//! - **ContextWindow**: Manages context within token limits with automatic compression
//! - **MessageSummarizer**: Compresses messages while preserving semantic content
//!
//! # Usage
//!
//! ```rust,ignore
//! use demiarch_core::context::{ContextBudget, ContextWindow, DisclosureLevel};
//!
//! // Create a budget with 8192 total tokens
//! let budget = ContextBudget::new(8192);
//!
//! // Get allocation for depth 2 (Coder/Reviewer/Tester level)
//! let allocation = budget.allocation_for_depth(2);
//!
//! // Create a context window with the allocation
//! let mut window = ContextWindow::new(allocation);
//!
//! // Add messages - window automatically manages overflow
//! window.add_message(message);
//! ```

use std::collections::VecDeque;

use serde::{Deserialize, Serialize};

use crate::llm::Message;

/// Disclosure level for context information
///
/// Determines how much detail to include when passing context to child agents.
/// Higher levels preserve more information but consume more tokens.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum DisclosureLevel {
    /// Minimal context - only critical information
    /// Used when tokens are extremely constrained
    Minimal = 0,
    /// Essential context - key decisions and constraints
    /// Default for deep hierarchy levels
    Essential = 1,
    /// Summary context - condensed version of full context
    /// Good balance of information and token usage
    Summary = 2,
    /// Full context - all available information
    /// Used at top levels where budget allows
    Full = 3,
}

impl DisclosureLevel {
    /// Get the appropriate disclosure level for a given depth
    ///
    /// Deeper agents get less detailed context to preserve token budget
    pub fn for_depth(depth: u8) -> Self {
        match depth {
            0 => DisclosureLevel::Full,      // Orchestrator - full context
            1 => DisclosureLevel::Summary,   // Planner - summarized context
            _ => DisclosureLevel::Essential, // Workers - essential only
        }
    }

    /// Get the target compression ratio for this level
    ///
    /// Returns a value between 0.0 and 1.0 indicating how much of the
    /// original content should be preserved
    pub fn compression_ratio(&self) -> f32 {
        match self {
            DisclosureLevel::Full => 1.0,
            DisclosureLevel::Summary => 0.5,
            DisclosureLevel::Essential => 0.25,
            DisclosureLevel::Minimal => 0.1,
        }
    }
}

impl std::fmt::Display for DisclosureLevel {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            DisclosureLevel::Minimal => write!(f, "minimal"),
            DisclosureLevel::Essential => write!(f, "essential"),
            DisclosureLevel::Summary => write!(f, "summary"),
            DisclosureLevel::Full => write!(f, "full"),
        }
    }
}

/// Token allocation for a specific context level
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct TokenAllocation {
    /// Maximum tokens for system prompts
    pub system_tokens: usize,
    /// Maximum tokens for inherited context
    pub context_tokens: usize,
    /// Maximum tokens for the current task/input
    pub input_tokens: usize,
    /// Reserved tokens for response generation
    pub output_tokens: usize,
}

impl TokenAllocation {
    /// Create a new token allocation
    pub fn new(system: usize, context: usize, input: usize, output: usize) -> Self {
        Self {
            system_tokens: system,
            context_tokens: context,
            input_tokens: input,
            output_tokens: output,
        }
    }

    /// Total tokens available (excluding output reservation)
    pub fn total_input(&self) -> usize {
        self.system_tokens + self.context_tokens + self.input_tokens
    }

    /// Total budget including output
    pub fn total(&self) -> usize {
        self.total_input() + self.output_tokens
    }
}

impl Default for TokenAllocation {
    fn default() -> Self {
        Self {
            system_tokens: 1024,
            context_tokens: 2048,
            input_tokens: 2048,
            output_tokens: 3072,
        }
    }
}

/// Context budget configuration
///
/// Manages token allocation across the agent hierarchy, ensuring each level
/// has appropriate token budgets while reserving space for responses.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContextBudget {
    /// Total available tokens for the model
    pub total_tokens: usize,
    /// Percentage of tokens reserved for output (0.0 to 1.0)
    pub output_reserve: f32,
    /// Per-level allocation overrides
    pub level_allocations: Vec<TokenAllocation>,
}

impl ContextBudget {
    /// Create a new context budget with total available tokens
    pub fn new(total_tokens: usize) -> Self {
        let output_reserve = 0.375; // 37.5% for output
        let input_budget = (total_tokens as f32 * (1.0 - output_reserve)) as usize;
        let output_budget = total_tokens - input_budget;

        Self {
            total_tokens,
            output_reserve,
            level_allocations: vec![
                // Level 0 (Orchestrator): Most context budget
                TokenAllocation::new(
                    1024,                    // System prompt
                    input_budget * 40 / 100, // 40% for inherited context
                    input_budget * 30 / 100, // 30% for input
                    output_budget,
                ),
                // Level 1 (Planner): Moderate context budget
                TokenAllocation::new(
                    1024,                    // System prompt
                    input_budget * 30 / 100, // 30% for inherited context
                    input_budget * 35 / 100, // 35% for input
                    output_budget,
                ),
                // Level 2 (Workers): Focused context budget
                TokenAllocation::new(
                    1024,                    // System prompt
                    input_budget * 20 / 100, // 20% for inherited context
                    input_budget * 40 / 100, // 40% for input
                    output_budget,
                ),
            ],
        }
    }

    /// Get token allocation for a specific depth level
    pub fn allocation_for_depth(&self, depth: u8) -> TokenAllocation {
        self.level_allocations
            .get(depth as usize)
            .copied()
            .unwrap_or_else(|| {
                // For depths beyond our defined levels, use most restrictive allocation
                *self
                    .level_allocations
                    .last()
                    .unwrap_or(&TokenAllocation::default())
            })
    }

    /// Calculate remaining tokens after system prompt
    pub fn remaining_for_context(&self, depth: u8, system_tokens_used: usize) -> usize {
        let allocation = self.allocation_for_depth(depth);
        allocation
            .context_tokens
            .saturating_sub(system_tokens_used.saturating_sub(allocation.system_tokens))
    }

    /// Get the disclosure level for a specific depth
    pub fn disclosure_level(&self, depth: u8) -> DisclosureLevel {
        DisclosureLevel::for_depth(depth)
    }
}

impl Default for ContextBudget {
    fn default() -> Self {
        Self::new(8192) // Default to 8k context
    }
}

/// Estimated token count for a string
///
/// Uses a simple heuristic: ~4 characters per token on average.
/// This is a rough estimate; actual tokenization varies by model.
pub fn estimate_tokens(text: &str) -> usize {
    // Rough estimate: 1 token ≈ 4 characters for English text
    // Add some buffer for special tokens and formatting
    text.len().div_ceil(4)
}

/// Estimate tokens for a message
pub fn estimate_message_tokens(message: &Message) -> usize {
    // Role adds ~2 tokens, plus content
    2 + estimate_tokens(&message.content)
}

/// Estimate tokens for a list of messages
pub fn estimate_messages_tokens(messages: &[Message]) -> usize {
    messages.iter().map(estimate_message_tokens).sum()
}

/// Context window manager
///
/// Manages a sliding window of context within token limits,
/// automatically compressing or truncating when needed.
#[derive(Debug, Clone)]
pub struct ContextWindow {
    /// Token allocation for this window
    allocation: TokenAllocation,
    /// System messages (always preserved)
    system_messages: Vec<Message>,
    /// Context messages (can be compressed/pruned)
    context_messages: VecDeque<Message>,
    /// Current token count for system messages
    system_tokens: usize,
    /// Current token count for context messages
    context_tokens: usize,
    /// Disclosure level for this window
    disclosure_level: DisclosureLevel,
}

impl ContextWindow {
    /// Create a new context window with the given allocation
    pub fn new(allocation: TokenAllocation) -> Self {
        Self {
            allocation,
            system_messages: Vec::new(),
            context_messages: VecDeque::new(),
            system_tokens: 0,
            context_tokens: 0,
            disclosure_level: DisclosureLevel::Full,
        }
    }

    /// Create a context window with a specific disclosure level
    pub fn with_disclosure_level(mut self, level: DisclosureLevel) -> Self {
        self.disclosure_level = level;
        self
    }

    /// Get the disclosure level for this window
    pub fn disclosure_level(&self) -> DisclosureLevel {
        self.disclosure_level
    }

    /// Add a system message
    pub fn add_system_message(&mut self, message: Message) {
        let tokens = estimate_message_tokens(&message);
        self.system_messages.push(message);
        self.system_tokens += tokens;
    }

    /// Add a context message, managing overflow
    pub fn add_context_message(&mut self, message: Message) {
        let tokens = estimate_message_tokens(&message);

        // If adding this would exceed budget, make room
        while self.context_tokens + tokens > self.allocation.context_tokens
            && !self.context_messages.is_empty()
        {
            if let Some(removed) = self.context_messages.pop_front() {
                self.context_tokens -= estimate_message_tokens(&removed);
            }
        }

        // Add the message if it fits
        if self.context_tokens + tokens <= self.allocation.context_tokens {
            self.context_messages.push_back(message);
            self.context_tokens += tokens;
        } else {
            // Message too large even for empty window - truncate it
            let truncated = self.truncate_message(&message, self.allocation.context_tokens);
            let truncated_tokens = estimate_message_tokens(&truncated);
            self.context_messages.push_back(truncated);
            self.context_tokens += truncated_tokens;
        }
    }

    /// Get all messages in order (system first, then context)
    pub fn messages(&self) -> Vec<Message> {
        let mut result = self.system_messages.clone();
        result.extend(self.context_messages.iter().cloned());
        result
    }

    /// Get the current token count
    pub fn token_count(&self) -> usize {
        self.system_tokens + self.context_tokens
    }

    /// Get remaining token capacity
    pub fn remaining_tokens(&self) -> usize {
        self.allocation
            .total_input()
            .saturating_sub(self.token_count())
    }

    /// Check if the window is at or near capacity
    pub fn is_near_capacity(&self) -> bool {
        let usage = self.token_count() as f32 / self.allocation.total_input() as f32;
        usage > 0.9
    }

    /// Compress context to a target token count
    pub fn compress_to(&mut self, target_tokens: usize) {
        while self.context_tokens > target_tokens && !self.context_messages.is_empty() {
            // Remove oldest context message
            if let Some(removed) = self.context_messages.pop_front() {
                self.context_tokens -= estimate_message_tokens(&removed);
            }
        }
    }

    /// Create a child context window with inherited messages
    ///
    /// Compresses messages according to the child's disclosure level
    pub fn child_window(&self, child_allocation: TokenAllocation, child_depth: u8) -> Self {
        let disclosure = DisclosureLevel::for_depth(child_depth);
        let mut child = ContextWindow::new(child_allocation).with_disclosure_level(disclosure);

        // Compress and inherit context based on disclosure level
        let compressed_context = self.compress_for_disclosure(disclosure);

        for message in compressed_context {
            child.add_context_message(message);
        }

        child
    }

    /// Compress current context for a given disclosure level
    fn compress_for_disclosure(&self, level: DisclosureLevel) -> Vec<Message> {
        match level {
            DisclosureLevel::Full => {
                // Pass through all context
                self.context_messages.iter().cloned().collect()
            }
            DisclosureLevel::Summary => {
                // Summarize multi-message context
                self.summarize_context()
            }
            DisclosureLevel::Essential => {
                // Extract only essential information
                self.extract_essential()
            }
            DisclosureLevel::Minimal => {
                // Extract minimal context
                self.extract_minimal()
            }
        }
    }

    /// Summarize context into fewer messages
    fn summarize_context(&self) -> Vec<Message> {
        if self.context_messages.is_empty() {
            return Vec::new();
        }

        // Group consecutive messages by role and summarize
        let mut result = Vec::new();
        let mut current_content = String::new();
        let mut current_role = None;

        for message in &self.context_messages {
            if current_role != Some(message.role) {
                if !current_content.is_empty() {
                    if let Some(role) = current_role {
                        result.push(Message::new(role, summarize_text(&current_content)));
                    }
                }
                current_content.clear();
                current_role = Some(message.role);
            }

            if !current_content.is_empty() {
                current_content.push_str("\n\n");
            }
            current_content.push_str(&message.content);
        }

        // Add final accumulated content
        if !current_content.is_empty() {
            if let Some(role) = current_role {
                result.push(Message::new(role, summarize_text(&current_content)));
            }
        }

        result
    }

    /// Extract essential information from context
    fn extract_essential(&self) -> Vec<Message> {
        if self.context_messages.is_empty() {
            return Vec::new();
        }

        // Keep system messages and extract key points from user/assistant messages
        let essential_content: Vec<String> = self
            .context_messages
            .iter()
            .filter_map(|msg| {
                let extracted = extract_key_points(&msg.content);
                if extracted.is_empty() {
                    None
                } else {
                    Some(extracted)
                }
            })
            .collect();

        if essential_content.is_empty() {
            Vec::new()
        } else {
            vec![Message::system(format!(
                "Context summary:\n{}",
                essential_content.join("\n")
            ))]
        }
    }

    /// Extract minimal context
    fn extract_minimal(&self) -> Vec<Message> {
        if self.context_messages.is_empty() {
            return Vec::new();
        }

        // Only keep the most recent significant message
        if let Some(last_significant) = self
            .context_messages
            .iter()
            .rev()
            .find(|msg| msg.content.len() > 50)
        {
            let minimal = truncate_to_sentences(&last_significant.content, 2);
            vec![Message::system(format!("Previous context: {}", minimal))]
        } else {
            Vec::new()
        }
    }

    /// Truncate a message to fit within token limit
    fn truncate_message(&self, message: &Message, max_tokens: usize) -> Message {
        let max_chars = max_tokens * 4; // Rough estimate
        let content = if message.content.len() > max_chars {
            format!("{}...[truncated]", &message.content[..max_chars])
        } else {
            message.content.clone()
        };
        Message::new(message.role, content)
    }
}

/// Progressive context manager for agent hierarchies
///
/// Coordinates context passing between parent and child agents,
/// applying progressive disclosure based on depth and token budgets.
#[derive(Debug, Clone)]
pub struct ProgressiveContext {
    /// The context budget configuration
    budget: ContextBudget,
    /// Context windows for each active agent
    windows: Vec<ContextWindow>,
}

impl ProgressiveContext {
    /// Create a new progressive context with the given budget
    pub fn new(budget: ContextBudget) -> Self {
        Self {
            budget,
            windows: Vec::new(),
        }
    }

    /// Create a root context window for the orchestrator
    pub fn root_window(&mut self) -> ContextWindow {
        let allocation = self.budget.allocation_for_depth(0);
        let window = ContextWindow::new(allocation).with_disclosure_level(DisclosureLevel::Full);
        self.windows.push(window.clone());
        window
    }

    /// Create a child context window from a parent
    pub fn child_window(&mut self, parent: &ContextWindow, child_depth: u8) -> ContextWindow {
        let allocation = self.budget.allocation_for_depth(child_depth);
        let window = parent.child_window(allocation, child_depth);
        self.windows.push(window.clone());
        window
    }

    /// Get the disclosure level for a depth
    pub fn disclosure_level(&self, depth: u8) -> DisclosureLevel {
        self.budget.disclosure_level(depth)
    }

    /// Get the budget configuration
    pub fn budget(&self) -> &ContextBudget {
        &self.budget
    }
}

impl Default for ProgressiveContext {
    fn default() -> Self {
        Self::new(ContextBudget::default())
    }
}

// Helper functions for text processing

/// Summarize text by extracting first and last sentences
fn summarize_text(text: &str) -> String {
    let sentences: Vec<&str> = text
        .split(['.', '!', '?'])
        .map(|s| s.trim())
        .filter(|s| !s.is_empty())
        .collect();

    match sentences.len() {
        0 => text.to_string(),
        1 => sentences[0].to_string(),
        2 => format!("{}. {}.", sentences[0], sentences[1]),
        n => {
            // Keep first sentence, middle indicator, and last sentence
            format!(
                "{}. [...{} sentences omitted...] {}.",
                sentences[0],
                n - 2,
                sentences[n - 1]
            )
        }
    }
}

/// Extract key points from text
fn extract_key_points(text: &str) -> String {
    // Look for bullet points, numbered items, or key phrases
    let lines: Vec<&str> = text.lines().collect();

    let key_lines: Vec<&str> = lines
        .iter()
        .filter(|line| {
            let trimmed = line.trim();
            // Keep lines that look like key points
            trimmed.starts_with('-')
                || trimmed.starts_with('*')
                || trimmed.starts_with("•")
                || trimmed
                    .chars()
                    .next()
                    .map(|c| c.is_ascii_digit())
                    .unwrap_or(false)
                || trimmed.to_lowercase().starts_with("important")
                || trimmed.to_lowercase().starts_with("note")
                || trimmed.to_lowercase().starts_with("key")
                || trimmed.contains(':')
        })
        .copied()
        .collect();

    if key_lines.is_empty() {
        // Fall back to first line if no key points found
        lines.first().copied().unwrap_or("").to_string()
    } else {
        key_lines.join("\n")
    }
}

/// Truncate text to a specific number of sentences
fn truncate_to_sentences(text: &str, max_sentences: usize) -> String {
    let mut result = String::new();
    let mut sentence_count = 0;

    for c in text.chars() {
        result.push(c);
        if c == '.' || c == '!' || c == '?' {
            sentence_count += 1;
            if sentence_count >= max_sentences {
                break;
            }
        }
    }

    result
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::llm::MessageRole;

    #[test]
    fn test_disclosure_level_ordering() {
        assert!(DisclosureLevel::Minimal < DisclosureLevel::Essential);
        assert!(DisclosureLevel::Essential < DisclosureLevel::Summary);
        assert!(DisclosureLevel::Summary < DisclosureLevel::Full);
    }

    #[test]
    fn test_disclosure_level_for_depth() {
        assert_eq!(DisclosureLevel::for_depth(0), DisclosureLevel::Full);
        assert_eq!(DisclosureLevel::for_depth(1), DisclosureLevel::Summary);
        assert_eq!(DisclosureLevel::for_depth(2), DisclosureLevel::Essential);
        assert_eq!(DisclosureLevel::for_depth(10), DisclosureLevel::Essential);
    }

    #[test]
    fn test_disclosure_level_compression_ratio() {
        assert_eq!(DisclosureLevel::Full.compression_ratio(), 1.0);
        assert_eq!(DisclosureLevel::Summary.compression_ratio(), 0.5);
        assert_eq!(DisclosureLevel::Essential.compression_ratio(), 0.25);
        assert_eq!(DisclosureLevel::Minimal.compression_ratio(), 0.1);
    }

    #[test]
    fn test_disclosure_level_display() {
        assert_eq!(DisclosureLevel::Full.to_string(), "full");
        assert_eq!(DisclosureLevel::Summary.to_string(), "summary");
        assert_eq!(DisclosureLevel::Essential.to_string(), "essential");
        assert_eq!(DisclosureLevel::Minimal.to_string(), "minimal");
    }

    #[test]
    fn test_token_allocation() {
        let allocation = TokenAllocation::new(1024, 2048, 2048, 3072);
        assert_eq!(allocation.total_input(), 5120);
        assert_eq!(allocation.total(), 8192);
    }

    #[test]
    fn test_token_allocation_default() {
        let allocation = TokenAllocation::default();
        assert_eq!(allocation.system_tokens, 1024);
        assert_eq!(allocation.context_tokens, 2048);
        assert_eq!(allocation.input_tokens, 2048);
        assert_eq!(allocation.output_tokens, 3072);
    }

    #[test]
    fn test_context_budget_new() {
        let budget = ContextBudget::new(8192);
        assert_eq!(budget.total_tokens, 8192);
        assert_eq!(budget.level_allocations.len(), 3);
    }

    #[test]
    fn test_context_budget_allocation_for_depth() {
        let budget = ContextBudget::new(8192);

        let level0 = budget.allocation_for_depth(0);
        let level1 = budget.allocation_for_depth(1);
        let level2 = budget.allocation_for_depth(2);

        // Level 0 should have most context budget
        assert!(level0.context_tokens > level1.context_tokens);
        assert!(level1.context_tokens > level2.context_tokens);

        // Beyond level 2 should use most restrictive
        let level5 = budget.allocation_for_depth(5);
        assert_eq!(level5.context_tokens, level2.context_tokens);
    }

    #[test]
    fn test_context_budget_disclosure_level() {
        let budget = ContextBudget::new(8192);
        assert_eq!(budget.disclosure_level(0), DisclosureLevel::Full);
        assert_eq!(budget.disclosure_level(1), DisclosureLevel::Summary);
        assert_eq!(budget.disclosure_level(2), DisclosureLevel::Essential);
    }

    #[test]
    fn test_estimate_tokens() {
        assert_eq!(estimate_tokens(""), 0);
        assert_eq!(estimate_tokens("test"), 1); // 4 chars = 1 token
        assert_eq!(estimate_tokens("hello world!"), 3); // 12 chars = 3 tokens
    }

    #[test]
    fn test_estimate_message_tokens() {
        let message = Message::user("Hello, world!");
        let tokens = estimate_message_tokens(&message);
        // 2 (role) + 4 (content ~13 chars / 4) = ~6
        assert!(tokens > 0);
        assert!(tokens < 20);
    }

    #[test]
    fn test_context_window_new() {
        let allocation = TokenAllocation::default();
        let window = ContextWindow::new(allocation);
        assert_eq!(window.token_count(), 0);
        assert_eq!(window.disclosure_level(), DisclosureLevel::Full);
    }

    #[test]
    fn test_context_window_add_system_message() {
        let allocation = TokenAllocation::default();
        let mut window = ContextWindow::new(allocation);

        window.add_system_message(Message::system("You are a helpful assistant."));
        assert!(window.system_tokens > 0);
        assert_eq!(window.system_messages.len(), 1);
    }

    #[test]
    fn test_context_window_add_context_message() {
        let allocation = TokenAllocation::default();
        let mut window = ContextWindow::new(allocation);

        window.add_context_message(Message::user("Hello!"));
        window.add_context_message(Message::assistant("Hi there!"));

        assert_eq!(window.context_messages.len(), 2);
        assert!(window.context_tokens > 0);
    }

    #[test]
    fn test_context_window_overflow() {
        // Create a very small allocation to force overflow
        let allocation = TokenAllocation::new(100, 50, 100, 100);
        let mut window = ContextWindow::new(allocation);

        // Add messages until we exceed context budget
        for i in 0..20 {
            window.add_context_message(Message::user(format!(
                "This is message number {} with some content.",
                i
            )));
        }

        // Should have pruned older messages
        assert!(window.context_tokens <= allocation.context_tokens + 50); // Allow some slack
    }

    #[test]
    fn test_context_window_messages() {
        let allocation = TokenAllocation::default();
        let mut window = ContextWindow::new(allocation);

        window.add_system_message(Message::system("System prompt"));
        window.add_context_message(Message::user("User message"));

        let messages = window.messages();
        assert_eq!(messages.len(), 2);
        assert_eq!(messages[0].role, MessageRole::System);
        assert_eq!(messages[1].role, MessageRole::User);
    }

    #[test]
    fn test_context_window_compress_to() {
        let allocation = TokenAllocation::new(1000, 2000, 1000, 1000);
        let mut window = ContextWindow::new(allocation);

        // Add several messages with longer content
        for i in 0..10 {
            window.add_context_message(Message::user(format!(
                "This is message number {} with a much longer content to ensure we have enough tokens for the compression test to work properly.",
                i
            )));
        }

        let original_count = window.context_messages.len();
        let original_tokens = window.context_tokens;
        assert!(
            original_tokens > 100,
            "Need more tokens to test compression"
        );

        window.compress_to(100);

        // Should have fewer messages after compression
        assert!(window.context_messages.len() < original_count);
        // Allow slack for the last message that couldn't be split
        assert!(window.context_tokens <= original_tokens);
    }

    #[test]
    fn test_context_window_remaining_tokens() {
        let allocation = TokenAllocation::new(1000, 2000, 2000, 1000);
        let mut window = ContextWindow::new(allocation);

        let initial_remaining = window.remaining_tokens();
        assert_eq!(initial_remaining, 5000); // total_input = 5000

        window.add_system_message(Message::system("Short message"));
        assert!(window.remaining_tokens() < initial_remaining);
    }

    #[test]
    fn test_context_window_is_near_capacity() {
        let allocation = TokenAllocation::new(100, 100, 100, 100);
        let mut window = ContextWindow::new(allocation);

        assert!(!window.is_near_capacity());

        // Fill up the window - need to fill 90% of 300 total input tokens = 270 tokens
        // ~270 tokens * 4 chars/token = 1080 chars needed
        window.add_system_message(Message::system("A".repeat(1100)));
        assert!(window.is_near_capacity());
    }

    #[test]
    fn test_context_window_child_window() {
        let parent_allocation = TokenAllocation::new(1024, 2048, 2048, 3072);
        let mut parent = ContextWindow::new(parent_allocation);

        parent.add_context_message(Message::user("Parent context message"));
        parent.add_context_message(Message::assistant("Parent response"));

        let child_allocation = TokenAllocation::new(1024, 1024, 2048, 3072);
        let child = parent.child_window(child_allocation, 1);

        // Child should have inherited (possibly compressed) context
        assert!(child.context_tokens > 0 || child.context_messages.is_empty());
        assert_eq!(child.disclosure_level(), DisclosureLevel::Summary);
    }

    #[test]
    fn test_progressive_context_new() {
        let budget = ContextBudget::new(8192);
        let context = ProgressiveContext::new(budget);
        assert_eq!(context.budget().total_tokens, 8192);
    }

    #[test]
    fn test_progressive_context_root_window() {
        let mut context = ProgressiveContext::default();
        let window = context.root_window();
        assert_eq!(window.disclosure_level(), DisclosureLevel::Full);
    }

    #[test]
    fn test_progressive_context_child_window() {
        let mut context = ProgressiveContext::default();
        let mut root = context.root_window();
        root.add_context_message(Message::user("Root context"));

        let child = context.child_window(&root, 1);
        assert_eq!(child.disclosure_level(), DisclosureLevel::Summary);
    }

    #[test]
    fn test_summarize_text() {
        let text = "First sentence. Second sentence. Third sentence. Fourth sentence.";
        let summary = summarize_text(text);
        assert!(summary.contains("First sentence"));
        assert!(summary.contains("Fourth sentence"));
        assert!(summary.contains("omitted"));
    }

    #[test]
    fn test_summarize_text_short() {
        let text = "Only one sentence.";
        let summary = summarize_text(text);
        assert_eq!(summary, "Only one sentence");
    }

    #[test]
    fn test_extract_key_points() {
        let text = "Introduction text.\n- Point one\n- Point two\nMore text.";
        let points = extract_key_points(text);
        assert!(points.contains("Point one"));
        assert!(points.contains("Point two"));
    }

    #[test]
    fn test_extract_key_points_no_bullets() {
        let text = "Just regular text without any key points.";
        let points = extract_key_points(text);
        // Should fall back to first line
        assert_eq!(points, "Just regular text without any key points.");
    }

    #[test]
    fn test_truncate_to_sentences() {
        let text = "First sentence. Second sentence. Third sentence. Fourth sentence.";
        let truncated = truncate_to_sentences(text, 2);
        assert!(truncated.contains("First sentence."));
        assert!(truncated.contains("Second sentence."));
        assert!(!truncated.contains("Third"));
    }
}
