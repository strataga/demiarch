//! Message building utilities for agents
//!
//! Provides common message construction patterns used by all agents,
//! including support for knowledge graph context enrichment.

use tracing::debug;

use crate::domain::knowledge::{
    ContextEnricher, EnrichedContext, EnrichmentConfig, KnowledgeGraphRepository,
};
use crate::llm::Message;

use super::context::AgentContext;
use super::traits::AgentInput;

/// Build the standard message sequence for an LLM call
///
/// Constructs messages in the correct order:
/// 1. System prompt
/// 2. Inherited context from parent agents
/// 3. Context messages from input
/// 4. The task/request
pub fn build_agent_messages(
    system_prompt: &str,
    inherited: &[Message],
    context: &[Message],
    task: &str,
) -> Vec<Message> {
    let mut messages = vec![Message::system(system_prompt)];

    // Add inherited context from parent agents
    messages.extend(inherited.iter().cloned());

    // Add context from input
    messages.extend(context.iter().cloned());

    // Add the task
    messages.push(Message::user(task));

    messages
}

/// Build messages from input and context with a system prompt
///
/// Convenience wrapper that extracts messages from AgentInput and AgentContext.
pub fn build_messages_from_input(
    system_prompt: &str,
    input: &AgentInput,
    context: &AgentContext,
) -> Vec<Message> {
    build_agent_messages(
        system_prompt,
        &context.inherited_messages,
        &input.context_messages,
        &input.task,
    )
}

// ========== Knowledge Graph Enriched Message Building ==========

/// Build messages with knowledge graph context enrichment
///
/// This function enhances the standard message building by:
/// 1. Analyzing the task to identify relevant concepts
/// 2. Querying the knowledge graph for related entities
/// 3. Injecting relevant context into the system prompt
///
/// Use this for code generation tasks where domain knowledge improves quality.
pub fn build_enriched_agent_messages(
    system_prompt: &str,
    inherited: &[Message],
    context_messages: &[Message],
    task: &str,
    enriched_context: Option<&EnrichedContext>,
) -> Vec<Message> {
    let mut messages = Vec::new();

    // Build enriched system prompt if context is available
    let final_system_prompt = if let Some(enriched) = enriched_context {
        if !enriched.formatted_context.is_empty() {
            format!("{}\n\n{}", system_prompt, enriched.formatted_context)
        } else {
            system_prompt.to_string()
        }
    } else {
        system_prompt.to_string()
    };

    messages.push(Message::system(&final_system_prompt));

    // Add inherited context from parent agents
    messages.extend(inherited.iter().cloned());

    // Add context from input
    messages.extend(context_messages.iter().cloned());

    // Add the task
    messages.push(Message::user(task));

    messages
}

/// Build enriched messages from input and context
///
/// Async version that automatically queries the knowledge graph for context.
pub async fn build_enriched_messages_from_input<R: KnowledgeGraphRepository>(
    system_prompt: &str,
    input: &AgentInput,
    context: &AgentContext,
    enricher: &ContextEnricher<R>,
) -> Vec<Message> {
    // Try to enrich context from the task
    let enriched = match enricher.enrich_from_query(&input.task).await {
        Ok(enriched) if !enriched.entities.is_empty() => {
            debug!(
                entities = enriched.entities.len(),
                relationships = enriched.relationships.len(),
                matched_terms = ?enriched.matched_terms,
                "Enriched task context from knowledge graph"
            );
            Some(enriched)
        }
        Ok(_) => {
            debug!("No relevant entities found for task");
            None
        }
        Err(e) => {
            debug!(error = %e, "Failed to enrich context, continuing without");
            None
        }
    };

    build_enriched_agent_messages(
        system_prompt,
        &context.inherited_messages,
        &input.context_messages,
        &input.task,
        enriched.as_ref(),
    )
}

/// Build enriched messages with pre-computed context
///
/// Use this when you've already computed the enriched context
/// (e.g., for multiple agents sharing the same context).
pub fn build_messages_with_enrichment(
    system_prompt: &str,
    input: &AgentInput,
    context: &AgentContext,
    enriched: &EnrichedContext,
) -> Vec<Message> {
    build_enriched_agent_messages(
        system_prompt,
        &context.inherited_messages,
        &input.context_messages,
        &input.task,
        Some(enriched),
    )
}

/// Configuration for context-aware message building
#[derive(Debug, Clone)]
pub struct EnrichedMessageConfig {
    /// Configuration for the enricher
    pub enrichment_config: EnrichmentConfig,
    /// Whether to include enrichment in system prompt (vs. as a separate message)
    pub inline_in_system: bool,
    /// Maximum tokens for enrichment context
    pub max_enrichment_tokens: usize,
}

impl Default for EnrichedMessageConfig {
    fn default() -> Self {
        Self {
            enrichment_config: EnrichmentConfig::default(),
            inline_in_system: true,
            max_enrichment_tokens: 500,
        }
    }
}

impl EnrichedMessageConfig {
    /// Create a minimal config for quick enrichment
    pub fn minimal() -> Self {
        Self {
            enrichment_config: EnrichmentConfig::minimal(),
            inline_in_system: true,
            max_enrichment_tokens: 200,
        }
    }

    /// Create a comprehensive config for detailed context
    pub fn comprehensive() -> Self {
        Self {
            enrichment_config: EnrichmentConfig::comprehensive(),
            inline_in_system: true,
            max_enrichment_tokens: 800,
        }
    }
}

/// Builder for creating enriched messages with fine-grained control
pub struct EnrichedMessageBuilder<'a, R: KnowledgeGraphRepository> {
    system_prompt: &'a str,
    input: &'a AgentInput,
    context: &'a AgentContext,
    enricher: Option<&'a ContextEnricher<R>>,
    config: EnrichedMessageConfig,
    extra_context: Vec<Message>,
}

impl<'a, R: KnowledgeGraphRepository> EnrichedMessageBuilder<'a, R> {
    /// Create a new message builder
    pub fn new(system_prompt: &'a str, input: &'a AgentInput, context: &'a AgentContext) -> Self {
        Self {
            system_prompt,
            input,
            context,
            enricher: None,
            config: EnrichedMessageConfig::default(),
            extra_context: Vec::new(),
        }
    }

    /// Add a context enricher
    pub fn with_enricher(mut self, enricher: &'a ContextEnricher<R>) -> Self {
        self.enricher = Some(enricher);
        self
    }

    /// Set the enrichment configuration
    pub fn with_config(mut self, config: EnrichedMessageConfig) -> Self {
        self.config = config;
        self
    }

    /// Add extra context messages
    pub fn with_extra_context(mut self, messages: Vec<Message>) -> Self {
        self.extra_context = messages;
        self
    }

    /// Build the messages (async for enrichment)
    pub async fn build(self) -> Vec<Message> {
        let enriched = if let Some(enricher) = self.enricher {
            match enricher.enrich_from_query(&self.input.task).await {
                Ok(e) if !e.entities.is_empty() => Some(e),
                _ => None,
            }
        } else {
            None
        };

        let mut messages = build_enriched_agent_messages(
            self.system_prompt,
            &self.context.inherited_messages,
            &self.input.context_messages,
            &self.input.task,
            enriched.as_ref(),
        );

        // Insert extra context before the final user message
        if !self.extra_context.is_empty() {
            let insert_pos = messages.len().saturating_sub(1);
            for (i, msg) in self.extra_context.into_iter().enumerate() {
                messages.insert(insert_pos + i, msg);
            }
        }

        messages
    }

    /// Build without enrichment (sync version)
    pub fn build_sync(self) -> Vec<Message> {
        let mut messages = build_agent_messages(
            self.system_prompt,
            &self.context.inherited_messages,
            &self.input.context_messages,
            &self.input.task,
        );

        // Insert extra context before the final user message
        if !self.extra_context.is_empty() {
            let insert_pos = messages.len().saturating_sub(1);
            for (i, msg) in self.extra_context.into_iter().enumerate() {
                messages.insert(insert_pos + i, msg);
            }
        }

        messages
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::knowledge::{
        EnrichedContext, EnrichmentStats, EntityContext, EntityType, KnowledgeEntity,
    };

    #[test]
    fn test_build_agent_messages() {
        let messages = build_agent_messages(
            "You are a helpful assistant",
            &[Message::user("previous context")],
            &[Message::assistant("some response")],
            "Do something",
        );

        assert_eq!(messages.len(), 4);
        // First message should be system
        assert!(messages[0].content.contains("helpful assistant"));
        // Last message should be the task
        assert!(messages[3].content.contains("Do something"));
    }

    #[test]
    fn test_build_agent_messages_empty_context() {
        let messages = build_agent_messages("System prompt", &[], &[], "Task");

        assert_eq!(messages.len(), 2);
    }

    #[test]
    fn test_build_enriched_messages_with_context() {
        let enriched = EnrichedContext {
            entities: vec![EntityContext {
                entity: KnowledgeEntity::new("tokio", EntityType::Library)
                    .with_description("Async runtime for Rust"),
                distance: 0,
                relevance_reason: "Direct match for library".into(),
                related_skills: vec!["skill-1".into()],
            }],
            relationships: vec![],
            formatted_context:
                "## Related Knowledge\n\n### Libraries\n- **tokio**: Async runtime for Rust\n"
                    .into(),
            matched_terms: vec!["tokio".into()],
            stats: EnrichmentStats::default(),
        };

        let messages = build_enriched_agent_messages(
            "You are a code assistant",
            &[],
            &[],
            "Write async code",
            Some(&enriched),
        );

        assert_eq!(messages.len(), 2);
        // System prompt should contain the enriched context
        assert!(messages[0].content.contains("Related Knowledge"));
        assert!(messages[0].content.contains("tokio"));
        // Task should be the last message
        assert!(messages[1].content.contains("async code"));
    }

    #[test]
    fn test_build_enriched_messages_without_context() {
        let messages = build_enriched_agent_messages(
            "You are a code assistant",
            &[],
            &[],
            "Write some code",
            None,
        );

        assert_eq!(messages.len(), 2);
        // System prompt should not contain enriched context
        assert!(!messages[0].content.contains("Related Knowledge"));
    }

    #[test]
    fn test_enriched_message_config_minimal() {
        let config = EnrichedMessageConfig::minimal();
        assert!(config.inline_in_system);
        assert_eq!(config.max_enrichment_tokens, 200);
    }

    #[test]
    fn test_enriched_message_config_comprehensive() {
        let config = EnrichedMessageConfig::comprehensive();
        assert!(config.inline_in_system);
        assert_eq!(config.max_enrichment_tokens, 800);
    }
}
