//! Document repository implementation
//!
//! Database operations for documents and document versions.

use chrono::Utc;
use sqlx::Row;

use crate::commands::document::{Document, DocumentStatus, DocumentType, DocumentVersion};
use crate::storage::Database;
use crate::Result;

/// Document repository for database operations
pub struct DocumentRepository<'a> {
    db: &'a Database,
}

impl<'a> DocumentRepository<'a> {
    /// Create a new document repository
    pub fn new(db: &'a Database) -> Self {
        Self { db }
    }

    /// Create a new document in the database
    pub async fn create(&self, document: &Document) -> Result<()> {
        sqlx::query(
            r#"
            INSERT INTO documents (
                id, project_id, doc_type, title, description, content, format,
                version, status, model_used, tokens_used, generation_cost_usd,
                created_at, updated_at
            )
            VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
            "#,
        )
        .bind(&document.id)
        .bind(&document.project_id)
        .bind(document.doc_type.as_str())
        .bind(&document.title)
        .bind(&document.description)
        .bind(&document.content)
        .bind(&document.format)
        .bind(document.version)
        .bind(document.status.as_str())
        .bind(&document.model_used)
        .bind(document.tokens_used)
        .bind(document.generation_cost_usd)
        .bind(document.created_at)
        .bind(document.updated_at)
        .execute(self.db.pool())
        .await?;

        Ok(())
    }

    /// Get a document by ID
    pub async fn get(&self, id: &str) -> Result<Option<Document>> {
        let row = sqlx::query(
            r#"
            SELECT id, project_id, doc_type, title, description, content, format,
                   version, status, model_used, tokens_used, generation_cost_usd,
                   created_at, updated_at
            FROM documents WHERE id = ?
            "#,
        )
        .bind(id)
        .fetch_optional(self.db.pool())
        .await?;

        Ok(row.map(|r| self.row_to_document(r)))
    }

    /// List documents for a project
    pub async fn list_by_project(
        &self,
        project_id: &str,
        doc_type: Option<DocumentType>,
    ) -> Result<Vec<Document>> {
        let rows = if let Some(dt) = doc_type {
            sqlx::query(
                r#"
                SELECT id, project_id, doc_type, title, description, content, format,
                       version, status, model_used, tokens_used, generation_cost_usd,
                       created_at, updated_at
                FROM documents WHERE project_id = ? AND doc_type = ?
                ORDER BY created_at DESC
                "#,
            )
            .bind(project_id)
            .bind(dt.as_str())
            .fetch_all(self.db.pool())
            .await?
        } else {
            sqlx::query(
                r#"
                SELECT id, project_id, doc_type, title, description, content, format,
                       version, status, model_used, tokens_used, generation_cost_usd,
                       created_at, updated_at
                FROM documents WHERE project_id = ?
                ORDER BY created_at DESC
                "#,
            )
            .bind(project_id)
            .fetch_all(self.db.pool())
            .await?
        };

        Ok(rows.into_iter().map(|r| self.row_to_document(r)).collect())
    }

    /// Update a document
    pub async fn update(&self, document: &Document) -> Result<()> {
        sqlx::query(
            r#"
            UPDATE documents
            SET title = ?, description = ?, content = ?, version = ?,
                status = ?, updated_at = ?
            WHERE id = ?
            "#,
        )
        .bind(&document.title)
        .bind(&document.description)
        .bind(&document.content)
        .bind(document.version)
        .bind(document.status.as_str())
        .bind(Utc::now())
        .bind(&document.id)
        .execute(self.db.pool())
        .await?;

        Ok(())
    }

    /// Update document status
    pub async fn update_status(&self, id: &str, status: DocumentStatus) -> Result<()> {
        sqlx::query("UPDATE documents SET status = ?, updated_at = ? WHERE id = ?")
            .bind(status.as_str())
            .bind(Utc::now())
            .bind(id)
            .execute(self.db.pool())
            .await?;

        Ok(())
    }

    /// Delete a document
    pub async fn delete(&self, id: &str) -> Result<()> {
        sqlx::query("DELETE FROM documents WHERE id = ?")
            .bind(id)
            .execute(self.db.pool())
            .await?;

        Ok(())
    }

    /// Save a document version
    pub async fn save_version(&self, version: &DocumentVersion) -> Result<()> {
        sqlx::query(
            r#"
            INSERT INTO document_versions (
                id, document_id, version_number, content, change_summary, model_used, created_at
            )
            VALUES (?, ?, ?, ?, ?, ?, ?)
            "#,
        )
        .bind(&version.id)
        .bind(&version.document_id)
        .bind(version.version_number)
        .bind(&version.content)
        .bind(&version.change_summary)
        .bind(&version.model_used)
        .bind(version.created_at)
        .execute(self.db.pool())
        .await?;

        Ok(())
    }

    /// Get all versions of a document
    pub async fn get_versions(&self, document_id: &str) -> Result<Vec<DocumentVersion>> {
        let rows = sqlx::query(
            r#"
            SELECT id, document_id, version_number, content, change_summary, model_used, created_at
            FROM document_versions WHERE document_id = ?
            ORDER BY version_number DESC
            "#,
        )
        .bind(document_id)
        .fetch_all(self.db.pool())
        .await?;

        Ok(rows
            .into_iter()
            .map(|r| DocumentVersion {
                id: r.get("id"),
                document_id: r.get("document_id"),
                version_number: r.get("version_number"),
                content: r.get("content"),
                change_summary: r.get("change_summary"),
                model_used: r.get("model_used"),
                created_at: r.get("created_at"),
            })
            .collect())
    }

    /// Convert a database row to a Document
    fn row_to_document(&self, row: sqlx::sqlite::SqliteRow) -> Document {
        Document {
            id: row.get("id"),
            project_id: row.get("project_id"),
            doc_type: DocumentType::parse(row.get("doc_type")).unwrap_or(DocumentType::Custom),
            title: row.get("title"),
            description: row.get("description"),
            content: row.get("content"),
            format: row.get("format"),
            version: row.get("version"),
            status: DocumentStatus::parse(row.get("status")).unwrap_or_default(),
            model_used: row.get("model_used"),
            tokens_used: row.get("tokens_used"),
            generation_cost_usd: row.get("generation_cost_usd"),
            created_at: row.get("created_at"),
            updated_at: row.get("updated_at"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::storage::Database;
    use uuid::Uuid;

    async fn create_test_db() -> Database {
        Database::in_memory()
            .await
            .expect("Failed to create test database")
    }

    async fn create_test_project(db: &Database) -> String {
        let project_id = Uuid::new_v4().to_string();
        sqlx::query("INSERT INTO projects (id, name, framework) VALUES (?, ?, ?)")
            .bind(&project_id)
            .bind("Test Project")
            .bind("rust")
            .execute(db.pool())
            .await
            .expect("Failed to insert test project");
        project_id
    }

    #[tokio::test]
    async fn test_document_create_and_get() {
        let db = create_test_db().await;
        let project_id = create_test_project(&db).await;
        let repo = DocumentRepository::new(&db);

        let doc = Document::new(&project_id, DocumentType::Prd, "Test PRD", "# Content");

        repo.create(&doc).await.expect("Failed to create");

        let retrieved = repo
            .get(&doc.id)
            .await
            .expect("Failed to get")
            .expect("Document not found");

        assert_eq!(retrieved.id, doc.id);
        assert_eq!(retrieved.project_id, project_id);
        assert_eq!(retrieved.title, "Test PRD");
        assert_eq!(retrieved.doc_type, DocumentType::Prd);
        assert_eq!(retrieved.status, DocumentStatus::Draft);
    }

    #[tokio::test]
    async fn test_document_list_by_project() {
        let db = create_test_db().await;
        let project_id = create_test_project(&db).await;
        let repo = DocumentRepository::new(&db);

        // Create multiple documents
        let prd = Document::new(&project_id, DocumentType::Prd, "PRD", "# PRD");
        let arch = Document::new(&project_id, DocumentType::Architecture, "Arch", "# Arch");
        let design = Document::new(&project_id, DocumentType::Design, "Design", "# Design");

        repo.create(&prd).await.unwrap();
        repo.create(&arch).await.unwrap();
        repo.create(&design).await.unwrap();

        // List all documents
        let all_docs = repo.list_by_project(&project_id, None).await.unwrap();
        assert_eq!(all_docs.len(), 3);

        // List only PRDs
        let prd_docs = repo
            .list_by_project(&project_id, Some(DocumentType::Prd))
            .await
            .unwrap();
        assert_eq!(prd_docs.len(), 1);
        assert_eq!(prd_docs[0].doc_type, DocumentType::Prd);

        // List only Architecture docs
        let arch_docs = repo
            .list_by_project(&project_id, Some(DocumentType::Architecture))
            .await
            .unwrap();
        assert_eq!(arch_docs.len(), 1);
    }

    #[tokio::test]
    async fn test_document_update() {
        let db = create_test_db().await;
        let project_id = create_test_project(&db).await;
        let repo = DocumentRepository::new(&db);

        let mut doc = Document::new(&project_id, DocumentType::Prd, "Original", "# V1");
        repo.create(&doc).await.unwrap();

        // Update document
        doc.title = "Updated Title".to_string();
        doc.content = "# V2".to_string();
        doc.version = 2;
        repo.update(&doc).await.expect("Failed to update");

        let retrieved = repo.get(&doc.id).await.unwrap().unwrap();
        assert_eq!(retrieved.title, "Updated Title");
        assert_eq!(retrieved.content, "# V2");
        assert_eq!(retrieved.version, 2);
    }

    #[tokio::test]
    async fn test_document_update_status() {
        let db = create_test_db().await;
        let project_id = create_test_project(&db).await;
        let repo = DocumentRepository::new(&db);

        let doc = Document::new(&project_id, DocumentType::Prd, "Test", "# Content");
        repo.create(&doc).await.unwrap();

        assert_eq!(
            repo.get(&doc.id).await.unwrap().unwrap().status,
            DocumentStatus::Draft
        );

        repo.update_status(&doc.id, DocumentStatus::Review)
            .await
            .expect("Failed to update status");

        assert_eq!(
            repo.get(&doc.id).await.unwrap().unwrap().status,
            DocumentStatus::Review
        );

        repo.update_status(&doc.id, DocumentStatus::Final)
            .await
            .unwrap();

        assert_eq!(
            repo.get(&doc.id).await.unwrap().unwrap().status,
            DocumentStatus::Final
        );
    }

    #[tokio::test]
    async fn test_document_delete() {
        let db = create_test_db().await;
        let project_id = create_test_project(&db).await;
        let repo = DocumentRepository::new(&db);

        let doc = Document::new(&project_id, DocumentType::Prd, "To Delete", "# Content");
        repo.create(&doc).await.unwrap();

        assert!(repo.get(&doc.id).await.unwrap().is_some());

        repo.delete(&doc.id).await.expect("Failed to delete");

        assert!(repo.get(&doc.id).await.unwrap().is_none());
    }

    #[tokio::test]
    async fn test_document_with_metadata() {
        let db = create_test_db().await;
        let project_id = create_test_project(&db).await;
        let repo = DocumentRepository::new(&db);

        let doc = Document::new(&project_id, DocumentType::Prd, "Test", "# Content")
            .with_description("A test document")
            .with_model("claude-3-5-sonnet")
            .with_tokens(1500)
            .with_cost(0.05);

        repo.create(&doc).await.unwrap();

        let retrieved = repo.get(&doc.id).await.unwrap().unwrap();
        assert_eq!(retrieved.description, Some("A test document".to_string()));
        assert_eq!(retrieved.model_used, Some("claude-3-5-sonnet".to_string()));
        assert_eq!(retrieved.tokens_used, Some(1500));
        assert_eq!(retrieved.generation_cost_usd, Some(0.05));
    }

    #[tokio::test]
    async fn test_document_version_save_and_get() {
        let db = create_test_db().await;
        let project_id = create_test_project(&db).await;
        let repo = DocumentRepository::new(&db);

        let doc = Document::new(&project_id, DocumentType::Prd, "Test", "# V1");
        repo.create(&doc).await.unwrap();

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
        repo.save_version(&version)
            .await
            .expect("Failed to save version");

        let versions = repo
            .get_versions(&doc.id)
            .await
            .expect("Failed to get versions");
        assert_eq!(versions.len(), 1);
        assert_eq!(versions[0].version_number, 1);
        assert_eq!(
            versions[0].change_summary,
            Some("Initial version".to_string())
        );
    }

    #[tokio::test]
    async fn test_document_multiple_versions() {
        let db = create_test_db().await;
        let project_id = create_test_project(&db).await;
        let repo = DocumentRepository::new(&db);

        let doc = Document::new(
            &project_id,
            DocumentType::Architecture,
            "Architecture",
            "# V1",
        );
        repo.create(&doc).await.unwrap();

        // Save multiple versions
        for i in 1..=3 {
            let version = DocumentVersion {
                id: Uuid::new_v4().to_string(),
                document_id: doc.id.clone(),
                version_number: i,
                content: format!("# V{}", i),
                change_summary: Some(format!("Version {}", i)),
                model_used: None,
                created_at: Utc::now(),
            };
            repo.save_version(&version).await.unwrap();
        }

        let versions = repo.get_versions(&doc.id).await.unwrap();
        assert_eq!(versions.len(), 3);
        // Should be ordered by version_number DESC
        assert_eq!(versions[0].version_number, 3);
        assert_eq!(versions[1].version_number, 2);
        assert_eq!(versions[2].version_number, 1);
    }

    #[tokio::test]
    async fn test_document_type_filtering() {
        let db = create_test_db().await;
        let project_id = create_test_project(&db).await;
        let repo = DocumentRepository::new(&db);

        // Create one of each type
        let types = [
            DocumentType::Prd,
            DocumentType::Architecture,
            DocumentType::Design,
            DocumentType::TechSpec,
            DocumentType::Custom,
        ];

        for doc_type in &types {
            let doc = Document::new(
                &project_id,
                *doc_type,
                format!("{:?}", doc_type),
                "# Content",
            );
            repo.create(&doc).await.unwrap();
        }

        // Test each filter
        for doc_type in &types {
            let docs = repo
                .list_by_project(&project_id, Some(*doc_type))
                .await
                .unwrap();
            assert_eq!(docs.len(), 1, "Expected 1 doc for type {:?}", doc_type);
            assert_eq!(docs[0].doc_type, *doc_type);
        }
    }
}
