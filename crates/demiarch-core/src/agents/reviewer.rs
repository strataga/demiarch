//! Reviewer agent - code review and validation (Level 3)
//!
//! The Reviewer is a leaf agent that reviews generated code for quality.

use std::future::Future;
use std::pin::Pin;
use std::sync::atomic::{AtomicU8, Ordering};

use serde::{Deserialize, Serialize};
use tracing::{debug, info};

use super::context::AgentContext;
use super::traits::{
    Agent, AgentArtifact, AgentCapability, AgentInput, AgentResult, AgentStatus,
};
use super::AgentType;
use crate::error::Result;
use crate::llm::Message;

/// Severity of a review issue
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum IssueSeverity {
    /// Critical issue that must be fixed
    Critical,
    /// Major issue that should be fixed
    Major,
    /// Minor issue or style suggestion
    Minor,
    /// Informational note
    Info,
}

impl std::fmt::Display for IssueSeverity {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Critical => write!(f, "critical"),
            Self::Major => write!(f, "major"),
            Self::Minor => write!(f, "minor"),
            Self::Info => write!(f, "info"),
        }
    }
}

/// A single review issue
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReviewIssue {
    /// Severity of the issue
    pub severity: IssueSeverity,
    /// Category of the issue
    pub category: String,
    /// Description of the issue
    pub description: String,
    /// File location (if applicable)
    pub file: Option<String>,
    /// Line number (if applicable)
    pub line: Option<usize>,
    /// Suggested fix
    pub suggestion: Option<String>,
}

/// Complete review result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CodeReview {
    /// Whether the code passed review
    pub approved: bool,
    /// Overall summary
    pub summary: String,
    /// List of issues found
    pub issues: Vec<ReviewIssue>,
    /// Positive aspects noted
    pub strengths: Vec<String>,
}

impl CodeReview {
    /// Create an approved review
    pub fn approved(summary: impl Into<String>) -> Self {
        Self {
            approved: true,
            summary: summary.into(),
            issues: Vec::new(),
            strengths: Vec::new(),
        }
    }

    /// Create a review with issues
    pub fn with_issues(summary: impl Into<String>, issues: Vec<ReviewIssue>) -> Self {
        let has_blocking = issues.iter().any(|i| {
            matches!(i.severity, IssueSeverity::Critical | IssueSeverity::Major)
        });

        Self {
            approved: !has_blocking,
            summary: summary.into(),
            issues,
            strengths: Vec::new(),
        }
    }

    /// Add strengths to the review
    pub fn with_strengths(mut self, strengths: Vec<String>) -> Self {
        self.strengths = strengths;
        self
    }

    /// Count issues by severity
    pub fn issue_count(&self, severity: IssueSeverity) -> usize {
        self.issues.iter().filter(|i| i.severity == severity).count()
    }
}

/// Reviewer agent - reviews code for quality and correctness
///
/// The Reviewer:
/// - Analyzes code for bugs, security issues, and best practices
/// - Provides actionable feedback on code quality
/// - Determines whether code is ready for production
/// - Is a leaf agent (cannot spawn children)
pub struct ReviewerAgent {
    /// Current execution status
    status: AtomicU8,
    /// Available capabilities
    capabilities: Vec<AgentCapability>,
}

impl ReviewerAgent {
    /// Create a new Reviewer agent
    pub fn new() -> Self {
        Self {
            status: AtomicU8::new(AgentStatus::Ready as u8),
            capabilities: vec![AgentCapability::CodeReview, AgentCapability::FileRead],
        }
    }

    /// Execute the review task
    async fn review(&self, input: AgentInput, context: AgentContext) -> Result<AgentResult> {
        info!(
            agent_id = %context.id,
            path = %context.path,
            "Reviewer starting code review"
        );

        // Register with the shared state
        context.register().await;

        // Update status to running
        self.set_status(AgentStatus::Running);
        context.update_status(AgentStatus::Running).await;

        // Build messages for the LLM
        let messages = self.build_messages(&input, &context);

        // Call the LLM to review the code
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
            "Reviewer received LLM response"
        );

        // Parse the review from the response
        let review = self.parse_review(&response.content);

        // Build result with review artifact
        let review_json = serde_json::to_string_pretty(&review).unwrap_or_default();
        let result = AgentResult::success(&response.content)
            .with_tokens(response.tokens_used)
            .with_artifact(AgentArtifact::review("code-review", &review_json));

        // Mark as completed
        self.set_status(AgentStatus::Completed);
        context.complete(result.clone()).await;

        info!(
            agent_id = %context.id,
            tokens = response.tokens_used,
            approved = review.approved,
            issues = review.issues.len(),
            "Reviewer completed"
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

    /// Parse the LLM response into a CodeReview
    fn parse_review(&self, response: &str) -> CodeReview {
        // Try to parse JSON if present
        if let Some(json_start) = response.find('{') {
            if let Some(json_end) = response.rfind('}') {
                let json_str = &response[json_start..=json_end];
                if let Ok(parsed) = serde_json::from_str::<CodeReview>(json_str) {
                    return parsed;
                }
            }
        }

        // Fall back to heuristic parsing
        let lower = response.to_lowercase();

        // Determine if response indicates approval
        let text_approved = !lower.contains("critical")
            && !lower.contains("must fix")
            && !lower.contains("blocking")
            && (lower.contains("approved") || lower.contains("lgtm") || lower.contains("looks good"));

        let mut issues = Vec::new();

        // Look for issue patterns
        if lower.contains("error") || lower.contains("bug") {
            issues.push(ReviewIssue {
                severity: IssueSeverity::Major,
                category: "bug".to_string(),
                description: "Potential bug identified in the code".to_string(),
                file: None,
                line: None,
                suggestion: None,
            });
        }

        if lower.contains("security") || lower.contains("vulnerability") {
            issues.push(ReviewIssue {
                severity: IssueSeverity::Critical,
                category: "security".to_string(),
                description: "Security concern identified".to_string(),
                file: None,
                line: None,
                suggestion: None,
            });
        }

        if lower.contains("style") || lower.contains("naming") {
            issues.push(ReviewIssue {
                severity: IssueSeverity::Minor,
                category: "style".to_string(),
                description: "Style or naming suggestion".to_string(),
                file: None,
                line: None,
                suggestion: None,
            });
        }

        // Create review - use text_approved only if no issues were found
        let mut review = CodeReview::with_issues(response.to_string(), issues);
        // If no issues but text indicates disapproval, mark as not approved
        if review.issues.is_empty() && !text_approved {
            review.approved = false;
        }
        review
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

impl Default for ReviewerAgent {
    fn default() -> Self {
        Self::new()
    }
}

impl Agent for ReviewerAgent {
    fn agent_type(&self) -> AgentType {
        AgentType::Reviewer
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
        Box::pin(self.review(input, context))
    }

    fn system_prompt(&self) -> String {
        r#"You are the Reviewer agent in a hierarchical code generation system.

Your role is to:
1. Review generated code for bugs, security issues, and quality
2. Identify potential improvements and best practices violations
3. Provide actionable feedback with specific suggestions
4. Determine if code is production-ready

Review Criteria:
- **Correctness**: Does the code work as intended?
- **Security**: Are there any security vulnerabilities?
- **Performance**: Are there obvious performance issues?
- **Readability**: Is the code easy to understand?
- **Maintainability**: Will it be easy to modify later?
- **Best Practices**: Does it follow language/framework conventions?

Issue Severity:
- **critical**: Must fix before merging (security, data loss)
- **major**: Should fix (bugs, significant issues)
- **minor**: Nice to fix (style, minor improvements)
- **info**: Informational notes

Output Format (JSON when possible):
```json
{
  "approved": true,
  "summary": "Overall assessment",
  "issues": [
    {
      "severity": "major",
      "category": "bug",
      "description": "What's wrong",
      "file": "path/to/file.rs",
      "line": 42,
      "suggestion": "How to fix it"
    }
  ],
  "strengths": ["Good error handling", "Clear naming"]
}
```

Be constructive and specific. Focus on what matters most."#
            .to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_reviewer_creation() {
        let reviewer = ReviewerAgent::new();
        assert_eq!(reviewer.agent_type(), AgentType::Reviewer);
        assert_eq!(reviewer.status(), AgentStatus::Ready);
        assert!(reviewer
            .capabilities()
            .contains(&AgentCapability::CodeReview));
    }

    #[test]
    fn test_reviewer_default() {
        let reviewer = ReviewerAgent::default();
        assert_eq!(reviewer.agent_type(), AgentType::Reviewer);
    }

    #[test]
    fn test_reviewer_is_leaf() {
        let reviewer = ReviewerAgent::new();
        assert_eq!(reviewer.max_child_depth(), 0);
    }

    #[test]
    fn test_code_review_approved() {
        let review = CodeReview::approved("Code looks good");
        assert!(review.approved);
        assert!(review.issues.is_empty());
    }

    #[test]
    fn test_code_review_with_issues() {
        let issues = vec![
            ReviewIssue {
                severity: IssueSeverity::Minor,
                category: "style".to_string(),
                description: "Use snake_case".to_string(),
                file: None,
                line: None,
                suggestion: Some("Rename variable".to_string()),
            },
        ];

        let review = CodeReview::with_issues("Minor issues found", issues);
        assert!(review.approved); // Minor issues don't block
        assert_eq!(review.issues.len(), 1);
        assert_eq!(review.issue_count(IssueSeverity::Minor), 1);
    }

    #[test]
    fn test_code_review_blocking_issues() {
        let issues = vec![ReviewIssue {
            severity: IssueSeverity::Critical,
            category: "security".to_string(),
            description: "SQL injection vulnerability".to_string(),
            file: Some("src/db.rs".to_string()),
            line: Some(42),
            suggestion: Some("Use parameterized queries".to_string()),
        }];

        let review = CodeReview::with_issues("Critical issues found", issues);
        assert!(!review.approved); // Critical issues block
    }

    #[test]
    fn test_parse_review_json() {
        let reviewer = ReviewerAgent::new();
        let json_response = r#"
        Here's my review:
        {
            "approved": true,
            "summary": "Code looks good overall",
            "issues": [],
            "strengths": ["Good error handling"]
        }
        "#;

        let review = reviewer.parse_review(json_response);
        assert!(review.approved);
        assert_eq!(review.strengths.len(), 1);
    }

    #[test]
    fn test_parse_review_heuristic() {
        let reviewer = ReviewerAgent::new();

        // Should be approved
        let response = "LGTM! The code looks good and follows best practices.";
        let review = reviewer.parse_review(response);
        assert!(review.approved);

        // Should not be approved (security concern)
        let response = "Found a security vulnerability in the authentication code.";
        let review = reviewer.parse_review(response);
        assert!(!review.approved);
        assert!(review.issues.iter().any(|i| i.category == "security"));
    }

    #[test]
    fn test_issue_severity_display() {
        assert_eq!(IssueSeverity::Critical.to_string(), "critical");
        assert_eq!(IssueSeverity::Major.to_string(), "major");
        assert_eq!(IssueSeverity::Minor.to_string(), "minor");
        assert_eq!(IssueSeverity::Info.to_string(), "info");
    }

    #[test]
    fn test_system_prompt() {
        let reviewer = ReviewerAgent::new();
        let prompt = reviewer.system_prompt();
        assert!(prompt.contains("Reviewer"));
        assert!(prompt.contains("security"));
    }
}
