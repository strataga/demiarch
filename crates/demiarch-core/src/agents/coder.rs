//! Coder agent - code generation (Level 3)
//!
//! The Coder is a leaf agent that generates code based on task descriptions.

use std::future::Future;
use std::pin::Pin;

use tracing::{debug, info};

use super::code_extraction::{
    extract_code_blocks, extract_files_from_response, language_to_extension,
};
use super::context::AgentContext;
use super::message_builder::build_messages_from_input;
use super::status::StatusTracker;
use super::traits::{Agent, AgentArtifact, AgentCapability, AgentInput, AgentResult, AgentStatus};
use super::AgentType;
use crate::error::Result;

/// Coder agent - generates code implementations
///
/// The Coder:
/// - Receives specific coding tasks from the Planner
/// - Generates code following best practices
/// - Produces code artifacts for review and testing
/// - Is a leaf agent (cannot spawn children)
pub struct CoderAgent {
    /// Current execution status
    status: StatusTracker,
    /// Available capabilities
    capabilities: Vec<AgentCapability>,
}

impl CoderAgent {
    /// Create a new Coder agent
    pub fn new() -> Self {
        Self {
            status: StatusTracker::new(),
            capabilities: vec![
                AgentCapability::CodeGeneration,
                AgentCapability::FileWrite,
                AgentCapability::FileRead,
            ],
        }
    }

    /// Execute the coding task
    async fn code(&self, input: AgentInput, context: AgentContext) -> Result<AgentResult> {
        // Check for cancellation at start
        if context.is_cancelled() {
            self.status.set(AgentStatus::Cancelled);
            return Ok(AgentResult::failure("Cancelled"));
        }

        info!(
            agent_id = %context.id,
            path = %context.path,
            "Coder starting code generation"
        );

        // Register with the shared state (include task for monitoring)
        context.register_with_task(Some(&input.task)).await;

        // Update status to running
        self.status.set(AgentStatus::Running);
        context.update_status(AgentStatus::Running).await;

        // Build messages for the LLM
        let messages = build_messages_from_input(&self.system_prompt(), &input, &context);

        // Call the LLM to generate code
        let llm_client = context.llm_client();
        let response = match llm_client.complete(messages, None).await {
            Ok(resp) => resp,
            Err(e) => {
                self.status.set(AgentStatus::Failed);
                let result = AgentResult::failure(format!("LLM call failed: {}", e));
                context.complete(result.clone()).await;
                return Ok(result);
            }
        };

        debug!(tokens = response.tokens_used, "Coder received LLM response");

        // Build result with artifacts
        let mut result = AgentResult::success(&response.content).with_tokens(response.tokens_used);
        let file_count;

        // First, try to extract files with proper paths from the response
        let extracted_files = extract_files_from_response(&response.content);

        if !extracted_files.is_empty() {
            // Use properly named files from the response
            for file in &extracted_files {
                let path_str = file.path.to_string_lossy().to_string();
                result = result.with_artifact(AgentArtifact::code(&path_str, &file.content));
            }
            file_count = extracted_files.len();
            info!(
                agent_id = %context.id,
                files = extracted_files.len(),
                "Coder extracted files with paths"
            );
        } else {
            // Fallback: extract code blocks and generate generic filenames
            let code_blocks = extract_code_blocks(&response.content);
            for (i, block) in code_blocks.iter().enumerate() {
                let filename = format!(
                    "generated-{}.{}",
                    i + 1,
                    language_to_extension(&block.language)
                );
                result = result.with_artifact(AgentArtifact::code(&filename, &block.code));
            }
            file_count = code_blocks.len();
        }

        // Mark as completed
        self.status.set(AgentStatus::Completed);
        context.complete(result.clone()).await;

        info!(
            agent_id = %context.id,
            tokens = response.tokens_used,
            files = file_count,
            "Coder completed"
        );

        Ok(result)
    }
}

impl Default for CoderAgent {
    fn default() -> Self {
        Self::new()
    }
}

impl Agent for CoderAgent {
    fn agent_type(&self) -> AgentType {
        AgentType::Coder
    }

    fn capabilities(&self) -> &[AgentCapability] {
        &self.capabilities
    }

    fn status(&self) -> AgentStatus {
        self.status.get()
    }

    fn execute(
        &self,
        input: AgentInput,
        context: AgentContext,
    ) -> Pin<Box<dyn Future<Output = Result<AgentResult>> + Send + '_>> {
        Box::pin(self.code(input, context))
    }

    fn system_prompt(&self) -> String {
        r#"You are the Coder agent in a hierarchical code generation system.

Your role is to:
1. Generate high-quality, production-ready code
2. Follow best practices for the target language/framework
3. Write clean, readable, and maintainable code
4. Include appropriate comments for complex logic

Guidelines:
- Use descriptive variable and function names
- Handle errors appropriately
- Follow the project's existing conventions when applicable
- Prefer simplicity over cleverness
- Write code that is easy to test

Output Format:
- Wrap code in markdown code blocks with language specification
- Example: ```rust
  fn example() { }
  ```
- Include file paths when creating new files
- Explain your implementation choices briefly

Do not include tests - those will be generated by the Tester agent.
Focus on clean, working implementation code."#
            .to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_coder_creation() {
        let coder = CoderAgent::new();
        assert_eq!(coder.agent_type(), AgentType::Coder);
        assert_eq!(coder.status(), AgentStatus::Ready);
        assert!(coder
            .capabilities()
            .contains(&AgentCapability::CodeGeneration));
    }

    #[test]
    fn test_coder_default() {
        let coder = CoderAgent::default();
        assert_eq!(coder.agent_type(), AgentType::Coder);
    }

    #[test]
    fn test_coder_is_leaf() {
        let coder = CoderAgent::new();
        assert_eq!(coder.max_child_depth(), 0);
    }

    #[test]
    fn test_extract_code_blocks() {
        let content = r#"
Here's the implementation:

```rust
fn hello() {
    println!("Hello, world!");
}
```

And some JavaScript:

```javascript
function greet() {
    console.log("Hello!");
}
```
"#;

        let blocks = extract_code_blocks(content);
        assert_eq!(blocks.len(), 2);
        assert_eq!(blocks[0].language, "rust");
        assert!(blocks[0].code.contains("fn hello()"));
        assert_eq!(blocks[1].language, "javascript");
        assert!(blocks[1].code.contains("function greet()"));
    }

    #[test]
    fn test_extract_code_blocks_empty() {
        let content = "No code blocks here.";
        let blocks = extract_code_blocks(content);
        assert!(blocks.is_empty());
    }

    #[test]
    fn test_language_extension() {
        assert_eq!(language_to_extension("rust"), "rs");
        assert_eq!(language_to_extension("Rust"), "rs");
        assert_eq!(language_to_extension("python"), "py");
        assert_eq!(language_to_extension("javascript"), "js");
        assert_eq!(language_to_extension("typescript"), "ts");
        assert_eq!(language_to_extension("unknown"), "txt");
    }

    #[test]
    fn test_system_prompt() {
        let coder = CoderAgent::new();
        let prompt = coder.system_prompt();
        assert!(prompt.contains("Coder"));
        assert!(prompt.contains("code"));
    }
}
