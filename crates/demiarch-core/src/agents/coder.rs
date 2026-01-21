//! Coder agent - code generation (Level 3)
//!
//! The Coder is a leaf agent that generates code based on task descriptions.

use std::future::Future;
use std::pin::Pin;
use std::sync::atomic::{AtomicU8, Ordering};

use tracing::{debug, info};

use super::AgentType;
use super::context::AgentContext;
use super::traits::{Agent, AgentArtifact, AgentCapability, AgentInput, AgentResult, AgentStatus};
use crate::error::Result;
use crate::llm::Message;

/// Coder agent - generates code implementations
///
/// The Coder:
/// - Receives specific coding tasks from the Planner
/// - Generates code following best practices
/// - Produces code artifacts for review and testing
/// - Is a leaf agent (cannot spawn children)
pub struct CoderAgent {
    /// Current execution status
    status: AtomicU8,
    /// Available capabilities
    capabilities: Vec<AgentCapability>,
}

impl CoderAgent {
    /// Create a new Coder agent
    pub fn new() -> Self {
        Self {
            status: AtomicU8::new(AgentStatus::Ready as u8),
            capabilities: vec![
                AgentCapability::CodeGeneration,
                AgentCapability::FileWrite,
                AgentCapability::FileRead,
            ],
        }
    }

    /// Execute the coding task
    async fn code(&self, input: AgentInput, context: AgentContext) -> Result<AgentResult> {
        info!(
            agent_id = %context.id,
            path = %context.path,
            "Coder starting code generation"
        );

        // Register with the shared state
        context.register().await;

        // Update status to running
        self.set_status(AgentStatus::Running);
        context.update_status(AgentStatus::Running).await;

        // Build messages for the LLM
        let messages = self.build_messages(&input, &context);

        // Call the LLM to generate code
        let llm_client = context.llm_client();
        let response = match llm_client.complete(messages, None).await {
            Ok(resp) => resp,
            Err(e) => {
                self.set_status(AgentStatus::Failed);
                let result = AgentResult::failure(format!("LLM call failed: {}", e));
                context.complete(result.clone()).await;
                return Ok(result);
            }
        };

        debug!(tokens = response.tokens_used, "Coder received LLM response");

        // Extract code blocks from the response
        let code_blocks = extract_code_blocks(&response.content);

        // Build result with artifacts
        let mut result = AgentResult::success(&response.content).with_tokens(response.tokens_used);

        for (i, (language, code)) in code_blocks.iter().enumerate() {
            let filename = format!("generated-{}.{}", i + 1, language_extension(language));
            result = result.with_artifact(AgentArtifact::code(&filename, code));
        }

        // Mark as completed
        self.set_status(AgentStatus::Completed);
        context.complete(result.clone()).await;

        info!(
            agent_id = %context.id,
            tokens = response.tokens_used,
            code_blocks = code_blocks.len(),
            "Coder completed"
        );

        Ok(result)
    }

    /// Build messages for the LLM call
    fn build_messages(&self, input: &AgentInput, context: &AgentContext) -> Vec<Message> {
        let mut messages = vec![Message::system(self.system_prompt())];

        // Add inherited context
        messages.extend(context.inherited_messages.clone());

        // Add context from input
        messages.extend(input.context_messages.clone());

        // Add the task
        messages.push(Message::user(&input.task));

        messages
    }

    /// Set the agent status
    fn set_status(&self, status: AgentStatus) {
        self.status.store(status as u8, Ordering::SeqCst);
    }

    /// Get the current status
    fn get_status(&self) -> AgentStatus {
        match self.status.load(Ordering::SeqCst) {
            0 => AgentStatus::Ready,
            1 => AgentStatus::Running,
            2 => AgentStatus::WaitingForChildren,
            3 => AgentStatus::Completed,
            4 => AgentStatus::Failed,
            _ => AgentStatus::Cancelled,
        }
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
        self.get_status()
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

/// Extract code blocks from markdown-formatted text
fn extract_code_blocks(content: &str) -> Vec<(String, String)> {
    let mut blocks = Vec::new();
    let mut in_block = false;
    let mut current_language = String::new();
    let mut current_code = String::new();

    for line in content.lines() {
        if line.starts_with("```") {
            if in_block {
                // End of block
                if !current_code.trim().is_empty() {
                    blocks.push((current_language.clone(), current_code.trim().to_string()));
                }
                current_language.clear();
                current_code.clear();
                in_block = false;
            } else {
                // Start of block
                current_language = line.trim_start_matches('`').trim().to_string();
                if current_language.is_empty() {
                    current_language = "txt".to_string();
                }
                in_block = true;
            }
        } else if in_block {
            current_code.push_str(line);
            current_code.push('\n');
        }
    }

    blocks
}

/// Get file extension for a language
fn language_extension(language: &str) -> &str {
    match language.to_lowercase().as_str() {
        "rust" | "rs" => "rs",
        "python" | "py" => "py",
        "javascript" | "js" => "js",
        "typescript" | "ts" => "ts",
        "go" | "golang" => "go",
        "java" => "java",
        "c" => "c",
        "cpp" | "c++" => "cpp",
        "csharp" | "c#" => "cs",
        "ruby" | "rb" => "rb",
        "php" => "php",
        "swift" => "swift",
        "kotlin" | "kt" => "kt",
        "scala" => "scala",
        "html" => "html",
        "css" => "css",
        "scss" | "sass" => "scss",
        "json" => "json",
        "yaml" | "yml" => "yaml",
        "toml" => "toml",
        "sql" => "sql",
        "bash" | "sh" | "shell" => "sh",
        "markdown" | "md" => "md",
        _ => "txt",
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
        assert!(
            coder
                .capabilities()
                .contains(&AgentCapability::CodeGeneration)
        );
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
        assert_eq!(blocks[0].0, "rust");
        assert!(blocks[0].1.contains("fn hello()"));
        assert_eq!(blocks[1].0, "javascript");
        assert!(blocks[1].1.contains("function greet()"));
    }

    #[test]
    fn test_extract_code_blocks_empty() {
        let content = "No code blocks here.";
        let blocks = extract_code_blocks(content);
        assert!(blocks.is_empty());
    }

    #[test]
    fn test_language_extension() {
        assert_eq!(language_extension("rust"), "rs");
        assert_eq!(language_extension("Rust"), "rs");
        assert_eq!(language_extension("python"), "py");
        assert_eq!(language_extension("javascript"), "js");
        assert_eq!(language_extension("typescript"), "ts");
        assert_eq!(language_extension("unknown"), "txt");
    }

    #[test]
    fn test_system_prompt() {
        let coder = CoderAgent::new();
        let prompt = coder.system_prompt();
        assert!(prompt.contains("Coder"));
        assert!(prompt.contains("code"));
    }
}
