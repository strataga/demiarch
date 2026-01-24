//! Search specifications
//!
//! Composable specifications for filtering search results.

use uuid::Uuid;

use crate::domain::specification::Specification;

use super::entity::{SearchEntityType, SearchResult, SearchScope};

/// Specification for filtering by allowed project IDs (privacy control)
pub struct PrivacyAllowedSpec {
    allowed_project_ids: Vec<Uuid>,
}

impl PrivacyAllowedSpec {
    /// Create a new privacy specification with allowed project IDs
    pub fn new(allowed_project_ids: Vec<Uuid>) -> Self {
        Self {
            allowed_project_ids,
        }
    }

    /// Create a spec that allows all projects
    pub fn allow_all() -> Self {
        Self {
            allowed_project_ids: Vec::new(),
        }
    }
}

impl Specification<SearchResult> for PrivacyAllowedSpec {
    fn is_satisfied_by(&self, result: &SearchResult) -> bool {
        // If no restrictions, allow all
        if self.allowed_project_ids.is_empty() {
            return true;
        }

        // Check if result's project is in allowed list
        self.allowed_project_ids.contains(&result.project_id)
    }
}

/// Specification for filtering by entity type
pub struct EntityTypeSpec {
    allowed_types: Vec<SearchEntityType>,
}

impl EntityTypeSpec {
    /// Create a new entity type specification
    pub fn new(allowed_types: Vec<SearchEntityType>) -> Self {
        Self { allowed_types }
    }

    /// Create a spec that allows all entity types
    pub fn all_types() -> Self {
        Self {
            allowed_types: Vec::new(),
        }
    }

    /// Create a spec for a single entity type
    pub fn single(entity_type: SearchEntityType) -> Self {
        Self {
            allowed_types: vec![entity_type],
        }
    }
}

impl Specification<SearchResult> for EntityTypeSpec {
    fn is_satisfied_by(&self, result: &SearchResult) -> bool {
        // If no restrictions, allow all
        if self.allowed_types.is_empty() {
            return true;
        }

        self.allowed_types.contains(&result.entity_type)
    }
}

/// Specification for filtering by search scope
pub struct ScopeSpec {
    scope: SearchScope,
}

impl ScopeSpec {
    /// Create a new scope specification
    pub fn new(scope: SearchScope) -> Self {
        Self { scope }
    }
}

impl Specification<SearchResult> for ScopeSpec {
    fn is_satisfied_by(&self, result: &SearchResult) -> bool {
        match &self.scope {
            SearchScope::CurrentProject(project_id) => result.project_id == *project_id,
            SearchScope::CrossProject {
                from_project: _,
                target_projects,
            } => {
                if let Some(targets) = target_projects {
                    targets.contains(&result.project_id)
                } else {
                    // No target restriction means all projects are allowed
                    true
                }
            }
            SearchScope::Global => true,
        }
    }
}

/// Specification for minimum relevance score
pub struct MinRelevanceSpec {
    min_score: f64,
}

impl MinRelevanceSpec {
    /// Create a new minimum relevance specification
    pub fn new(min_score: f64) -> Self {
        Self { min_score }
    }
}

impl Specification<SearchResult> for MinRelevanceSpec {
    fn is_satisfied_by(&self, result: &SearchResult) -> bool {
        result.score >= self.min_score
    }
}

/// Builder for composing search specifications
pub struct SearchSpecBuilder {
    privacy_spec: Option<PrivacyAllowedSpec>,
    entity_type_spec: Option<EntityTypeSpec>,
    scope_spec: Option<ScopeSpec>,
    min_relevance_spec: Option<MinRelevanceSpec>,
}

impl SearchSpecBuilder {
    /// Create a new builder
    pub fn new() -> Self {
        Self {
            privacy_spec: None,
            entity_type_spec: None,
            scope_spec: None,
            min_relevance_spec: None,
        }
    }

    /// Add privacy constraints
    pub fn with_privacy(mut self, allowed_project_ids: Vec<Uuid>) -> Self {
        self.privacy_spec = Some(PrivacyAllowedSpec::new(allowed_project_ids));
        self
    }

    /// Add entity type filter
    pub fn with_entity_types(mut self, types: Vec<SearchEntityType>) -> Self {
        self.entity_type_spec = Some(EntityTypeSpec::new(types));
        self
    }

    /// Add scope filter
    pub fn with_scope(mut self, scope: SearchScope) -> Self {
        self.scope_spec = Some(ScopeSpec::new(scope));
        self
    }

    /// Add minimum relevance filter
    pub fn with_min_relevance(mut self, min_score: f64) -> Self {
        self.min_relevance_spec = Some(MinRelevanceSpec::new(min_score));
        self
    }

    /// Check if a result satisfies all specifications
    pub fn is_satisfied_by(&self, result: &SearchResult) -> bool {
        // Check each spec if present
        if let Some(ref spec) = self.privacy_spec {
            if !spec.is_satisfied_by(result) {
                return false;
            }
        }

        if let Some(ref spec) = self.entity_type_spec {
            if !spec.is_satisfied_by(result) {
                return false;
            }
        }

        if let Some(ref spec) = self.scope_spec {
            if !spec.is_satisfied_by(result) {
                return false;
            }
        }

        if let Some(ref spec) = self.min_relevance_spec {
            if !spec.is_satisfied_by(result) {
                return false;
            }
        }

        true
    }

    /// Filter a collection of results
    pub fn filter(&self, results: Vec<SearchResult>) -> Vec<SearchResult> {
        results
            .into_iter()
            .filter(|r| self.is_satisfied_by(r))
            .collect()
    }
}

impl Default for SearchSpecBuilder {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_test_result(
        project_id: Uuid,
        entity_type: SearchEntityType,
        score: f64,
    ) -> SearchResult {
        SearchResult::new(
            Uuid::new_v4().to_string(),
            entity_type,
            project_id,
            "Test",
            "Test snippet",
        )
        .with_score(score)
    }

    #[test]
    fn test_privacy_spec() {
        let project1 = Uuid::new_v4();
        let project2 = Uuid::new_v4();
        let project3 = Uuid::new_v4();

        let spec = PrivacyAllowedSpec::new(vec![project1, project2]);

        let result1 = create_test_result(project1, SearchEntityType::Feature, 1.0);
        let result2 = create_test_result(project3, SearchEntityType::Feature, 1.0);

        assert!(spec.is_satisfied_by(&result1));
        assert!(!spec.is_satisfied_by(&result2));
    }

    #[test]
    fn test_privacy_spec_allow_all() {
        let spec = PrivacyAllowedSpec::allow_all();
        let result = create_test_result(Uuid::new_v4(), SearchEntityType::Feature, 1.0);
        assert!(spec.is_satisfied_by(&result));
    }

    #[test]
    fn test_entity_type_spec() {
        let spec = EntityTypeSpec::new(vec![SearchEntityType::Feature, SearchEntityType::Document]);

        let feature_result = create_test_result(Uuid::new_v4(), SearchEntityType::Feature, 1.0);
        let skill_result = create_test_result(Uuid::new_v4(), SearchEntityType::Skill, 1.0);

        assert!(spec.is_satisfied_by(&feature_result));
        assert!(!spec.is_satisfied_by(&skill_result));
    }

    #[test]
    fn test_min_relevance_spec() {
        let spec = MinRelevanceSpec::new(0.5);

        let high_score = create_test_result(Uuid::new_v4(), SearchEntityType::Feature, 0.8);
        let low_score = create_test_result(Uuid::new_v4(), SearchEntityType::Feature, 0.3);

        assert!(spec.is_satisfied_by(&high_score));
        assert!(!spec.is_satisfied_by(&low_score));
    }

    #[test]
    fn test_search_spec_builder() {
        let project1 = Uuid::new_v4();
        let project2 = Uuid::new_v4();

        let builder = SearchSpecBuilder::new()
            .with_privacy(vec![project1])
            .with_entity_types(vec![SearchEntityType::Feature])
            .with_min_relevance(0.5);

        let matching = create_test_result(project1, SearchEntityType::Feature, 0.8);
        let wrong_project = create_test_result(project2, SearchEntityType::Feature, 0.8);
        let wrong_type = create_test_result(project1, SearchEntityType::Skill, 0.8);
        let low_score = create_test_result(project1, SearchEntityType::Feature, 0.3);

        assert!(builder.is_satisfied_by(&matching));
        assert!(!builder.is_satisfied_by(&wrong_project));
        assert!(!builder.is_satisfied_by(&wrong_type));
        assert!(!builder.is_satisfied_by(&low_score));
    }

    #[test]
    fn test_filter_results() {
        let project = Uuid::new_v4();

        let results = vec![
            create_test_result(project, SearchEntityType::Feature, 0.8),
            create_test_result(project, SearchEntityType::Feature, 0.3),
            create_test_result(project, SearchEntityType::Skill, 0.9),
        ];

        let builder = SearchSpecBuilder::new()
            .with_entity_types(vec![SearchEntityType::Feature])
            .with_min_relevance(0.5);

        let filtered = builder.filter(results);
        assert_eq!(filtered.len(), 1);
        assert_eq!(filtered[0].score, 0.8);
    }
}
