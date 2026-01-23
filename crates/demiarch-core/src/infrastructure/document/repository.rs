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
