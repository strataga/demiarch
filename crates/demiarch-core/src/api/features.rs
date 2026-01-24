//! Features API
//!
//! Provides high-level operations for feature management from GUI.

use crate::commands::feature::{Feature, FeatureRepository, FeatureStatus};
use crate::Result;
use serde::{Deserialize, Serialize};

use super::get_database;

/// Feature summary for GUI display
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FeatureSummary {
    pub id: String,
    pub project_id: String,
    pub title: String,
    pub description: Option<String>,
    pub status: String,
    pub priority: i32,
    pub labels: Vec<String>,
    pub phase_id: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

impl From<Feature> for FeatureSummary {
    fn from(f: Feature) -> Self {
        Self {
            id: f.id,
            project_id: f.project_id,
            title: f.title,
            description: f.description,
            status: f.status.as_str().to_string(),
            priority: f.priority,
            labels: f.labels.unwrap_or_default(),
            phase_id: f.phase_id,
            created_at: f.created_at.to_rfc3339(),
            updated_at: f.updated_at.to_rfc3339(),
        }
    }
}

/// Request to create a new feature
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateFeatureRequest {
    pub project_id: String,
    pub title: String,
    pub description: Option<String>,
    pub acceptance_criteria: Option<String>,
    pub labels: Option<Vec<String>>,
    pub phase_id: Option<String>,
    pub priority: Option<i32>,
}

/// Request to update a feature
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdateFeatureRequest {
    pub title: Option<String>,
    pub description: Option<String>,
    pub acceptance_criteria: Option<String>,
    pub labels: Option<Vec<String>>,
    pub phase_id: Option<String>,
    pub status: Option<String>,
    pub priority: Option<i32>,
}

/// List features for a project
pub async fn list_by_project(
    project_id: &str,
    status: Option<&str>,
) -> Result<Vec<FeatureSummary>> {
    let db = get_database().await?;
    let repo = FeatureRepository::new(&db);

    let status_filter = status.and_then(FeatureStatus::parse);
    let features = repo.list_by_project(project_id, status_filter).await?;

    Ok(features.into_iter().map(FeatureSummary::from).collect())
}

/// Get a single feature by ID
pub async fn get(id: &str) -> Result<Option<FeatureSummary>> {
    let db = get_database().await?;
    let repo = FeatureRepository::new(&db);

    let feature = repo.get(id).await?;
    Ok(feature.map(FeatureSummary::from))
}

/// Create a new feature
pub async fn create(request: CreateFeatureRequest) -> Result<FeatureSummary> {
    let db = get_database().await?;
    let repo = FeatureRepository::new(&db);

    let mut feature = Feature::new(&request.project_id, &request.title);

    if let Some(desc) = request.description {
        feature = feature.with_description(desc);
    }
    if let Some(criteria) = request.acceptance_criteria {
        feature = feature.with_acceptance_criteria(criteria);
    }
    if let Some(labels) = request.labels {
        feature = feature.with_labels(labels);
    }
    if let Some(phase_id) = request.phase_id {
        feature = feature.with_phase(phase_id);
    }
    if let Some(priority) = request.priority {
        feature = feature.with_priority(priority);
    }

    repo.create(&feature).await?;

    Ok(FeatureSummary::from(feature))
}

/// Update a feature's status
pub async fn update_status(id: &str, status: &str) -> Result<()> {
    let db = get_database().await?;
    let repo = FeatureRepository::new(&db);

    let status = FeatureStatus::parse(status)
        .ok_or_else(|| crate::Error::InvalidInput(format!("Invalid status: {}", status)))?;

    repo.update_status(id, status).await?;
    Ok(())
}

/// Delete a feature
pub async fn delete(id: &str) -> Result<()> {
    let db = get_database().await?;
    let repo = FeatureRepository::new(&db);

    repo.delete(id).await?;
    Ok(())
}
