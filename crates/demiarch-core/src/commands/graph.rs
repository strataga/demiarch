//! Knowledge graph exploration commands
//!
//! Commands for analyzing and exploring the knowledge graph built from skills.

use std::collections::HashMap;

use crate::domain::knowledge::{
    EntityType, EntityWithDistance, KnowledgeEntity, KnowledgeGraphRepository,
    KnowledgeRelationship, RelationshipType,
};
use crate::error::Result;
use crate::infrastructure::knowledge::SqliteKnowledgeGraphRepository;
use sqlx::SqlitePool;

/// Statistics about the knowledge graph
#[derive(Debug, Clone)]
pub struct GraphStatistics {
    /// Total number of entities
    pub total_entities: u64,
    /// Total number of relationships
    pub total_relationships: u64,
    /// Breakdown by entity type
    pub entities_by_type: HashMap<EntityType, u64>,
    /// Breakdown by relationship type
    pub relationships_by_type: HashMap<RelationshipType, u64>,
    /// Total skills linked to entities
    pub linked_skills: u64,
    /// Average confidence score
    pub average_confidence: f32,
}

/// Result of exploring an entity's neighborhood
#[derive(Debug, Clone)]
pub struct ExploreResult {
    /// The root entity
    pub root_entity: KnowledgeEntity,
    /// Entities in the neighborhood
    pub neighbors: Vec<EntityWithDistance>,
    /// Relationships within the neighborhood
    pub relationships: Vec<KnowledgeRelationship>,
    /// Path to each neighbor
    pub paths: HashMap<String, Vec<String>>,
}

/// A single entity with its relationships for display
#[derive(Debug, Clone)]
pub struct EntityWithRelationships {
    /// The entity
    pub entity: KnowledgeEntity,
    /// Outgoing relationships (this entity is source)
    pub outgoing: Vec<(RelationshipType, String)>,
    /// Incoming relationships (this entity is target)
    pub incoming: Vec<(RelationshipType, String)>,
}

/// Get knowledge graph statistics
pub async fn get_stats(pool: &SqlitePool) -> Result<GraphStatistics> {
    let repo = SqliteKnowledgeGraphRepository::new(pool.clone());

    let base_stats = repo.get_stats().await?;
    let all_entities = repo.list_entities().await?;

    // Calculate entities by type
    let mut entities_by_type: HashMap<EntityType, u64> = HashMap::new();
    let mut total_confidence = 0.0f32;

    for entity in &all_entities {
        *entities_by_type.entry(entity.entity_type).or_insert(0) += 1;
        total_confidence += entity.confidence;
    }

    let average_confidence = if !all_entities.is_empty() {
        total_confidence / all_entities.len() as f32
    } else {
        0.0
    };

    // Calculate relationships by type
    let mut relationships_by_type: HashMap<RelationshipType, u64> = HashMap::new();
    for entity in &all_entities {
        let rels = repo.list_relationships_for_entity(&entity.id).await?;
        for rel in rels {
            *relationships_by_type
                .entry(rel.relationship_type)
                .or_insert(0) += 1;
        }
    }

    // Count linked skills
    let linked_skills = base_stats.total_skill_links;

    Ok(GraphStatistics {
        total_entities: base_stats.total_entities,
        total_relationships: base_stats.total_relationships,
        entities_by_type,
        relationships_by_type,
        linked_skills,
        average_confidence,
    })
}

/// Explore an entity's neighborhood
pub async fn explore_entity(
    pool: &SqlitePool,
    entity_query: &str,
    max_depth: u32,
    relationship_filter: Option<RelationshipType>,
) -> Result<Option<ExploreResult>> {
    let repo = SqliteKnowledgeGraphRepository::new(pool.clone());

    // First, try to find the entity by name search
    let search_results = repo.search_entities(entity_query, 5).await?;

    let root_entity = if let Some(exact_match) = search_results
        .iter()
        .find(|e| e.name.to_lowercase() == entity_query.to_lowercase())
    {
        exact_match.clone()
    } else if let Some(first) = search_results.first() {
        first.clone()
    } else {
        // Try by canonical name
        if let Some(entity) = repo
            .get_entity_by_canonical_name(&KnowledgeEntity::canonicalize(entity_query))
            .await?
        {
            entity
        } else {
            return Ok(None);
        }
    };

    // Get neighborhood
    let relationship_types: Option<Vec<RelationshipType>> = relationship_filter.map(|t| vec![t]);

    let neighbors = repo
        .get_neighborhood(&root_entity.id, max_depth, relationship_types.as_deref())
        .await?;

    // Gather relationships between root and neighbors
    let root_rels = repo.list_relationships_for_entity(&root_entity.id).await?;

    // Build paths map
    let mut paths: HashMap<String, Vec<String>> = HashMap::new();
    for neighbor in &neighbors {
        paths.insert(neighbor.entity.id.clone(), neighbor.path.clone());
    }

    Ok(Some(ExploreResult {
        root_entity,
        neighbors,
        relationships: root_rels,
        paths,
    }))
}

/// List all entities of a specific type
pub async fn list_entities_by_type(
    pool: &SqlitePool,
    entity_type: EntityType,
) -> Result<Vec<KnowledgeEntity>> {
    let repo = SqliteKnowledgeGraphRepository::new(pool.clone());
    repo.list_entities_by_type(entity_type).await
}

/// Search entities by query
pub async fn search_entities(
    pool: &SqlitePool,
    query: &str,
    limit: usize,
) -> Result<Vec<KnowledgeEntity>> {
    let repo = SqliteKnowledgeGraphRepository::new(pool.clone());
    repo.search_entities(query, limit).await
}

/// Get entity with all its relationships
pub async fn get_entity_details(
    pool: &SqlitePool,
    entity_query: &str,
) -> Result<Option<EntityWithRelationships>> {
    let repo = SqliteKnowledgeGraphRepository::new(pool.clone());

    // Find the entity
    let search_results = repo.search_entities(entity_query, 1).await?;
    let entity = if let Some(e) = search_results.first() {
        e.clone()
    } else {
        return Ok(None);
    };

    // Get relationships
    let all_rels = repo.list_relationships_for_entity(&entity.id).await?;

    let mut outgoing = Vec::new();
    let mut incoming = Vec::new();

    for rel in all_rels {
        if rel.source_entity_id == entity.id {
            // Get target entity name
            if let Some(target) = repo.get_entity(&rel.target_entity_id).await? {
                outgoing.push((rel.relationship_type, target.name));
            }
        } else {
            // Get source entity name
            if let Some(source) = repo.get_entity(&rel.source_entity_id).await? {
                incoming.push((rel.relationship_type, source.name));
            }
        }
    }

    Ok(Some(EntityWithRelationships {
        entity,
        outgoing,
        incoming,
    }))
}

/// Get skills linked to an entity
pub async fn get_linked_skills(pool: &SqlitePool, entity_id: &str) -> Result<Vec<String>> {
    let repo = SqliteKnowledgeGraphRepository::new(pool.clone());
    repo.get_skills_for_entity(entity_id).await
}

/// Format graph statistics for display
pub fn format_stats(stats: &GraphStatistics) -> String {
    let mut output = String::new();

    output.push_str("Knowledge Graph Statistics\n");
    output.push_str("==========================\n\n");

    output.push_str(&format!("Total Entities:      {}\n", stats.total_entities));
    output.push_str(&format!(
        "Total Relationships: {}\n",
        stats.total_relationships
    ));
    output.push_str(&format!("Linked Skills:       {}\n", stats.linked_skills));
    output.push_str(&format!(
        "Average Confidence:  {:.1}%\n\n",
        stats.average_confidence * 100.0
    ));

    if !stats.entities_by_type.is_empty() {
        output.push_str("Entities by Type:\n");
        let mut sorted: Vec<_> = stats.entities_by_type.iter().collect();
        sorted.sort_by(|a, b| b.1.cmp(a.1));
        for (entity_type, count) in sorted {
            output.push_str(&format!("  {:20} {}\n", entity_type.as_str(), count));
        }
        output.push('\n');
    }

    if !stats.relationships_by_type.is_empty() {
        output.push_str("Relationships by Type:\n");
        let mut sorted: Vec<_> = stats.relationships_by_type.iter().collect();
        sorted.sort_by(|a, b| b.1.cmp(a.1));
        for (rel_type, count) in sorted {
            output.push_str(&format!("  {:20} {}\n", rel_type.as_str(), count));
        }
    }

    output
}

/// Format explore result as a tree
pub fn format_explore_tree(result: &ExploreResult, max_depth: u32) -> String {
    let mut output = String::new();

    // Root entity
    output.push_str(&format!(
        "{} ({})\n",
        result.root_entity.name,
        result.root_entity.entity_type.as_str()
    ));

    if let Some(desc) = &result.root_entity.description {
        output.push_str(&format!("  {}\n", desc));
    }

    output.push_str(&format!(
        "  Confidence: {:.1}%\n\n",
        result.root_entity.confidence * 100.0
    ));

    // Group neighbors by depth
    let mut by_depth: HashMap<u32, Vec<&EntityWithDistance>> = HashMap::new();
    for neighbor in &result.neighbors {
        by_depth
            .entry(neighbor.distance)
            .or_default()
            .push(neighbor);
    }

    for depth in 1..=max_depth {
        if let Some(entities) = by_depth.get(&depth) {
            output.push_str(&format!("Depth {}:\n", depth));
            for neighbor in entities {
                let indent = "  ".repeat(depth as usize);
                let rel_desc = find_relationship_description(
                    &result.relationships,
                    &result.root_entity.id,
                    &neighbor.entity.id,
                );
                output.push_str(&format!(
                    "{}{} {} ({})\n",
                    indent,
                    rel_desc,
                    neighbor.entity.name,
                    neighbor.entity.entity_type.as_str()
                ));
            }
            output.push('\n');
        }
    }

    output
}

/// Format explore result as a simple list
pub fn format_explore_list(result: &ExploreResult) -> String {
    let mut output = String::new();

    output.push_str(&format!("Entity: {}\n", result.root_entity.name));
    output.push_str(&format!(
        "Type: {}\n",
        result.root_entity.entity_type.as_str()
    ));
    if let Some(desc) = &result.root_entity.description {
        output.push_str(&format!("Description: {}\n", desc));
    }
    output.push_str(&format!(
        "Confidence: {:.1}%\n",
        result.root_entity.confidence * 100.0
    ));
    output.push_str(&format!(
        "Source Skills: {}\n\n",
        result.root_entity.source_skill_ids.len()
    ));

    // Relationships
    output.push_str("Relationships:\n");
    for rel in &result.relationships {
        let direction = if rel.source_entity_id == result.root_entity.id {
            let target_name = result
                .neighbors
                .iter()
                .find(|n| n.entity.id == rel.target_entity_id)
                .map(|n| n.entity.name.as_str())
                .unwrap_or(&rel.target_entity_id);
            format!("-> {} {}", rel.relationship_type.as_str(), target_name)
        } else {
            let source_name = result
                .neighbors
                .iter()
                .find(|n| n.entity.id == rel.source_entity_id)
                .map(|n| n.entity.name.as_str())
                .unwrap_or(&rel.source_entity_id);
            format!("<- {} {}", rel.relationship_type.as_str(), source_name)
        };
        output.push_str(&format!("  {}\n", direction));
    }

    output.push_str(&format!("\nNeighbors ({}):\n", result.neighbors.len()));
    for neighbor in &result.neighbors {
        output.push_str(&format!(
            "  [{}] {} ({})\n",
            neighbor.distance,
            neighbor.entity.name,
            neighbor.entity.entity_type.as_str()
        ));
    }

    output
}

/// Find relationship description between two entities
fn find_relationship_description(
    relationships: &[KnowledgeRelationship],
    from_id: &str,
    to_id: &str,
) -> &'static str {
    for rel in relationships {
        if rel.source_entity_id == from_id && rel.target_entity_id == to_id {
            return match rel.relationship_type {
                RelationshipType::Uses => "->",
                RelationshipType::UsedBy => "<-",
                RelationshipType::DependsOn => "=>",
                RelationshipType::DependencyOf => "<=",
                RelationshipType::SimilarTo => "<>",
                RelationshipType::PartOf => "[]",
                RelationshipType::Contains => "{}",
                _ => "--",
            };
        }
    }
    "--"
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_format_stats() {
        let stats = GraphStatistics {
            total_entities: 100,
            total_relationships: 250,
            entities_by_type: {
                let mut m = HashMap::new();
                m.insert(EntityType::Library, 30);
                m.insert(EntityType::Concept, 50);
                m
            },
            relationships_by_type: {
                let mut m = HashMap::new();
                m.insert(RelationshipType::Uses, 100);
                m.insert(RelationshipType::DependsOn, 50);
                m
            },
            linked_skills: 75,
            average_confidence: 0.78,
        };

        let output = format_stats(&stats);
        assert!(output.contains("Total Entities:"));
        assert!(output.contains("100"));
        assert!(output.contains("78.0%"));
    }
}
