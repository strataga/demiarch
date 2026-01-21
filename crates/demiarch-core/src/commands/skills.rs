//! Skills commands for managing learned skills
//!
//! Provides CLI commands for listing, searching, viewing, and managing
//! learned skills extracted from agent interactions.

use crate::error::Result;
use crate::skills::{LearnedSkill, SkillCategory, SkillStats, SkillStore};
use crate::storage::Database;

/// List all skills with optional filtering
pub async fn list_skills(
    category: Option<SkillCategory>,
    limit: Option<u32>,
) -> Result<Vec<LearnedSkill>> {
    let db = Database::default()
        .await
        .map_err(|e| crate::error::Error::Other(e.to_string()))?;
    let store = SkillStore::new(db.pool().clone());

    let skills = if let Some(cat) = category {
        store.list_by_category(cat).await?
    } else {
        store.list().await?
    };

    // Apply limit if specified
    let skills = if let Some(limit) = limit {
        skills.into_iter().take(limit as usize).collect()
    } else {
        skills
    };

    Ok(skills)
}

/// Get a specific skill by ID
pub async fn get_skill(id: &str) -> Result<Option<LearnedSkill>> {
    let db = Database::default()
        .await
        .map_err(|e| crate::error::Error::Other(e.to_string()))?;
    let store = SkillStore::new(db.pool().clone());

    store.get(id).await
}

/// Search skills by query string
pub async fn search_skills(query: &str) -> Result<Vec<LearnedSkill>> {
    let db = Database::default()
        .await
        .map_err(|e| crate::error::Error::Other(e.to_string()))?;
    let store = SkillStore::new(db.pool().clone());

    store.search(query).await
}

/// Search skills by tags
pub async fn search_skills_by_tags(tags: &[String]) -> Result<Vec<LearnedSkill>> {
    let db = Database::default()
        .await
        .map_err(|e| crate::error::Error::Other(e.to_string()))?;
    let store = SkillStore::new(db.pool().clone());

    store.search_by_tags(tags).await
}

/// Get top skills by usage
pub async fn top_skills(limit: u32) -> Result<Vec<LearnedSkill>> {
    let db = Database::default()
        .await
        .map_err(|e| crate::error::Error::Other(e.to_string()))?;
    let store = SkillStore::new(db.pool().clone());

    store.top_by_usage(limit).await
}

/// Get skills for a specific project
pub async fn skills_by_project(project_id: &str) -> Result<Vec<LearnedSkill>> {
    let db = Database::default()
        .await
        .map_err(|e| crate::error::Error::Other(e.to_string()))?;
    let store = SkillStore::new(db.pool().clone());

    store.list_by_project(project_id).await
}

/// Save a skill
pub async fn save_skill(skill: &LearnedSkill) -> Result<()> {
    let db = Database::default()
        .await
        .map_err(|e| crate::error::Error::Other(e.to_string()))?;
    let store = SkillStore::new(db.pool().clone());

    store.save(skill).await
}

/// Record that a skill was used
pub async fn record_skill_usage(skill_id: &str, success: bool) -> Result<()> {
    let db = Database::default()
        .await
        .map_err(|e| crate::error::Error::Other(e.to_string()))?;
    let store = SkillStore::new(db.pool().clone());

    store.record_usage(skill_id, success).await
}

/// Delete a skill
pub async fn delete_skill(id: &str) -> Result<bool> {
    let db = Database::default()
        .await
        .map_err(|e| crate::error::Error::Other(e.to_string()))?;
    let store = SkillStore::new(db.pool().clone());

    store.delete(id).await
}

/// Get skill statistics
pub async fn skill_stats() -> Result<SkillStats> {
    let db = Database::default()
        .await
        .map_err(|e| crate::error::Error::Other(e.to_string()))?;
    let store = SkillStore::new(db.pool().clone());

    store.stats().await
}

/// Get total skill count
pub async fn skill_count() -> Result<u64> {
    let db = Database::default()
        .await
        .map_err(|e| crate::error::Error::Other(e.to_string()))?;
    let store = SkillStore::new(db.pool().clone());

    store.count().await
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_skill_count_empty() {
        // This test would require a test database setup
        // For now, just verify the function signature is correct
        let _ = skill_count().await;
    }
}
