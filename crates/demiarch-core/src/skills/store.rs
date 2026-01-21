//! Skill storage and retrieval
//!
//! This module provides persistent storage for learned skills using SQLite.
//! It supports CRUD operations, full-text search, and usage tracking.

use chrono::{DateTime, Utc};
use sqlx::{FromRow, SqlitePool};
use tracing::{debug, info};

use crate::error::{Error, Result};

use super::types::{
    LearnedSkill, PatternType, PatternVariable, SkillCategory, SkillConfidence, SkillMetadata,
    SkillPattern, SkillSource, SkillUsageStats,
};

/// Store for persisting and retrieving learned skills
#[derive(Clone)]
pub struct SkillStore {
    pool: SqlitePool,
}

impl SkillStore {
    /// Create a new skill store with the given database pool
    pub fn new(pool: SqlitePool) -> Self {
        Self { pool }
    }

    /// Save a learned skill to the database
    pub async fn save(&self, skill: &LearnedSkill) -> Result<()> {
        let tags_json = serde_json::to_string(&skill.tags)
            .map_err(|e| Error::Other(format!("Failed to serialize tags: {}", e)))?;

        let variables_json = serde_json::to_string(&skill.pattern.variables)
            .map_err(|e| Error::Other(format!("Failed to serialize variables: {}", e)))?;

        let applicability_json = serde_json::to_string(&skill.pattern.applicability)
            .map_err(|e| Error::Other(format!("Failed to serialize applicability: {}", e)))?;

        let limitations_json = serde_json::to_string(&skill.pattern.limitations)
            .map_err(|e| Error::Other(format!("Failed to serialize limitations: {}", e)))?;

        let metadata_json = serde_json::to_string(&skill.metadata)
            .map_err(|e| Error::Other(format!("Failed to serialize metadata: {}", e)))?;

        sqlx::query(
            r#"
            INSERT INTO learned_skills (
                id, name, description, category,
                pattern_type, pattern_template, pattern_variables,
                pattern_applicability, pattern_limitations,
                source_project_id, source_feature_id, source_agent_type,
                source_original_task, source_model_used, source_tokens_used,
                confidence, tags,
                times_used, success_count, failure_count, last_used_at,
                metadata, created_at, updated_at
            ) VALUES (
                ?, ?, ?, ?,
                ?, ?, ?,
                ?, ?,
                ?, ?, ?,
                ?, ?, ?,
                ?, ?,
                ?, ?, ?, ?,
                ?, ?, ?
            )
            ON CONFLICT(id) DO UPDATE SET
                name = excluded.name,
                description = excluded.description,
                category = excluded.category,
                pattern_type = excluded.pattern_type,
                pattern_template = excluded.pattern_template,
                pattern_variables = excluded.pattern_variables,
                pattern_applicability = excluded.pattern_applicability,
                pattern_limitations = excluded.pattern_limitations,
                confidence = excluded.confidence,
                tags = excluded.tags,
                times_used = excluded.times_used,
                success_count = excluded.success_count,
                failure_count = excluded.failure_count,
                last_used_at = excluded.last_used_at,
                metadata = excluded.metadata,
                updated_at = excluded.updated_at
            "#,
        )
        .bind(&skill.id)
        .bind(&skill.name)
        .bind(&skill.description)
        .bind(skill.category.as_str())
        .bind(skill.pattern.pattern_type.as_str())
        .bind(&skill.pattern.template)
        .bind(&variables_json)
        .bind(&applicability_json)
        .bind(&limitations_json)
        .bind(&skill.source.project_id)
        .bind(&skill.source.feature_id)
        .bind(&skill.source.agent_type)
        .bind(&skill.source.original_task)
        .bind(&skill.source.model_used)
        .bind(skill.source.tokens_used.map(|t| t as i32))
        .bind(skill.confidence.as_str())
        .bind(&tags_json)
        .bind(skill.usage_stats.times_used as i32)
        .bind(skill.usage_stats.success_count as i32)
        .bind(skill.usage_stats.failure_count as i32)
        .bind(skill.usage_stats.last_used_at.map(|dt| dt.to_rfc3339()))
        .bind(&metadata_json)
        .bind(skill.created_at.to_rfc3339())
        .bind(skill.updated_at.to_rfc3339())
        .execute(&self.pool)
        .await?;

        info!(skill_id = %skill.id, skill_name = %skill.name, "Skill saved");
        Ok(())
    }

    /// Get a skill by ID
    pub async fn get(&self, id: &str) -> Result<Option<LearnedSkill>> {
        let row: Option<SkillRow> = sqlx::query_as(
            r#"
            SELECT * FROM learned_skills WHERE id = ?
            "#,
        )
        .bind(id)
        .fetch_optional(&self.pool)
        .await?;

        row.map(|r| r.into_learned_skill()).transpose()
    }

    /// Get all skills
    pub async fn list(&self) -> Result<Vec<LearnedSkill>> {
        let rows: Vec<SkillRow> = sqlx::query_as(
            r#"
            SELECT * FROM learned_skills
            ORDER BY times_used DESC, confidence DESC, created_at DESC
            "#,
        )
        .fetch_all(&self.pool)
        .await?;

        rows.into_iter()
            .map(|r| r.into_learned_skill())
            .collect()
    }

    /// Get skills by category
    pub async fn list_by_category(&self, category: SkillCategory) -> Result<Vec<LearnedSkill>> {
        let rows: Vec<SkillRow> = sqlx::query_as(
            r#"
            SELECT * FROM learned_skills
            WHERE category = ?
            ORDER BY times_used DESC, confidence DESC, created_at DESC
            "#,
        )
        .bind(category.as_str())
        .fetch_all(&self.pool)
        .await?;

        rows.into_iter()
            .map(|r| r.into_learned_skill())
            .collect()
    }

    /// Search skills using full-text search
    pub async fn search(&self, query: &str) -> Result<Vec<LearnedSkill>> {
        // Use FTS5 for full-text search
        let rows: Vec<SkillRow> = sqlx::query_as(
            r#"
            SELECT ls.* FROM learned_skills ls
            JOIN learned_skills_fts fts ON ls.rowid = fts.rowid
            WHERE learned_skills_fts MATCH ?
            ORDER BY rank, ls.times_used DESC
            LIMIT 20
            "#,
        )
        .bind(format!("{}*", query)) // Add wildcard for prefix matching
        .fetch_all(&self.pool)
        .await?;

        rows.into_iter()
            .map(|r| r.into_learned_skill())
            .collect()
    }

    /// Search skills by tags
    pub async fn search_by_tags(&self, tags: &[String]) -> Result<Vec<LearnedSkill>> {
        // Build a query that matches any of the tags
        let tag_patterns: Vec<String> = tags
            .iter()
            .map(|t| format!("%\"{}%", t.to_lowercase()))
            .collect();

        let mut skills = Vec::new();

        for pattern in tag_patterns {
            let rows: Vec<SkillRow> = sqlx::query_as(
                r#"
                SELECT * FROM learned_skills
                WHERE LOWER(tags) LIKE ?
                ORDER BY times_used DESC, confidence DESC
                "#,
            )
            .bind(&pattern)
            .fetch_all(&self.pool)
            .await?;

            for row in rows {
                let skill = row.into_learned_skill()?;
                if !skills.iter().any(|s: &LearnedSkill| s.id == skill.id) {
                    skills.push(skill);
                }
            }
        }

        Ok(skills)
    }

    /// Get top skills by usage
    pub async fn top_by_usage(&self, limit: u32) -> Result<Vec<LearnedSkill>> {
        let rows: Vec<SkillRow> = sqlx::query_as(
            r#"
            SELECT * FROM learned_skills
            WHERE times_used > 0
            ORDER BY times_used DESC, success_count DESC
            LIMIT ?
            "#,
        )
        .bind(limit as i32)
        .fetch_all(&self.pool)
        .await?;

        rows.into_iter()
            .map(|r| r.into_learned_skill())
            .collect()
    }

    /// Get skills by project
    pub async fn list_by_project(&self, project_id: &str) -> Result<Vec<LearnedSkill>> {
        let rows: Vec<SkillRow> = sqlx::query_as(
            r#"
            SELECT * FROM learned_skills
            WHERE source_project_id = ?
            ORDER BY created_at DESC
            "#,
        )
        .bind(project_id)
        .fetch_all(&self.pool)
        .await?;

        rows.into_iter()
            .map(|r| r.into_learned_skill())
            .collect()
    }

    /// Update usage statistics for a skill
    pub async fn record_usage(&self, skill_id: &str, success: bool) -> Result<()> {
        let success_increment = if success { 1 } else { 0 };
        let failure_increment = if success { 0 } else { 1 };

        sqlx::query(
            r#"
            UPDATE learned_skills
            SET times_used = times_used + 1,
                success_count = success_count + ?,
                failure_count = failure_count + ?,
                last_used_at = ?,
                updated_at = ?
            WHERE id = ?
            "#,
        )
        .bind(success_increment)
        .bind(failure_increment)
        .bind(Utc::now().to_rfc3339())
        .bind(Utc::now().to_rfc3339())
        .bind(skill_id)
        .execute(&self.pool)
        .await?;

        debug!(skill_id = %skill_id, success = success, "Recorded skill usage");
        Ok(())
    }

    /// Delete a skill by ID
    pub async fn delete(&self, id: &str) -> Result<bool> {
        let result = sqlx::query(
            r#"
            DELETE FROM learned_skills WHERE id = ?
            "#,
        )
        .bind(id)
        .execute(&self.pool)
        .await?;

        let deleted = result.rows_affected() > 0;
        if deleted {
            info!(skill_id = %id, "Skill deleted");
        }
        Ok(deleted)
    }

    /// Get skill count
    pub async fn count(&self) -> Result<u64> {
        let (count,): (i64,) = sqlx::query_as(
            r#"
            SELECT COUNT(*) FROM learned_skills
            "#,
        )
        .fetch_one(&self.pool)
        .await?;

        Ok(count as u64)
    }

    /// Get skill statistics
    pub async fn stats(&self) -> Result<SkillStats> {
        let (total,): (i64,) = sqlx::query_as("SELECT COUNT(*) FROM learned_skills")
            .fetch_one(&self.pool)
            .await?;

        let (total_uses,): (i64,) =
            sqlx::query_as("SELECT COALESCE(SUM(times_used), 0) FROM learned_skills")
                .fetch_one(&self.pool)
                .await?;

        let (high_confidence,): (i64,) =
            sqlx::query_as("SELECT COUNT(*) FROM learned_skills WHERE confidence = 'high'")
                .fetch_one(&self.pool)
                .await?;

        let categories: Vec<(String, i64)> = sqlx::query_as(
            r#"
            SELECT category, COUNT(*) as count
            FROM learned_skills
            GROUP BY category
            ORDER BY count DESC
            "#,
        )
        .fetch_all(&self.pool)
        .await?;

        Ok(SkillStats {
            total_skills: total as u64,
            total_uses: total_uses as u64,
            high_confidence_skills: high_confidence as u64,
            skills_by_category: categories
                .into_iter()
                .map(|(cat, count)| (SkillCategory::from_str(&cat), count as u64))
                .collect(),
        })
    }
}

/// Statistics about stored skills
#[derive(Debug, Clone)]
pub struct SkillStats {
    /// Total number of skills
    pub total_skills: u64,
    /// Total times skills have been used
    pub total_uses: u64,
    /// Number of high-confidence skills
    pub high_confidence_skills: u64,
    /// Skills grouped by category
    pub skills_by_category: Vec<(SkillCategory, u64)>,
}

/// Database row for learned_skills table
#[derive(Debug, FromRow)]
struct SkillRow {
    id: String,
    name: String,
    description: String,
    category: String,
    pattern_type: String,
    pattern_template: String,
    pattern_variables: Option<String>,
    pattern_applicability: Option<String>,
    pattern_limitations: Option<String>,
    source_project_id: Option<String>,
    source_feature_id: Option<String>,
    source_agent_type: Option<String>,
    source_original_task: Option<String>,
    source_model_used: Option<String>,
    source_tokens_used: Option<i32>,
    confidence: String,
    tags: Option<String>,
    times_used: i32,
    success_count: i32,
    failure_count: i32,
    last_used_at: Option<String>,
    metadata: Option<String>,
    created_at: String,
    updated_at: String,
}

impl SkillRow {
    /// Convert a database row to a LearnedSkill
    fn into_learned_skill(self) -> Result<LearnedSkill> {
        let category = SkillCategory::from_str(&self.category);

        let pattern_type = match self.pattern_type.as_str() {
            "technique" => PatternType::Technique,
            "architecture_pattern" => PatternType::ArchitecturePattern,
            "command_template" => PatternType::CommandTemplate,
            "config_pattern" => PatternType::ConfigPattern,
            "workflow_pattern" => PatternType::WorkflowPattern,
            _ => PatternType::CodeTemplate,
        };

        let variables: Vec<PatternVariable> = self
            .pattern_variables
            .as_ref()
            .map(|s| serde_json::from_str(s).unwrap_or_default())
            .unwrap_or_default();

        let applicability: Vec<String> = self
            .pattern_applicability
            .as_ref()
            .map(|s| serde_json::from_str(s).unwrap_or_default())
            .unwrap_or_default();

        let limitations: Vec<String> = self
            .pattern_limitations
            .as_ref()
            .map(|s| serde_json::from_str(s).unwrap_or_default())
            .unwrap_or_default();

        let pattern = SkillPattern {
            pattern_type,
            template: self.pattern_template,
            variables,
            applicability,
            limitations,
        };

        let source = SkillSource {
            project_id: self.source_project_id,
            feature_id: self.source_feature_id,
            agent_type: self.source_agent_type,
            original_task: self.source_original_task,
            model_used: self.source_model_used,
            tokens_used: self.source_tokens_used.map(|t| t as u32),
        };

        let confidence = SkillConfidence::from_str(&self.confidence);

        let tags: Vec<String> = self
            .tags
            .as_ref()
            .map(|s| serde_json::from_str(s).unwrap_or_default())
            .unwrap_or_default();

        let last_used_at: Option<DateTime<Utc>> = self
            .last_used_at
            .as_ref()
            .and_then(|s| DateTime::parse_from_rfc3339(s).ok())
            .map(|dt| dt.with_timezone(&Utc));

        let usage_stats = SkillUsageStats {
            times_used: self.times_used as u32,
            success_count: self.success_count as u32,
            failure_count: self.failure_count as u32,
            last_used_at,
        };

        let metadata: SkillMetadata = self
            .metadata
            .as_ref()
            .map(|s| serde_json::from_str(s).unwrap_or_default())
            .unwrap_or_default();

        let created_at = DateTime::parse_from_rfc3339(&self.created_at)
            .map(|dt| dt.with_timezone(&Utc))
            .unwrap_or_else(|_| Utc::now());

        let updated_at = DateTime::parse_from_rfc3339(&self.updated_at)
            .map(|dt| dt.with_timezone(&Utc))
            .unwrap_or_else(|_| Utc::now());

        Ok(LearnedSkill {
            id: self.id,
            name: self.name,
            description: self.description,
            category,
            pattern,
            source,
            confidence,
            tags,
            usage_stats,
            metadata,
            created_at,
            updated_at,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::storage::Database;

    async fn setup_test_db() -> SkillStore {
        let db = Database::in_memory()
            .await
            .expect("Failed to create test database");
        SkillStore::new(db.pool().clone())
    }

    fn create_test_skill(name: &str) -> LearnedSkill {
        LearnedSkill::new(
            name,
            format!("Description for {}", name),
            SkillCategory::CodeGeneration,
            SkillPattern::code("// test template"),
        )
        .with_tags(vec!["test".into(), "rust".into()])
    }

    #[tokio::test]
    async fn test_save_and_get() {
        let store = setup_test_db().await;
        let skill = create_test_skill("Test Skill");
        let id = skill.id.clone();

        store.save(&skill).await.unwrap();

        let retrieved = store.get(&id).await.unwrap();
        assert!(retrieved.is_some());

        let retrieved = retrieved.unwrap();
        assert_eq!(retrieved.name, "Test Skill");
        assert_eq!(retrieved.category, SkillCategory::CodeGeneration);
    }

    #[tokio::test]
    async fn test_list() {
        let store = setup_test_db().await;

        store.save(&create_test_skill("Skill 1")).await.unwrap();
        store.save(&create_test_skill("Skill 2")).await.unwrap();
        store.save(&create_test_skill("Skill 3")).await.unwrap();

        let skills = store.list().await.unwrap();
        assert_eq!(skills.len(), 3);
    }

    #[tokio::test]
    async fn test_list_by_category() {
        let store = setup_test_db().await;

        let mut skill1 = create_test_skill("Code Skill");
        skill1.category = SkillCategory::CodeGeneration;
        store.save(&skill1).await.unwrap();

        let mut skill2 = create_test_skill("Test Skill");
        skill2.category = SkillCategory::Testing;
        store.save(&skill2).await.unwrap();

        let code_skills = store
            .list_by_category(SkillCategory::CodeGeneration)
            .await
            .unwrap();
        assert_eq!(code_skills.len(), 1);
        assert_eq!(code_skills[0].name, "Code Skill");
    }

    #[tokio::test]
    async fn test_search() {
        let store = setup_test_db().await;

        let skill = LearnedSkill::new(
            "Error Handling Pattern",
            "A pattern for handling errors gracefully",
            SkillCategory::ErrorHandling,
            SkillPattern::code("// error handling"),
        );
        store.save(&skill).await.unwrap();

        let results = store.search("error").await.unwrap();
        assert!(!results.is_empty());
        assert!(results.iter().any(|s| s.name.contains("Error")));
    }

    #[tokio::test]
    async fn test_record_usage() {
        let store = setup_test_db().await;
        let skill = create_test_skill("Usage Test");
        let id = skill.id.clone();
        store.save(&skill).await.unwrap();

        store.record_usage(&id, true).await.unwrap();
        store.record_usage(&id, true).await.unwrap();
        store.record_usage(&id, false).await.unwrap();

        let updated = store.get(&id).await.unwrap().unwrap();
        assert_eq!(updated.usage_stats.times_used, 3);
        assert_eq!(updated.usage_stats.success_count, 2);
        assert_eq!(updated.usage_stats.failure_count, 1);
    }

    #[tokio::test]
    async fn test_delete() {
        let store = setup_test_db().await;
        let skill = create_test_skill("To Delete");
        let id = skill.id.clone();
        store.save(&skill).await.unwrap();

        assert!(store.get(&id).await.unwrap().is_some());

        let deleted = store.delete(&id).await.unwrap();
        assert!(deleted);

        assert!(store.get(&id).await.unwrap().is_none());
    }

    #[tokio::test]
    async fn test_count() {
        let store = setup_test_db().await;

        assert_eq!(store.count().await.unwrap(), 0);

        store.save(&create_test_skill("Skill 1")).await.unwrap();
        store.save(&create_test_skill("Skill 2")).await.unwrap();

        assert_eq!(store.count().await.unwrap(), 2);
    }

    #[tokio::test]
    async fn test_stats() {
        let store = setup_test_db().await;

        let mut skill = create_test_skill("High Confidence");
        skill.confidence = SkillConfidence::High;
        store.save(&skill).await.unwrap();

        store.record_usage(&skill.id, true).await.unwrap();

        let stats = store.stats().await.unwrap();
        assert_eq!(stats.total_skills, 1);
        assert_eq!(stats.total_uses, 1);
        assert_eq!(stats.high_confidence_skills, 1);
    }

    #[tokio::test]
    async fn test_upsert() {
        let store = setup_test_db().await;
        let mut skill = create_test_skill("Upsert Test");
        let id = skill.id.clone();

        store.save(&skill).await.unwrap();

        skill.description = "Updated description".into();
        store.save(&skill).await.unwrap();

        let retrieved = store.get(&id).await.unwrap().unwrap();
        assert_eq!(retrieved.description, "Updated description");

        assert_eq!(store.count().await.unwrap(), 1);
    }
}
