//! Message building utilities for agents
//!
//! Provides common message construction patterns used by all agents.

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

#[cfg(test)]
mod tests {
    use super::*;

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
        let messages = build_agent_messages(
            "System prompt",
            &[],
            &[],
            "Task",
        );

        assert_eq!(messages.len(), 2);
    }
}
