//! Tester agent - test generation (Level 3)
//!
//! The Tester is a leaf agent that generates tests for code.

use std::future::Future;
use std::pin::Pin;
use std::sync::atomic::{AtomicU8, Ordering};

use serde::{Deserialize, Serialize};
use tracing::{debug, info};

use super::AgentType;
use super::context::AgentContext;
use super::traits::{Agent, AgentArtifact, AgentCapability, AgentInput, AgentResult, AgentStatus};
use crate::error::Result;
use crate::llm::Message;

/// Type of test generated
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TestType {
    /// Unit test for individual functions
    Unit,
    /// Integration test for component interaction
    Integration,
    /// End-to-end test for full workflows
    EndToEnd,
    /// Property-based/fuzz test
    Property,
    /// Snapshot test for output comparison
    Snapshot,
}

impl std::fmt::Display for TestType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Unit => write!(f, "unit"),
            Self::Integration => write!(f, "integration"),
            Self::EndToEnd => write!(f, "e2e"),
            Self::Property => write!(f, "property"),
            Self::Snapshot => write!(f, "snapshot"),
        }
    }
}

/// A generated test case
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TestCase {
    /// Name of the test
    pub name: String,
    /// Type of test
    pub test_type: TestType,
    /// Description of what the test verifies
    pub description: String,
    /// The test code
    pub code: String,
    /// Target file/function being tested
    pub target: Option<String>,
}

/// Test generation result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TestSuite {
    /// Name of the test suite
    pub name: String,
    /// Generated test cases
    pub tests: Vec<TestCase>,
    /// Coverage notes
    pub coverage_notes: Vec<String>,
}

impl TestSuite {
    /// Create a new test suite
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            tests: Vec::new(),
            coverage_notes: Vec::new(),
        }
    }

    /// Add a test case
    pub fn with_test(mut self, test: TestCase) -> Self {
        self.tests.push(test);
        self
    }

    /// Count tests by type
    pub fn test_count(&self, test_type: TestType) -> usize {
        self.tests
            .iter()
            .filter(|t| t.test_type == test_type)
            .count()
    }
}

/// Tester agent - generates tests for implementations
///
/// The Tester:
/// - Analyzes code to determine what should be tested
/// - Generates comprehensive test suites
/// - Includes edge cases and error scenarios
/// - Is a leaf agent (cannot spawn children)
pub struct TesterAgent {
    /// Current execution status
    status: AtomicU8,
    /// Available capabilities
    capabilities: Vec<AgentCapability>,
}

impl TesterAgent {
    /// Create a new Tester agent
    pub fn new() -> Self {
        Self {
            status: AtomicU8::new(AgentStatus::Ready as u8),
            capabilities: vec![AgentCapability::TestGeneration, AgentCapability::FileRead],
        }
    }

    /// Execute the test generation task
    async fn generate_tests(
        &self,
        input: AgentInput,
        context: AgentContext,
    ) -> Result<AgentResult> {
        info!(
            agent_id = %context.id,
            path = %context.path,
            "Tester starting test generation"
        );

        // Register with the shared state
        context.register().await;

        // Update status to running
        self.set_status(AgentStatus::Running);
        context.update_status(AgentStatus::Running).await;

        // Build messages for the LLM
        let messages = self.build_messages(&input, &context);

        // Call the LLM to generate tests
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

        debug!(
            tokens = response.tokens_used,
            "Tester received LLM response"
        );

        // Extract test code blocks from the response
        let test_blocks = extract_test_blocks(&response.content);

        // Build result with test artifacts
        let mut result = AgentResult::success(&response.content).with_tokens(response.tokens_used);

        for (i, (language, code)) in test_blocks.iter().enumerate() {
            let filename = format!("test-{}.{}", i + 1, language_test_extension(language));
            result = result.with_artifact(AgentArtifact::test(&filename, code));
        }

        // Mark as completed
        self.set_status(AgentStatus::Completed);
        context.complete(result.clone()).await;

        info!(
            agent_id = %context.id,
            tokens = response.tokens_used,
            tests = test_blocks.len(),
            "Tester completed"
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

impl Default for TesterAgent {
    fn default() -> Self {
        Self::new()
    }
}

impl Agent for TesterAgent {
    fn agent_type(&self) -> AgentType {
        AgentType::Tester
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
        Box::pin(self.generate_tests(input, context))
    }

    fn system_prompt(&self) -> String {
        r#"You are the Tester agent in a hierarchical code generation system.

Your role is to:
1. Generate comprehensive test suites for code
2. Cover happy paths, edge cases, and error scenarios
3. Write clear, maintainable tests
4. Ensure tests are independent and repeatable

Test Types to Consider:
- **Unit Tests**: Test individual functions in isolation
- **Integration Tests**: Test component interactions
- **Edge Cases**: Empty inputs, null values, boundaries
- **Error Cases**: Invalid inputs, exceptions, failures

Guidelines:
- Each test should verify one specific behavior
- Use descriptive test names that explain what's being tested
- Include setup (arrange), action (act), and verification (assert)
- Mock external dependencies when appropriate
- Test both success and failure paths

Output Format:
- Wrap test code in markdown code blocks with language specification
- Use standard testing frameworks for the language:
  - Rust: #[test] with assert! macros
  - Python: pytest or unittest
  - JavaScript/TypeScript: Jest or Vitest
  - Go: testing package

Example structure for Rust:
```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_function_does_expected_thing() {
        // Arrange
        let input = create_test_input();

        // Act
        let result = function_under_test(input);

        // Assert
        assert_eq!(result, expected_value);
    }

    #[test]
    fn test_function_handles_error_case() {
        // Test error handling
    }
}
```

Focus on meaningful tests that catch real bugs."#
            .to_string()
    }
}

/// Extract test code blocks from markdown-formatted text
fn extract_test_blocks(content: &str) -> Vec<(String, String)> {
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

/// Get file extension for test files
fn language_test_extension(language: &str) -> &str {
    match language.to_lowercase().as_str() {
        "rust" | "rs" => "rs",
        "python" | "py" => "py",
        "javascript" | "js" => "test.js",
        "typescript" | "ts" => "test.ts",
        "go" | "golang" => "test.go",
        "java" => "java",
        "csharp" | "c#" => "cs",
        "ruby" | "rb" => "rb",
        "php" => "php",
        _ => "txt",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_tester_creation() {
        let tester = TesterAgent::new();
        assert_eq!(tester.agent_type(), AgentType::Tester);
        assert_eq!(tester.status(), AgentStatus::Ready);
        assert!(
            tester
                .capabilities()
                .contains(&AgentCapability::TestGeneration)
        );
    }

    #[test]
    fn test_tester_default() {
        let tester = TesterAgent::default();
        assert_eq!(tester.agent_type(), AgentType::Tester);
    }

    #[test]
    fn test_tester_is_leaf() {
        let tester = TesterAgent::new();
        assert_eq!(tester.max_child_depth(), 0);
    }

    #[test]
    fn test_test_suite_creation() {
        let suite = TestSuite::new("MyTests")
            .with_test(TestCase {
                name: "test_add".to_string(),
                test_type: TestType::Unit,
                description: "Tests addition".to_string(),
                code: "assert_eq!(1 + 1, 2);".to_string(),
                target: Some("add".to_string()),
            })
            .with_test(TestCase {
                name: "test_integration".to_string(),
                test_type: TestType::Integration,
                description: "Tests integration".to_string(),
                code: "// integration test".to_string(),
                target: None,
            });

        assert_eq!(suite.tests.len(), 2);
        assert_eq!(suite.test_count(TestType::Unit), 1);
        assert_eq!(suite.test_count(TestType::Integration), 1);
    }

    #[test]
    fn test_extract_test_blocks() {
        let content = r#"
Here are the tests:

```rust
#[test]
fn test_example() {
    assert!(true);
}
```

And Python tests:

```python
def test_example():
    assert True
```
"#;

        let blocks = extract_test_blocks(content);
        assert_eq!(blocks.len(), 2);
        assert_eq!(blocks[0].0, "rust");
        assert!(blocks[0].1.contains("#[test]"));
        assert_eq!(blocks[1].0, "python");
        assert!(blocks[1].1.contains("def test_example"));
    }

    #[test]
    fn test_test_type_display() {
        assert_eq!(TestType::Unit.to_string(), "unit");
        assert_eq!(TestType::Integration.to_string(), "integration");
        assert_eq!(TestType::EndToEnd.to_string(), "e2e");
        assert_eq!(TestType::Property.to_string(), "property");
        assert_eq!(TestType::Snapshot.to_string(), "snapshot");
    }

    #[test]
    fn test_language_test_extension() {
        assert_eq!(language_test_extension("rust"), "rs");
        assert_eq!(language_test_extension("javascript"), "test.js");
        assert_eq!(language_test_extension("typescript"), "test.ts");
        assert_eq!(language_test_extension("go"), "test.go");
    }

    #[test]
    fn test_system_prompt() {
        let tester = TesterAgent::new();
        let prompt = tester.system_prompt();
        assert!(prompt.contains("Tester"));
        assert!(prompt.contains("test"));
        assert!(prompt.contains("Unit"));
    }
}
