//! Document generation commands
//!
//! Auto-generates PRD and architecture documents from project information
//! using LLM integration. Supports versioning and multiple document types.

use std::sync::Arc;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use tracing::{debug, info};
use uuid::Uuid;

use crate::config::Config;
use crate::cost::CostTracker;
use crate::error::{Error, Result};
use crate::llm::{LlmClient, Message};
use crate::storage::Database;

use super::feature::{Feature, FeatureRepository};
use super::project::{Project, ProjectRepository};

// Re-export repository from infrastructure for backwards compatibility
pub use crate::infrastructure::document::DocumentRepository;

/// Document type
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum DocumentType {
    /// Product Requirements Document
    Prd,
    /// Architecture Document
    Architecture,
    /// Design Document
    Design,
    /// Technical Specification
    TechSpec,
    /// Custom document type
    Custom,
}

impl DocumentType {
    /// Convert to string for database storage
    pub fn as_str(&self) -> &'static str {
        match self {
            DocumentType::Prd => "prd",
            DocumentType::Architecture => "architecture",
            DocumentType::Design => "design",
            DocumentType::TechSpec => "tech_spec",
            DocumentType::Custom => "custom",
        }
    }

    /// Parse from database string
    pub fn parse(s: &str) -> Option<Self> {
        match s {
            "prd" => Some(DocumentType::Prd),
            "architecture" => Some(DocumentType::Architecture),
            "design" => Some(DocumentType::Design),
            "tech_spec" => Some(DocumentType::TechSpec),
            "custom" => Some(DocumentType::Custom),
            _ => None,
        }
    }

    /// Get the display name for this document type
    pub fn display_name(&self) -> &'static str {
        match self {
            DocumentType::Prd => "Product Requirements Document",
            DocumentType::Architecture => "Architecture Document",
            DocumentType::Design => "Design Document",
            DocumentType::TechSpec => "Technical Specification",
            DocumentType::Custom => "Custom Document",
        }
    }
}

/// Document status
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum DocumentStatus {
    #[default]
    Draft,
    Review,
    Final,
    Archived,
}

impl DocumentStatus {
    /// Convert to string for database storage
    pub fn as_str(&self) -> &'static str {
        match self {
            DocumentStatus::Draft => "draft",
            DocumentStatus::Review => "review",
            DocumentStatus::Final => "final",
            DocumentStatus::Archived => "archived",
        }
    }

    /// Parse from database string
    pub fn parse(s: &str) -> Option<Self> {
        match s {
            "draft" => Some(DocumentStatus::Draft),
            "review" => Some(DocumentStatus::Review),
            "final" => Some(DocumentStatus::Final),
            "archived" => Some(DocumentStatus::Archived),
            _ => None,
        }
    }
}

/// A generated document
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Document {
    /// Unique document identifier
    pub id: String,
    /// Associated project ID
    pub project_id: String,
    /// Document type
    pub doc_type: DocumentType,
    /// Document title
    pub title: String,
    /// Optional description
    pub description: Option<String>,
    /// Document content (markdown)
    pub content: String,
    /// Content format (markdown or json)
    pub format: String,
    /// Document version number
    pub version: i32,
    /// Document status
    pub status: DocumentStatus,
    /// Model used for generation
    pub model_used: Option<String>,
    /// Tokens used for generation
    pub tokens_used: Option<i32>,
    /// Generation cost in USD
    pub generation_cost_usd: Option<f64>,
    /// When the document was created
    pub created_at: DateTime<Utc>,
    /// When the document was last updated
    pub updated_at: DateTime<Utc>,
}

impl Document {
    /// Create a new document
    pub fn new(
        project_id: impl Into<String>,
        doc_type: DocumentType,
        title: impl Into<String>,
        content: impl Into<String>,
    ) -> Self {
        let now = Utc::now();
        Self {
            id: Uuid::new_v4().to_string(),
            project_id: project_id.into(),
            doc_type,
            title: title.into(),
            description: None,
            content: content.into(),
            format: "markdown".to_string(),
            version: 1,
            status: DocumentStatus::Draft,
            model_used: None,
            tokens_used: None,
            generation_cost_usd: None,
            created_at: now,
            updated_at: now,
        }
    }

    /// Set the description
    pub fn with_description(mut self, description: impl Into<String>) -> Self {
        self.description = Some(description.into());
        self
    }

    /// Set the model used
    pub fn with_model(mut self, model: impl Into<String>) -> Self {
        self.model_used = Some(model.into());
        self
    }

    /// Set the tokens used
    pub fn with_tokens(mut self, tokens: i32) -> Self {
        self.tokens_used = Some(tokens);
        self
    }

    /// Set the generation cost
    pub fn with_cost(mut self, cost: f64) -> Self {
        self.generation_cost_usd = Some(cost);
        self
    }
}

/// A document version for history tracking
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DocumentVersion {
    /// Unique version identifier
    pub id: String,
    /// Parent document ID
    pub document_id: String,
    /// Version number
    pub version_number: i32,
    /// Document content at this version
    pub content: String,
    /// Summary of changes
    pub change_summary: Option<String>,
    /// Model used for this version
    pub model_used: Option<String>,
    /// When this version was created
    pub created_at: DateTime<Utc>,
}

/// Document generator using LLM
pub struct DocumentGenerator {
    llm_client: LlmClient,
}

impl DocumentGenerator {
    /// Create a new document generator
    pub fn new(config: Config, cost_tracker: Option<Arc<CostTracker>>) -> Result<Self> {
        let api_key = config
            .llm
            .resolved_api_key()
            .map_err(|e| Error::ConfigError(e.to_string()))?
            .ok_or_else(|| {
                Error::LLMError(
                    "API key not configured. Set DEMIARCH_API_KEY or OPENROUTER_API_KEY environment variable.".to_string()
                )
            })?;

        let mut builder = LlmClient::builder()
            .config(config.llm.clone())
            .api_key(api_key);

        if let Some(tracker) = cost_tracker {
            builder = builder.cost_tracker(tracker);
        }

        let llm_client = builder.build()?;

        Ok(Self { llm_client })
    }

    /// Generate a PRD for a project
    pub async fn generate_prd(
        &self,
        project: &Project,
        features: &[Feature],
    ) -> Result<GeneratedDocument> {
        info!(project_id = %project.id, "Generating PRD");

        let features_list = features
            .iter()
            .map(|f| {
                format!(
                    "- {} (Priority: {}, Status: {:?})",
                    f.title, f.priority, f.status
                )
            })
            .collect::<Vec<_>>()
            .join("\n");

        let user_prompt = format!(
            r#"Generate a comprehensive Product Requirements Document (PRD) for the following project:

**Project Name**: {}
**Framework/Tech Stack**: {}
**Description**: {}

**Existing Features**:
{}

Please create a complete PRD with the following sections:
1. Executive Summary
2. Problem Statement
3. Target Users & Personas
4. Product Goals & Success Metrics
5. Feature Requirements (detailed)
6. Non-Functional Requirements
7. User Stories
8. Acceptance Criteria
9. Timeline & Milestones
10. Risks & Mitigations
11. Appendix (glossary, references)"#,
            project.name,
            project.framework,
            project
                .description
                .as_deref()
                .unwrap_or("No description provided"),
            if features_list.is_empty() {
                "No features defined yet".to_string()
            } else {
                features_list
            }
        );

        let messages = vec![
            Message::system(PRD_SYSTEM_PROMPT),
            Message::user(user_prompt),
        ];

        debug!(
            message_count = messages.len(),
            "Sending PRD generation request to LLM"
        );

        let response = self.llm_client.complete_with_fallback(messages).await?;

        info!(
            tokens = response.tokens_used,
            model = %response.model,
            "Received PRD response"
        );

        let cost_usd = estimate_cost(
            &response.model,
            response.input_tokens,
            response.output_tokens,
        );

        Ok(GeneratedDocument {
            doc_type: DocumentType::Prd,
            title: format!("{} - Product Requirements Document", project.name),
            content: response.content,
            model: response.model,
            tokens_used: response.tokens_used,
            cost_usd,
        })
    }

    /// Generate an architecture document for a project
    pub async fn generate_architecture(
        &self,
        project: &Project,
        features: &[Feature],
    ) -> Result<GeneratedDocument> {
        info!(project_id = %project.id, "Generating Architecture Document");

        let features_list = features
            .iter()
            .map(|f| {
                format!(
                    "- {}: {}",
                    f.title,
                    f.description.as_deref().unwrap_or("No description")
                )
            })
            .collect::<Vec<_>>()
            .join("\n");

        let user_prompt = format!(
            r#"Generate a comprehensive Architecture Document for the following project:

**Project Name**: {}
**Framework/Tech Stack**: {}
**Repository URL**: {}
**Description**: {}

**Features to Support**:
{}

Please create a complete architecture document with the following sections:
1. Overview & Goals
2. System Context (external integrations, dependencies)
3. High-Level Architecture (with ASCII diagram)
4. Component Architecture (detailed breakdown)
5. Data Architecture (models, storage, flows)
6. API Design (endpoints, contracts)
7. Technology Stack Rationale
8. Security Architecture
9. Scalability & Performance Considerations
10. Deployment Architecture
11. Development Guidelines
12. Future Considerations"#,
            project.name,
            project.framework,
            if project.repo_url.is_empty() {
                "Not specified"
            } else {
                &project.repo_url
            },
            project
                .description
                .as_deref()
                .unwrap_or("No description provided"),
            if features_list.is_empty() {
                "No features defined yet".to_string()
            } else {
                features_list
            }
        );

        let messages = vec![
            Message::system(ARCHITECTURE_SYSTEM_PROMPT),
            Message::user(user_prompt),
        ];

        debug!(
            message_count = messages.len(),
            "Sending architecture generation request to LLM"
        );

        let response = self.llm_client.complete_with_fallback(messages).await?;

        info!(
            tokens = response.tokens_used,
            model = %response.model,
            "Received architecture response"
        );

        let cost_usd = estimate_cost(
            &response.model,
            response.input_tokens,
            response.output_tokens,
        );

        Ok(GeneratedDocument {
            doc_type: DocumentType::Architecture,
            title: format!("{} - Architecture Document", project.name),
            content: response.content,
            model: response.model,
            tokens_used: response.tokens_used,
            cost_usd,
        })
    }
}

/// Result of document generation
#[derive(Debug, Clone)]
pub struct GeneratedDocument {
    /// Document type
    pub doc_type: DocumentType,
    /// Generated title
    pub title: String,
    /// Generated content
    pub content: String,
    /// Model used
    pub model: String,
    /// Tokens used
    pub tokens_used: u32,
    /// Estimated cost in USD
    pub cost_usd: f64,
}

/// Estimate cost based on model and token counts
fn estimate_cost(model: &str, input_tokens: u32, output_tokens: u32) -> f64 {
    let (input_price, output_price) = match model {
        m if m.contains("claude-3-5-sonnet") || m.contains("claude-sonnet-4") => (3.0, 15.0),
        m if m.contains("claude-3-5-haiku") || m.contains("claude-3-haiku") => (0.25, 1.25),
        m if m.contains("claude-3-opus") || m.contains("claude-opus-4") => (15.0, 75.0),
        m if m.contains("gpt-4o") => (2.5, 10.0),
        m if m.contains("gpt-4-turbo") => (10.0, 30.0),
        m if m.contains("gpt-3.5") => (0.5, 1.5),
        _ => (3.0, 15.0),
    };

    let input_cost = (input_tokens as f64 / 1_000_000.0) * input_price;
    let output_cost = (output_tokens as f64 / 1_000_000.0) * output_price;

    input_cost + output_cost
}

/// System prompt for PRD generation
const PRD_SYSTEM_PROMPT: &str = r#"You are an expert product manager with extensive experience writing Product Requirements Documents (PRDs) for software projects. Your PRDs are known for being comprehensive, clear, and actionable.

## Output Guidelines

1. **Structure**: Use clear markdown headings and subheadings
2. **Clarity**: Write in clear, concise language avoiding jargon
3. **Completeness**: Cover all aspects thoroughly but avoid unnecessary padding
4. **Actionability**: Requirements should be specific and testable
5. **User Focus**: Keep the end user's needs at the center

## Format Requirements

- Use markdown formatting throughout
- Include bullet points and numbered lists for clarity
- Create tables where appropriate (e.g., feature prioritization)
- Use bold for important terms and concepts
- Include clear acceptance criteria for each requirement

## Quality Standards

- Each requirement should be SMART (Specific, Measurable, Achievable, Relevant, Time-bound)
- User stories should follow the format: "As a [user type], I want [goal] so that [benefit]"
- Include success metrics that can be objectively measured
- Identify potential risks and provide mitigation strategies"#;

/// System prompt for architecture document generation
const ARCHITECTURE_SYSTEM_PROMPT: &str = r#"You are an expert software architect with deep experience in designing scalable, maintainable systems. Your architecture documents are known for being technically precise while remaining accessible.

## Output Guidelines

1. **Technical Accuracy**: Be precise about technology choices and their implications
2. **Visual Clarity**: Include ASCII diagrams to illustrate architecture
3. **Rationale**: Explain WHY decisions were made, not just WHAT
4. **Trade-offs**: Acknowledge trade-offs in architectural decisions
5. **Practical**: Focus on actionable guidance for developers

## Diagram Format

Use ASCII art for architectural diagrams:

```
┌─────────────────┐     ┌─────────────────┐
│    Frontend     │────▶│    Backend      │
│   (React/Vue)   │     │   (Rust/Node)   │
└─────────────────┘     └────────┬────────┘
                                 │
                                 ▼
                        ┌─────────────────┐
                        │    Database     │
                        │   (PostgreSQL)  │
                        └─────────────────┘
```

## Quality Standards

- Architecture should support the stated non-functional requirements
- Consider security at every layer
- Design for observability (logging, metrics, tracing)
- Plan for graceful degradation and error handling
- Document clear boundaries between components
- Include data flow diagrams where appropriate"#;

// ============================================================================
// Public API functions
// ============================================================================

/// Generate a PRD for a project
pub async fn generate_prd(
    db: &Database,
    project_id: &str,
    cost_tracker: Option<Arc<CostTracker>>,
) -> Result<Document> {
    let config = Config::load().map_err(|e| Error::ConfigError(e.to_string()))?;

    let project_repo = ProjectRepository::new(db);
    let feature_repo = FeatureRepository::new(db);
    let doc_repo = DocumentRepository::new(db);

    let project = project_repo
        .get(project_id)
        .await?
        .ok_or_else(|| Error::NotFound(format!("Project not found: {}", project_id)))?;

    let features = feature_repo.list_by_project(project_id, None).await?;

    let generator = DocumentGenerator::new(config, cost_tracker)?;
    let generated = generator.generate_prd(&project, &features).await?;

    let document = Document::new(
        project_id,
        generated.doc_type,
        generated.title,
        generated.content,
    )
    .with_model(generated.model)
    .with_tokens(generated.tokens_used as i32)
    .with_cost(generated.cost_usd);

    doc_repo.create(&document).await?;

    info!(document_id = %document.id, "PRD created successfully");

    Ok(document)
}

/// Generate an architecture document for a project
pub async fn generate_architecture(
    db: &Database,
    project_id: &str,
    cost_tracker: Option<Arc<CostTracker>>,
) -> Result<Document> {
    let config = Config::load().map_err(|e| Error::ConfigError(e.to_string()))?;

    let project_repo = ProjectRepository::new(db);
    let feature_repo = FeatureRepository::new(db);
    let doc_repo = DocumentRepository::new(db);

    let project = project_repo
        .get(project_id)
        .await?
        .ok_or_else(|| Error::NotFound(format!("Project not found: {}", project_id)))?;

    let features = feature_repo.list_by_project(project_id, None).await?;

    let generator = DocumentGenerator::new(config, cost_tracker)?;
    let generated = generator.generate_architecture(&project, &features).await?;

    let document = Document::new(
        project_id,
        generated.doc_type,
        generated.title,
        generated.content,
    )
    .with_model(generated.model)
    .with_tokens(generated.tokens_used as i32)
    .with_cost(generated.cost_usd);

    doc_repo.create(&document).await?;

    info!(document_id = %document.id, "Architecture document created successfully");

    Ok(document)
}

/// List documents for a project
pub async fn list_documents(
    db: &Database,
    project_id: &str,
    doc_type: Option<DocumentType>,
) -> Result<Vec<Document>> {
    let doc_repo = DocumentRepository::new(db);
    doc_repo.list_by_project(project_id, doc_type).await
}

/// Get a document by ID
pub async fn get_document(db: &Database, document_id: &str) -> Result<Option<Document>> {
    let doc_repo = DocumentRepository::new(db);
    doc_repo.get(document_id).await
}

/// Update document status
pub async fn update_document_status(
    db: &Database,
    document_id: &str,
    status: DocumentStatus,
) -> Result<()> {
    let doc_repo = DocumentRepository::new(db);

    // Verify document exists
    if doc_repo.get(document_id).await?.is_none() {
        return Err(Error::NotFound(format!(
            "Document not found: {}",
            document_id
        )));
    }

    doc_repo.update_status(document_id, status).await
}

/// Delete a document
pub async fn delete_document(db: &Database, document_id: &str) -> Result<()> {
    let doc_repo = DocumentRepository::new(db);

    // Verify document exists
    if doc_repo.get(document_id).await?.is_none() {
        return Err(Error::NotFound(format!(
            "Document not found: {}",
            document_id
        )));
    }

    doc_repo.delete(document_id).await
}

/// Export a document to a file
pub async fn export_document(
    db: &Database,
    document_id: &str,
    output_path: &std::path::Path,
) -> Result<()> {
    let doc_repo = DocumentRepository::new(db);

    let document = doc_repo
        .get(document_id)
        .await?
        .ok_or_else(|| Error::NotFound(format!("Document not found: {}", document_id)))?;

    std::fs::write(output_path, &document.content).map_err(Error::Io)?;

    info!(
        document_id = %document_id,
        path = %output_path.display(),
        "Document exported successfully"
    );

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_document_type_parse() {
        assert_eq!(DocumentType::parse("prd"), Some(DocumentType::Prd));
        assert_eq!(
            DocumentType::parse("architecture"),
            Some(DocumentType::Architecture)
        );
        assert_eq!(DocumentType::parse("design"), Some(DocumentType::Design));
        assert_eq!(
            DocumentType::parse("tech_spec"),
            Some(DocumentType::TechSpec)
        );
        assert_eq!(DocumentType::parse("custom"), Some(DocumentType::Custom));
        assert_eq!(DocumentType::parse("invalid"), None);
    }

    #[test]
    fn test_document_status_parse() {
        assert_eq!(DocumentStatus::parse("draft"), Some(DocumentStatus::Draft));
        assert_eq!(
            DocumentStatus::parse("review"),
            Some(DocumentStatus::Review)
        );
        assert_eq!(DocumentStatus::parse("final"), Some(DocumentStatus::Final));
        assert_eq!(
            DocumentStatus::parse("archived"),
            Some(DocumentStatus::Archived)
        );
        assert_eq!(DocumentStatus::parse("invalid"), None);
    }

    #[test]
    fn test_document_new() {
        let doc = Document::new("proj-123", DocumentType::Prd, "Test PRD", "# Content");

        assert!(!doc.id.is_empty());
        assert_eq!(doc.project_id, "proj-123");
        assert_eq!(doc.doc_type, DocumentType::Prd);
        assert_eq!(doc.title, "Test PRD");
        assert_eq!(doc.content, "# Content");
        assert_eq!(doc.version, 1);
        assert_eq!(doc.status, DocumentStatus::Draft);
    }

    #[test]
    fn test_document_with_builders() {
        let doc = Document::new("proj-123", DocumentType::Architecture, "Test", "Content")
            .with_description("A test document")
            .with_model("claude-3-5-sonnet")
            .with_tokens(1000)
            .with_cost(0.05);

        assert_eq!(doc.description, Some("A test document".to_string()));
        assert_eq!(doc.model_used, Some("claude-3-5-sonnet".to_string()));
        assert_eq!(doc.tokens_used, Some(1000));
        assert_eq!(doc.generation_cost_usd, Some(0.05));
    }

    #[test]
    fn test_estimate_cost() {
        // Claude Sonnet: 1000 input + 500 output
        let cost = estimate_cost("anthropic/claude-sonnet-4-20250514", 1000, 500);
        // Expected: (1000/1M * 3) + (500/1M * 15) = 0.003 + 0.0075 = 0.0105
        assert!((cost - 0.0105).abs() < 0.0001);
    }

    #[tokio::test]
    async fn test_document_repository_crud() {
        let db = Database::in_memory()
            .await
            .expect("Failed to create database");

        // First create a project
        let project = Project::new("test-project", "rust", "");
        let project_repo = ProjectRepository::new(&db);
        project_repo.create(&project).await.unwrap();

        let doc_repo = DocumentRepository::new(&db);

        // Create
        let doc = Document::new(&project.id, DocumentType::Prd, "Test PRD", "# Content");
        doc_repo.create(&doc).await.unwrap();

        // Read
        let retrieved = doc_repo.get(&doc.id).await.unwrap().unwrap();
        assert_eq!(retrieved.title, "Test PRD");
        assert_eq!(retrieved.doc_type, DocumentType::Prd);

        // List
        let docs = doc_repo.list_by_project(&project.id, None).await.unwrap();
        assert_eq!(docs.len(), 1);

        // List by type
        let prd_docs = doc_repo
            .list_by_project(&project.id, Some(DocumentType::Prd))
            .await
            .unwrap();
        assert_eq!(prd_docs.len(), 1);

        let arch_docs = doc_repo
            .list_by_project(&project.id, Some(DocumentType::Architecture))
            .await
            .unwrap();
        assert_eq!(arch_docs.len(), 0);

        // Update status
        doc_repo
            .update_status(&doc.id, DocumentStatus::Review)
            .await
            .unwrap();
        let updated = doc_repo.get(&doc.id).await.unwrap().unwrap();
        assert_eq!(updated.status, DocumentStatus::Review);

        // Delete
        doc_repo.delete(&doc.id).await.unwrap();
        let deleted = doc_repo.get(&doc.id).await.unwrap();
        assert!(deleted.is_none());
    }

    #[tokio::test]
    async fn test_document_versions() {
        let db = Database::in_memory()
            .await
            .expect("Failed to create database");

        // Create project and document
        let project = Project::new("test-project", "rust", "");
        let project_repo = ProjectRepository::new(&db);
        project_repo.create(&project).await.unwrap();

        let doc_repo = DocumentRepository::new(&db);
        let doc = Document::new(&project.id, DocumentType::Prd, "Test PRD", "# V1");
        doc_repo.create(&doc).await.unwrap();

        // Save version
        let version = DocumentVersion {
            id: Uuid::new_v4().to_string(),
            document_id: doc.id.clone(),
            version_number: 1,
            content: "# V1".to_string(),
            change_summary: Some("Initial version".to_string()),
            model_used: Some("claude-3-5-sonnet".to_string()),
            created_at: Utc::now(),
        };
        doc_repo.save_version(&version).await.unwrap();

        // Get versions
        let versions = doc_repo.get_versions(&doc.id).await.unwrap();
        assert_eq!(versions.len(), 1);
        assert_eq!(versions[0].version_number, 1);
    }
}
