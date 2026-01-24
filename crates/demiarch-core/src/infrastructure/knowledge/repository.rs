//! SQLite implementation of the KnowledgeGraphRepository
//!
//! Uses recursive CTEs for efficient graph traversal and FTS5 for full-text search.

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use sqlx::{FromRow, SqlitePool};
use tracing::{debug, info};

use crate::domain::knowledge::{
    EntitySearchResult, EntityType, EntityWithDistance, KnowledgeEntity,
    KnowledgeGraphRepository, KnowledgeGraphStats, KnowledgeRelationship, PathRelationship,
    PathStep, RelationshipType, TraversalDirection,
};
use crate::error::{Error, Result};

/// SQLite implementation of the knowledge graph repository
#[derive(Clone)]
pub struct SqliteKnowledgeGraphRepository {
    pool: SqlitePool,
}

impl SqliteKnowledgeGraphRepository {
    /// Create a new SQLite knowledge graph repository
    pub fn new(pool: SqlitePool) -> Self {
        Self { pool }
    }
}

#[async_trait]
impl KnowledgeGraphRepository for SqliteKnowledgeGraphRepository {
    // ========== Entity Operations ==========

    async fn save_entity(&self, entity: &KnowledgeEntity) -> Result<()> {
        let aliases_json = serde_json::to_string(&entity.aliases)
            .map_err(|e| Error::Other(format!("Failed to serialize aliases: {}", e)))?;

        let source_skill_ids_json = serde_json::to_string(&entity.source_skill_ids)
            .map_err(|e| Error::Other(format!("Failed to serialize source_skill_ids: {}", e)))?;

        sqlx::query(
            r#"
            INSERT INTO knowledge_entities (
                id, entity_type, name, canonical_name, description,
                aliases, source_skill_ids, confidence, created_at, updated_at
            ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
            ON CONFLICT(id) DO UPDATE SET
                entity_type = excluded.entity_type,
                name = excluded.name,
                canonical_name = excluded.canonical_name,
                description = excluded.description,
                aliases = excluded.aliases,
                source_skill_ids = excluded.source_skill_ids,
                confidence = excluded.confidence,
                updated_at = excluded.updated_at
            "#,
        )
        .bind(&entity.id)
        .bind(entity.entity_type.as_str())
        .bind(&entity.name)
        .bind(&entity.canonical_name)
        .bind(&entity.description)
        .bind(&aliases_json)
        .bind(&source_skill_ids_json)
        .bind(entity.confidence)
        .bind(entity.created_at.to_rfc3339())
        .bind(entity.updated_at.to_rfc3339())
        .execute(&self.pool)
        .await?;

        debug!(entity_id = %entity.id, entity_name = %entity.name, "Entity saved");
        Ok(())
    }

    async fn get_entity(&self, id: &str) -> Result<Option<KnowledgeEntity>> {
        let row: Option<EntityRow> = sqlx::query_as(
            "SELECT * FROM knowledge_entities WHERE id = ?",
        )
        .bind(id)
        .fetch_optional(&self.pool)
        .await?;

        row.map(|r| r.into_entity()).transpose()
    }

    async fn get_entity_by_canonical_name(&self, canonical_name: &str) -> Result<Option<KnowledgeEntity>> {
        let row: Option<EntityRow> = sqlx::query_as(
            "SELECT * FROM knowledge_entities WHERE canonical_name = ?",
        )
        .bind(canonical_name)
        .fetch_optional(&self.pool)
        .await?;

        row.map(|r| r.into_entity()).transpose()
    }

    async fn list_entities(&self) -> Result<Vec<KnowledgeEntity>> {
        let rows: Vec<EntityRow> = sqlx::query_as(
            "SELECT * FROM knowledge_entities ORDER BY confidence DESC, name",
        )
        .fetch_all(&self.pool)
        .await?;

        rows.into_iter().map(|r| r.into_entity()).collect()
    }

    async fn list_entities_by_type(&self, entity_type: EntityType) -> Result<Vec<KnowledgeEntity>> {
        let rows: Vec<EntityRow> = sqlx::query_as(
            "SELECT * FROM knowledge_entities WHERE entity_type = ? ORDER BY confidence DESC, name",
        )
        .bind(entity_type.as_str())
        .fetch_all(&self.pool)
        .await?;

        rows.into_iter().map(|r| r.into_entity()).collect()
    }

    async fn delete_entity(&self, id: &str) -> Result<bool> {
        let result = sqlx::query("DELETE FROM knowledge_entities WHERE id = ?")
            .bind(id)
            .execute(&self.pool)
            .await?;

        let deleted = result.rows_affected() > 0;
        if deleted {
            info!(entity_id = %id, "Entity deleted");
        }
        Ok(deleted)
    }

    async fn count_entities(&self) -> Result<u64> {
        let (count,): (i64,) = sqlx::query_as("SELECT COUNT(*) FROM knowledge_entities")
            .fetch_one(&self.pool)
            .await?;
        Ok(count as u64)
    }

    // ========== Relationship Operations ==========

    async fn save_relationship(&self, relationship: &KnowledgeRelationship) -> Result<()> {
        let evidence_json = serde_json::to_string(&relationship.evidence)
            .map_err(|e| Error::Other(format!("Failed to serialize evidence: {}", e)))?;

        sqlx::query(
            r#"
            INSERT INTO knowledge_relationships (
                id, source_entity_id, target_entity_id, relationship_type,
                weight, evidence, created_at, updated_at
            ) VALUES (?, ?, ?, ?, ?, ?, ?, ?)
            ON CONFLICT(source_entity_id, target_entity_id, relationship_type) DO UPDATE SET
                weight = excluded.weight,
                evidence = excluded.evidence,
                updated_at = excluded.updated_at
            "#,
        )
        .bind(&relationship.id)
        .bind(&relationship.source_entity_id)
        .bind(&relationship.target_entity_id)
        .bind(relationship.relationship_type.as_str())
        .bind(relationship.weight)
        .bind(&evidence_json)
        .bind(relationship.created_at.to_rfc3339())
        .bind(relationship.updated_at.to_rfc3339())
        .execute(&self.pool)
        .await?;

        debug!(
            relationship_id = %relationship.id,
            source = %relationship.source_entity_id,
            target = %relationship.target_entity_id,
            "Relationship saved"
        );
        Ok(())
    }

    async fn get_relationship(&self, id: &str) -> Result<Option<KnowledgeRelationship>> {
        let row: Option<RelationshipRow> = sqlx::query_as(
            "SELECT * FROM knowledge_relationships WHERE id = ?",
        )
        .bind(id)
        .fetch_optional(&self.pool)
        .await?;

        row.map(|r| r.into_relationship()).transpose()
    }

    async fn get_relationship_between(
        &self,
        source_id: &str,
        target_id: &str,
        relationship_type: RelationshipType,
    ) -> Result<Option<KnowledgeRelationship>> {
        let row: Option<RelationshipRow> = sqlx::query_as(
            r#"
            SELECT * FROM knowledge_relationships
            WHERE source_entity_id = ? AND target_entity_id = ? AND relationship_type = ?
            "#,
        )
        .bind(source_id)
        .bind(target_id)
        .bind(relationship_type.as_str())
        .fetch_optional(&self.pool)
        .await?;

        row.map(|r| r.into_relationship()).transpose()
    }

    async fn list_relationships_for_entity(&self, entity_id: &str) -> Result<Vec<KnowledgeRelationship>> {
        let rows: Vec<RelationshipRow> = sqlx::query_as(
            r#"
            SELECT * FROM knowledge_relationships
            WHERE source_entity_id = ? OR target_entity_id = ?
            ORDER BY weight DESC
            "#,
        )
        .bind(entity_id)
        .bind(entity_id)
        .fetch_all(&self.pool)
        .await?;

        rows.into_iter().map(|r| r.into_relationship()).collect()
    }

    async fn list_outgoing_relationships(&self, entity_id: &str) -> Result<Vec<KnowledgeRelationship>> {
        let rows: Vec<RelationshipRow> = sqlx::query_as(
            "SELECT * FROM knowledge_relationships WHERE source_entity_id = ? ORDER BY weight DESC",
        )
        .bind(entity_id)
        .fetch_all(&self.pool)
        .await?;

        rows.into_iter().map(|r| r.into_relationship()).collect()
    }

    async fn list_incoming_relationships(&self, entity_id: &str) -> Result<Vec<KnowledgeRelationship>> {
        let rows: Vec<RelationshipRow> = sqlx::query_as(
            "SELECT * FROM knowledge_relationships WHERE target_entity_id = ? ORDER BY weight DESC",
        )
        .bind(entity_id)
        .fetch_all(&self.pool)
        .await?;

        rows.into_iter().map(|r| r.into_relationship()).collect()
    }

    async fn delete_relationship(&self, id: &str) -> Result<bool> {
        let result = sqlx::query("DELETE FROM knowledge_relationships WHERE id = ?")
            .bind(id)
            .execute(&self.pool)
            .await?;

        let deleted = result.rows_affected() > 0;
        if deleted {
            info!(relationship_id = %id, "Relationship deleted");
        }
        Ok(deleted)
    }

    async fn count_relationships(&self) -> Result<u64> {
        let (count,): (i64,) = sqlx::query_as("SELECT COUNT(*) FROM knowledge_relationships")
            .fetch_one(&self.pool)
            .await?;
        Ok(count as u64)
    }

    // ========== Graph Traversal Operations ==========

    async fn get_neighborhood(
        &self,
        start_entity_id: &str,
        max_depth: u32,
        relationship_types: Option<&[RelationshipType]>,
    ) -> Result<Vec<EntityWithDistance>> {
        // Build the relationship type filter
        let type_filter = if let Some(types) = relationship_types {
            let types_str: Vec<&str> = types.iter().map(|t| t.as_str()).collect();
            format!(
                "AND r.relationship_type IN ({})",
                types_str.iter().map(|_| "?").collect::<Vec<_>>().join(", ")
            )
        } else {
            String::new()
        };

        // Use recursive CTE for graph traversal
        let query = format!(
            r#"
            WITH RECURSIVE reachable(entity_id, depth, path) AS (
                -- Base case: start entity
                SELECT ?, 0, ?

                UNION ALL

                -- Recursive case: follow relationships
                SELECT
                    CASE
                        WHEN r.source_entity_id = prev.entity_id THEN r.target_entity_id
                        ELSE r.source_entity_id
                    END,
                    prev.depth + 1,
                    prev.path || ',' || CASE
                        WHEN r.source_entity_id = prev.entity_id THEN r.target_entity_id
                        ELSE r.source_entity_id
                    END
                FROM reachable prev
                JOIN knowledge_relationships r ON (
                    r.source_entity_id = prev.entity_id OR r.target_entity_id = prev.entity_id
                )
                WHERE prev.depth < ?
                    AND prev.path NOT LIKE '%' || CASE
                        WHEN r.source_entity_id = prev.entity_id THEN r.target_entity_id
                        ELSE r.source_entity_id
                    END || '%'
                    {}
            )
            SELECT DISTINCT
                e.*,
                MIN(r.depth) as distance,
                (SELECT path FROM reachable WHERE entity_id = e.id ORDER BY depth LIMIT 1) as path
            FROM knowledge_entities e
            JOIN reachable r ON e.id = r.entity_id
            WHERE e.id != ?
            GROUP BY e.id
            ORDER BY distance, e.confidence DESC
            "#,
            type_filter
        );

        // Build the query with bindings
        let mut query_builder = sqlx::query_as::<_, NeighborhoodRow>(&query)
            .bind(start_entity_id)
            .bind(start_entity_id)
            .bind(max_depth as i32);

        // Add relationship type bindings if specified
        if let Some(types) = relationship_types {
            for t in types {
                query_builder = query_builder.bind(t.as_str());
            }
        }

        query_builder = query_builder.bind(start_entity_id);

        let rows: Vec<NeighborhoodRow> = query_builder.fetch_all(&self.pool).await?;

        rows.into_iter()
            .map(|r| {
                let entity = EntityRow {
                    id: r.id,
                    entity_type: r.entity_type,
                    name: r.name,
                    canonical_name: r.canonical_name,
                    description: r.description,
                    aliases: r.aliases,
                    source_skill_ids: r.source_skill_ids,
                    confidence: r.confidence,
                    created_at: r.created_at,
                    updated_at: r.updated_at,
                }
                .into_entity()?;

                let path: Vec<String> = r
                    .path
                    .unwrap_or_default()
                    .split(',')
                    .filter(|s| !s.is_empty())
                    .map(String::from)
                    .collect();

                Ok(EntityWithDistance {
                    entity,
                    distance: r.distance as u32,
                    path,
                })
            })
            .collect()
    }

    async fn find_path(
        &self,
        source_id: &str,
        target_id: &str,
        max_depth: u32,
    ) -> Result<Option<Vec<PathStep>>> {
        // Use recursive CTE to find shortest path
        let rows: Vec<PathRow> = sqlx::query_as(
            r#"
            WITH RECURSIVE path_finder(entity_id, depth, path, rel_path) AS (
                SELECT ?, 0, ?, ''

                UNION ALL

                SELECT
                    CASE
                        WHEN r.source_entity_id = prev.entity_id THEN r.target_entity_id
                        ELSE r.source_entity_id
                    END,
                    prev.depth + 1,
                    prev.path || ',' || CASE
                        WHEN r.source_entity_id = prev.entity_id THEN r.target_entity_id
                        ELSE r.source_entity_id
                    END,
                    prev.rel_path || CASE WHEN prev.rel_path = '' THEN '' ELSE ',' END
                        || r.id || ':' || r.relationship_type || ':' || r.weight
                FROM path_finder prev
                JOIN knowledge_relationships r ON (
                    r.source_entity_id = prev.entity_id OR r.target_entity_id = prev.entity_id
                )
                WHERE prev.depth < ?
                    AND prev.path NOT LIKE '%' || CASE
                        WHEN r.source_entity_id = prev.entity_id THEN r.target_entity_id
                        ELSE r.source_entity_id
                    END || '%'
            )
            SELECT path, rel_path, depth
            FROM path_finder
            WHERE entity_id = ?
            ORDER BY depth
            LIMIT 1
            "#,
        )
        .bind(source_id)
        .bind(source_id)
        .bind(max_depth as i32)
        .bind(target_id)
        .fetch_all(&self.pool)
        .await?;

        if rows.is_empty() {
            return Ok(None);
        }

        let row = &rows[0];
        let entity_ids: Vec<&str> = row.path.split(',').filter(|s| !s.is_empty()).collect();
        let rel_parts: Vec<&str> = row.rel_path.split(',').filter(|s| !s.is_empty()).collect();

        let mut steps = Vec::with_capacity(entity_ids.len());

        for (i, entity_id) in entity_ids.iter().enumerate() {
            let relationship = if i > 0 && i - 1 < rel_parts.len() {
                let parts: Vec<&str> = rel_parts[i - 1].split(':').collect();
                if parts.len() >= 3 {
                    Some(PathRelationship {
                        relationship_id: parts[0].to_string(),
                        relationship_type: RelationshipType::parse(parts[1])
                            .unwrap_or(RelationshipType::RelatedTo),
                        weight: parts[2].parse().unwrap_or(0.5),
                    })
                } else {
                    None
                }
            } else {
                None
            };

            steps.push(PathStep {
                entity_id: entity_id.to_string(),
                relationship,
            });
        }

        Ok(Some(steps))
    }

    async fn get_connected_entities(
        &self,
        entity_id: &str,
        relationship_type: RelationshipType,
        direction: TraversalDirection,
    ) -> Result<Vec<KnowledgeEntity>> {
        let query = match direction {
            TraversalDirection::Outgoing => {
                r#"
                SELECT e.* FROM knowledge_entities e
                JOIN knowledge_relationships r ON e.id = r.target_entity_id
                WHERE r.source_entity_id = ? AND r.relationship_type = ?
                ORDER BY r.weight DESC
                "#
            }
            TraversalDirection::Incoming => {
                r#"
                SELECT e.* FROM knowledge_entities e
                JOIN knowledge_relationships r ON e.id = r.source_entity_id
                WHERE r.target_entity_id = ? AND r.relationship_type = ?
                ORDER BY r.weight DESC
                "#
            }
            TraversalDirection::Both => {
                r#"
                SELECT DISTINCT e.* FROM knowledge_entities e
                JOIN knowledge_relationships r ON (
                    (e.id = r.target_entity_id AND r.source_entity_id = ?)
                    OR (e.id = r.source_entity_id AND r.target_entity_id = ?)
                )
                WHERE r.relationship_type = ?
                ORDER BY e.confidence DESC
                "#
            }
        };

        let rows: Vec<EntityRow> = match direction {
            TraversalDirection::Both => {
                sqlx::query_as(query)
                    .bind(entity_id)
                    .bind(entity_id)
                    .bind(relationship_type.as_str())
                    .fetch_all(&self.pool)
                    .await?
            }
            _ => {
                sqlx::query_as(query)
                    .bind(entity_id)
                    .bind(relationship_type.as_str())
                    .fetch_all(&self.pool)
                    .await?
            }
        };

        rows.into_iter().map(|r| r.into_entity()).collect()
    }

    // ========== Search Operations ==========

    async fn search_entities(&self, query: &str, limit: usize) -> Result<Vec<KnowledgeEntity>> {
        let rows: Vec<EntityRow> = sqlx::query_as(
            r#"
            SELECT e.* FROM knowledge_entities e
            JOIN knowledge_entities_fts fts ON e.rowid = fts.rowid
            WHERE knowledge_entities_fts MATCH ?
            ORDER BY rank, e.confidence DESC
            LIMIT ?
            "#,
        )
        .bind(format!("{}*", query))
        .bind(limit as i32)
        .fetch_all(&self.pool)
        .await?;

        rows.into_iter().map(|r| r.into_entity()).collect()
    }

    async fn get_entities_for_skill(&self, skill_id: &str) -> Result<Vec<KnowledgeEntity>> {
        let rows: Vec<EntityRow> = sqlx::query_as(
            r#"
            SELECT e.* FROM knowledge_entities e
            JOIN skill_entity_links l ON e.id = l.entity_id
            WHERE l.skill_id = ?
            ORDER BY l.relevance DESC
            "#,
        )
        .bind(skill_id)
        .fetch_all(&self.pool)
        .await?;

        rows.into_iter().map(|r| r.into_entity()).collect()
    }

    async fn get_skills_for_entity(&self, entity_id: &str) -> Result<Vec<String>> {
        let rows: Vec<(String,)> = sqlx::query_as(
            "SELECT skill_id FROM skill_entity_links WHERE entity_id = ? ORDER BY relevance DESC",
        )
        .bind(entity_id)
        .fetch_all(&self.pool)
        .await?;

        Ok(rows.into_iter().map(|(id,)| id).collect())
    }

    // ========== Skill-Entity Link Operations ==========

    async fn link_skill_to_entity(
        &self,
        skill_id: &str,
        entity_id: &str,
        relevance: f32,
    ) -> Result<()> {
        sqlx::query(
            r#"
            INSERT INTO skill_entity_links (skill_id, entity_id, relevance, created_at)
            VALUES (?, ?, ?, ?)
            ON CONFLICT(skill_id, entity_id) DO UPDATE SET
                relevance = excluded.relevance
            "#,
        )
        .bind(skill_id)
        .bind(entity_id)
        .bind(relevance)
        .bind(Utc::now().to_rfc3339())
        .execute(&self.pool)
        .await?;

        debug!(skill_id = %skill_id, entity_id = %entity_id, "Skill-entity link created");
        Ok(())
    }

    async fn unlink_skill_from_entity(&self, skill_id: &str, entity_id: &str) -> Result<bool> {
        let result = sqlx::query(
            "DELETE FROM skill_entity_links WHERE skill_id = ? AND entity_id = ?",
        )
        .bind(skill_id)
        .bind(entity_id)
        .execute(&self.pool)
        .await?;

        Ok(result.rows_affected() > 0)
    }

    // ========== Entity Embedding Operations ==========

    async fn save_entity_embedding(
        &self,
        entity_id: &str,
        embedding: &[f32],
        model: &str,
    ) -> Result<()> {
        let id = uuid::Uuid::new_v4().to_string();
        let embedding_bytes: Vec<u8> = embedding.iter().flat_map(|f| f.to_le_bytes()).collect();
        let dimensions = embedding.len() as i32;
        let now = Utc::now().to_rfc3339();

        sqlx::query(
            r#"
            INSERT INTO entity_embeddings (id, entity_id, embedding_model, embedding, dimensions, created_at, updated_at)
            VALUES (?, ?, ?, ?, ?, ?, ?)
            ON CONFLICT(entity_id, embedding_model) DO UPDATE SET
                embedding = excluded.embedding,
                dimensions = excluded.dimensions,
                updated_at = excluded.updated_at
            "#,
        )
        .bind(&id)
        .bind(entity_id)
        .bind(model)
        .bind(&embedding_bytes)
        .bind(dimensions)
        .bind(&now)
        .bind(&now)
        .execute(&self.pool)
        .await?;

        debug!(entity_id = %entity_id, model = %model, "Entity embedding saved");
        Ok(())
    }

    async fn get_entity_embedding(
        &self,
        entity_id: &str,
        model: &str,
    ) -> Result<Option<Vec<f32>>> {
        let row: Option<(Vec<u8>,)> = sqlx::query_as(
            "SELECT embedding FROM entity_embeddings WHERE entity_id = ? AND embedding_model = ?",
        )
        .bind(entity_id)
        .bind(model)
        .fetch_optional(&self.pool)
        .await?;

        Ok(row.map(|(bytes,)| {
            bytes
                .chunks_exact(4)
                .map(|chunk| f32::from_le_bytes([chunk[0], chunk[1], chunk[2], chunk[3]]))
                .collect()
        }))
    }

    async fn semantic_search_entities(
        &self,
        query_embedding: &[f32],
        model: &str,
        limit: usize,
        min_similarity: f32,
    ) -> Result<Vec<EntitySearchResult>> {
        // Fetch all embeddings for the model
        let rows: Vec<EmbeddingRow> = sqlx::query_as(
            "SELECT entity_id, embedding FROM entity_embeddings WHERE embedding_model = ?",
        )
        .bind(model)
        .fetch_all(&self.pool)
        .await?;

        if rows.is_empty() {
            return Ok(Vec::new());
        }

        // Compute similarities
        let mut results: Vec<(String, f32)> = rows
            .into_iter()
            .filter_map(|row| {
                let embedding: Vec<f32> = row
                    .embedding
                    .chunks_exact(4)
                    .map(|chunk| f32::from_le_bytes([chunk[0], chunk[1], chunk[2], chunk[3]]))
                    .collect();
                let similarity = cosine_similarity(query_embedding, &embedding);
                if similarity >= min_similarity {
                    Some((row.entity_id, similarity))
                } else {
                    None
                }
            })
            .collect();

        // Sort by similarity
        results.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
        results.truncate(limit);

        // Fetch the entities
        let mut search_results = Vec::with_capacity(results.len());
        for (entity_id, similarity) in results {
            if let Some(entity) = self.get_entity(&entity_id).await? {
                search_results.push(EntitySearchResult { entity, similarity });
            }
        }

        Ok(search_results)
    }

    // ========== Statistics ==========

    async fn get_stats(&self) -> Result<KnowledgeGraphStats> {
        let (total_entities,): (i64,) =
            sqlx::query_as("SELECT COUNT(*) FROM knowledge_entities")
                .fetch_one(&self.pool)
                .await?;

        let (total_relationships,): (i64,) =
            sqlx::query_as("SELECT COUNT(*) FROM knowledge_relationships")
                .fetch_one(&self.pool)
                .await?;

        let (total_skill_links,): (i64,) =
            sqlx::query_as("SELECT COUNT(*) FROM skill_entity_links")
                .fetch_one(&self.pool)
                .await?;

        let (avg_confidence,): (Option<f64>,) =
            sqlx::query_as("SELECT AVG(confidence) FROM knowledge_entities")
                .fetch_one(&self.pool)
                .await?;

        let (avg_weight,): (Option<f64>,) =
            sqlx::query_as("SELECT AVG(weight) FROM knowledge_relationships")
                .fetch_one(&self.pool)
                .await?;

        let (entities_with_embeddings,): (i64,) =
            sqlx::query_as("SELECT COUNT(DISTINCT entity_id) FROM entity_embeddings")
                .fetch_one(&self.pool)
                .await?;

        let entities_by_type: Vec<(String, i64)> = sqlx::query_as(
            "SELECT entity_type, COUNT(*) FROM knowledge_entities GROUP BY entity_type ORDER BY COUNT(*) DESC",
        )
        .fetch_all(&self.pool)
        .await?;

        let relationships_by_type: Vec<(String, i64)> = sqlx::query_as(
            "SELECT relationship_type, COUNT(*) FROM knowledge_relationships GROUP BY relationship_type ORDER BY COUNT(*) DESC",
        )
        .fetch_all(&self.pool)
        .await?;

        Ok(KnowledgeGraphStats {
            total_entities: total_entities as u64,
            total_relationships: total_relationships as u64,
            total_skill_links: total_skill_links as u64,
            entities_by_type: entities_by_type
                .into_iter()
                .filter_map(|(t, c)| EntityType::parse(&t).map(|et| (et, c as u64)))
                .collect(),
            relationships_by_type: relationships_by_type
                .into_iter()
                .filter_map(|(t, c)| RelationshipType::parse(&t).map(|rt| (rt, c as u64)))
                .collect(),
            average_entity_confidence: avg_confidence.unwrap_or(0.0) as f32,
            average_relationship_weight: avg_weight.unwrap_or(0.0) as f32,
            entities_with_embeddings: entities_with_embeddings as u64,
        })
    }
}

/// Compute cosine similarity between two vectors
fn cosine_similarity(a: &[f32], b: &[f32]) -> f32 {
    if a.len() != b.len() {
        return 0.0;
    }

    let dot_product: f32 = a.iter().zip(b.iter()).map(|(x, y)| x * y).sum();
    let magnitude_a: f32 = a.iter().map(|x| x * x).sum::<f32>().sqrt();
    let magnitude_b: f32 = b.iter().map(|x| x * x).sum::<f32>().sqrt();

    if magnitude_a == 0.0 || magnitude_b == 0.0 {
        return 0.0;
    }

    dot_product / (magnitude_a * magnitude_b)
}

// ========== Database Row Types ==========

#[derive(Debug, FromRow)]
struct EntityRow {
    id: String,
    entity_type: String,
    name: String,
    canonical_name: String,
    description: Option<String>,
    aliases: Option<String>,
    source_skill_ids: Option<String>,
    confidence: f32,
    created_at: String,
    updated_at: String,
}

impl EntityRow {
    fn into_entity(self) -> Result<KnowledgeEntity> {
        let entity_type = EntityType::parse(&self.entity_type)
            .ok_or_else(|| Error::Other(format!("Invalid entity type: {}", self.entity_type)))?;

        let aliases: Vec<String> = self
            .aliases
            .as_ref()
            .map(|s| serde_json::from_str(s).unwrap_or_default())
            .unwrap_or_default();

        let source_skill_ids: Vec<String> = self
            .source_skill_ids
            .as_ref()
            .map(|s| serde_json::from_str(s).unwrap_or_default())
            .unwrap_or_default();

        let created_at = DateTime::parse_from_rfc3339(&self.created_at)
            .map(|dt| dt.with_timezone(&Utc))
            .unwrap_or_else(|_| Utc::now());

        let updated_at = DateTime::parse_from_rfc3339(&self.updated_at)
            .map(|dt| dt.with_timezone(&Utc))
            .unwrap_or_else(|_| Utc::now());

        Ok(KnowledgeEntity {
            id: self.id,
            entity_type,
            name: self.name,
            canonical_name: self.canonical_name,
            description: self.description,
            aliases,
            source_skill_ids,
            confidence: self.confidence,
            created_at,
            updated_at,
        })
    }
}

#[derive(Debug, FromRow)]
struct RelationshipRow {
    id: String,
    source_entity_id: String,
    target_entity_id: String,
    relationship_type: String,
    weight: f32,
    evidence: Option<String>,
    created_at: String,
    updated_at: String,
}

impl RelationshipRow {
    fn into_relationship(self) -> Result<KnowledgeRelationship> {
        let relationship_type = RelationshipType::parse(&self.relationship_type).ok_or_else(|| {
            Error::Other(format!("Invalid relationship type: {}", self.relationship_type))
        })?;

        let evidence = self
            .evidence
            .as_ref()
            .map(|s| serde_json::from_str(s).unwrap_or_default())
            .unwrap_or_default();

        let created_at = DateTime::parse_from_rfc3339(&self.created_at)
            .map(|dt| dt.with_timezone(&Utc))
            .unwrap_or_else(|_| Utc::now());

        let updated_at = DateTime::parse_from_rfc3339(&self.updated_at)
            .map(|dt| dt.with_timezone(&Utc))
            .unwrap_or_else(|_| Utc::now());

        Ok(KnowledgeRelationship {
            id: self.id,
            source_entity_id: self.source_entity_id,
            target_entity_id: self.target_entity_id,
            relationship_type,
            weight: self.weight,
            evidence,
            created_at,
            updated_at,
        })
    }
}

#[derive(Debug, FromRow)]
struct NeighborhoodRow {
    id: String,
    entity_type: String,
    name: String,
    canonical_name: String,
    description: Option<String>,
    aliases: Option<String>,
    source_skill_ids: Option<String>,
    confidence: f32,
    created_at: String,
    updated_at: String,
    distance: i32,
    path: Option<String>,
}

#[derive(Debug, FromRow)]
struct PathRow {
    path: String,
    rel_path: String,
    #[allow(dead_code)]
    depth: i32,
}

#[derive(Debug, FromRow)]
struct EmbeddingRow {
    entity_id: String,
    embedding: Vec<u8>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::storage::migrations::run_migrations;
    use sqlx::sqlite::SqlitePoolOptions;

    async fn setup_test_db() -> SqliteKnowledgeGraphRepository {
        let pool = SqlitePoolOptions::new()
            .max_connections(1)
            .connect("sqlite::memory:")
            .await
            .expect("Failed to create test pool");

        run_migrations(&pool)
            .await
            .expect("Failed to run migrations");

        SqliteKnowledgeGraphRepository::new(pool)
    }

    async fn create_test_skill(repo: &SqliteKnowledgeGraphRepository, skill_id: &str) {
        sqlx::query(
            r#"
            INSERT INTO learned_skills (
                id, name, description, category, pattern_type, pattern_template,
                confidence, times_used, last_used_at, created_at, updated_at
            ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
            "#,
        )
        .bind(skill_id)
        .bind("Test Skill")
        .bind("A test skill for testing")
        .bind("other")
        .bind("code_snippet")
        .bind("test template")
        .bind("high") // confidence is TEXT: 'low', 'medium', 'high'
        .bind(0)
        .bind(Utc::now().to_rfc3339())
        .bind(Utc::now().to_rfc3339())
        .bind(Utc::now().to_rfc3339())
        .execute(&repo.pool)
        .await
        .expect("Failed to create test skill");
    }

    #[tokio::test]
    async fn test_save_and_get_entity() {
        let repo = setup_test_db().await;

        let entity = KnowledgeEntity::new("tokio", EntityType::Library)
            .with_description("Async runtime for Rust")
            .with_confidence(0.8);

        repo.save_entity(&entity).await.unwrap();

        let retrieved = repo.get_entity(&entity.id).await.unwrap();
        assert!(retrieved.is_some());

        let retrieved = retrieved.unwrap();
        assert_eq!(retrieved.name, "tokio");
        assert_eq!(retrieved.entity_type, EntityType::Library);
        assert_eq!(retrieved.confidence, 0.8);
    }

    #[tokio::test]
    async fn test_save_and_get_relationship() {
        let repo = setup_test_db().await;

        let entity1 = KnowledgeEntity::new("tokio", EntityType::Library);
        let entity2 = KnowledgeEntity::new("async-trait", EntityType::Library);

        repo.save_entity(&entity1).await.unwrap();
        repo.save_entity(&entity2).await.unwrap();

        let rel = KnowledgeRelationship::new(&entity1.id, &entity2.id, RelationshipType::Uses)
            .with_weight(0.9);

        repo.save_relationship(&rel).await.unwrap();

        let retrieved = repo.get_relationship(&rel.id).await.unwrap();
        assert!(retrieved.is_some());

        let retrieved = retrieved.unwrap();
        assert_eq!(retrieved.source_entity_id, entity1.id);
        assert_eq!(retrieved.target_entity_id, entity2.id);
        assert_eq!(retrieved.relationship_type, RelationshipType::Uses);
    }

    #[tokio::test]
    async fn test_get_neighborhood() {
        let repo = setup_test_db().await;

        // Create a graph: A -> B -> C
        let a = KnowledgeEntity::new("A", EntityType::Concept);
        let b = KnowledgeEntity::new("B", EntityType::Concept);
        let c = KnowledgeEntity::new("C", EntityType::Concept);

        repo.save_entity(&a).await.unwrap();
        repo.save_entity(&b).await.unwrap();
        repo.save_entity(&c).await.unwrap();

        let rel_ab = KnowledgeRelationship::new(&a.id, &b.id, RelationshipType::Uses);
        let rel_bc = KnowledgeRelationship::new(&b.id, &c.id, RelationshipType::Uses);

        repo.save_relationship(&rel_ab).await.unwrap();
        repo.save_relationship(&rel_bc).await.unwrap();

        // Get 1-hop neighborhood from A
        let neighbors = repo.get_neighborhood(&a.id, 1, None).await.unwrap();
        assert_eq!(neighbors.len(), 1);
        assert_eq!(neighbors[0].entity.name, "B");
        assert_eq!(neighbors[0].distance, 1);

        // Get 2-hop neighborhood from A
        let neighbors = repo.get_neighborhood(&a.id, 2, None).await.unwrap();
        assert_eq!(neighbors.len(), 2);
    }

    #[tokio::test]
    async fn test_find_path() {
        let repo = setup_test_db().await;

        let a = KnowledgeEntity::new("A", EntityType::Concept);
        let b = KnowledgeEntity::new("B", EntityType::Concept);
        let c = KnowledgeEntity::new("C", EntityType::Concept);

        repo.save_entity(&a).await.unwrap();
        repo.save_entity(&b).await.unwrap();
        repo.save_entity(&c).await.unwrap();

        let rel_ab = KnowledgeRelationship::new(&a.id, &b.id, RelationshipType::Uses);
        let rel_bc = KnowledgeRelationship::new(&b.id, &c.id, RelationshipType::Uses);

        repo.save_relationship(&rel_ab).await.unwrap();
        repo.save_relationship(&rel_bc).await.unwrap();

        let path = repo.find_path(&a.id, &c.id, 3).await.unwrap();
        assert!(path.is_some());

        let path = path.unwrap();
        assert_eq!(path.len(), 3); // A -> B -> C
        assert_eq!(path[0].entity_id, a.id);
        assert_eq!(path[2].entity_id, c.id);
    }

    #[tokio::test]
    async fn test_skill_entity_links() {
        let repo = setup_test_db().await;

        let entity = KnowledgeEntity::new("tokio", EntityType::Library);
        repo.save_entity(&entity).await.unwrap();

        // We need a skill in the learned_skills table first
        // For this test, we'll just test the link table directly
        // In production, the skill would exist

        let skills = repo.get_skills_for_entity(&entity.id).await.unwrap();
        assert!(skills.is_empty());
    }

    #[tokio::test]
    async fn test_search_entities() {
        let repo = setup_test_db().await;

        let e1 = KnowledgeEntity::new("tokio", EntityType::Library)
            .with_description("Async runtime for Rust");
        let e2 = KnowledgeEntity::new("async-std", EntityType::Library)
            .with_description("Async standard library");

        repo.save_entity(&e1).await.unwrap();
        repo.save_entity(&e2).await.unwrap();

        let results = repo.search_entities("async", 10).await.unwrap();
        assert!(!results.is_empty());
    }

    #[tokio::test]
    async fn test_entity_embeddings() {
        let repo = setup_test_db().await;

        let entity = KnowledgeEntity::new("test", EntityType::Concept);
        repo.save_entity(&entity).await.unwrap();

        let embedding = vec![0.1, 0.2, 0.3, 0.4, 0.5];
        repo.save_entity_embedding(&entity.id, &embedding, "test-model")
            .await
            .unwrap();

        let retrieved = repo
            .get_entity_embedding(&entity.id, "test-model")
            .await
            .unwrap();
        assert!(retrieved.is_some());

        let retrieved = retrieved.unwrap();
        assert_eq!(retrieved.len(), 5);
        assert!((retrieved[0] - 0.1).abs() < 0.001);
    }

    #[tokio::test]
    async fn test_semantic_search() {
        let repo = setup_test_db().await;

        let e1 = KnowledgeEntity::new("similar", EntityType::Concept);
        let e2 = KnowledgeEntity::new("different", EntityType::Concept);

        repo.save_entity(&e1).await.unwrap();
        repo.save_entity(&e2).await.unwrap();

        // Similar embedding
        let emb1 = vec![1.0, 0.0, 0.0];
        // Different embedding (orthogonal)
        let emb2 = vec![0.0, 1.0, 0.0];

        repo.save_entity_embedding(&e1.id, &emb1, "test").await.unwrap();
        repo.save_entity_embedding(&e2.id, &emb2, "test").await.unwrap();

        // Query similar to e1
        let query = vec![0.9, 0.1, 0.0];
        let results = repo.semantic_search_entities(&query, "test", 10, 0.5).await.unwrap();

        assert!(!results.is_empty());
        assert_eq!(results[0].entity.id, e1.id);
    }

    #[tokio::test]
    async fn test_stats() {
        let repo = setup_test_db().await;

        let stats = repo.get_stats().await.unwrap();
        assert_eq!(stats.total_entities, 0);
        assert_eq!(stats.total_relationships, 0);

        let entity = KnowledgeEntity::new("test", EntityType::Concept);
        repo.save_entity(&entity).await.unwrap();

        let stats = repo.get_stats().await.unwrap();
        assert_eq!(stats.total_entities, 1);
    }

    #[test]
    fn test_cosine_similarity() {
        let a = vec![1.0, 0.0, 0.0];
        let b = vec![1.0, 0.0, 0.0];
        assert!((cosine_similarity(&a, &b) - 1.0).abs() < 0.001);

        let c = vec![0.0, 1.0, 0.0];
        assert!(cosine_similarity(&a, &c).abs() < 0.001);
    }

    #[tokio::test]
    async fn test_get_entity_by_canonical_name() {
        let repo = setup_test_db().await;

        let entity = KnowledgeEntity::new("Tokio Runtime", EntityType::Library)
            .with_description("Async runtime");

        repo.save_entity(&entity).await.unwrap();

        let retrieved = repo
            .get_entity_by_canonical_name(&entity.canonical_name)
            .await
            .unwrap();
        assert!(retrieved.is_some());
        assert_eq!(retrieved.unwrap().name, "Tokio Runtime");

        // Non-existent canonical name
        let not_found = repo
            .get_entity_by_canonical_name("nonexistent")
            .await
            .unwrap();
        assert!(not_found.is_none());
    }

    #[tokio::test]
    async fn test_list_entities() {
        let repo = setup_test_db().await;

        // Initially empty
        let entities = repo.list_entities().await.unwrap();
        assert!(entities.is_empty());

        // Add multiple entities
        let e1 = KnowledgeEntity::new("A", EntityType::Library).with_confidence(0.5);
        let e2 = KnowledgeEntity::new("B", EntityType::Concept).with_confidence(0.9);
        let e3 = KnowledgeEntity::new("C", EntityType::Framework).with_confidence(0.7);

        repo.save_entity(&e1).await.unwrap();
        repo.save_entity(&e2).await.unwrap();
        repo.save_entity(&e3).await.unwrap();

        let entities = repo.list_entities().await.unwrap();
        assert_eq!(entities.len(), 3);
        // Should be ordered by confidence DESC
        assert_eq!(entities[0].name, "B"); // 0.9
        assert_eq!(entities[1].name, "C"); // 0.7
        assert_eq!(entities[2].name, "A"); // 0.5
    }

    #[tokio::test]
    async fn test_list_entities_by_type() {
        let repo = setup_test_db().await;

        let lib1 = KnowledgeEntity::new("lib1", EntityType::Library);
        let lib2 = KnowledgeEntity::new("lib2", EntityType::Library);
        let concept = KnowledgeEntity::new("concept1", EntityType::Concept);

        repo.save_entity(&lib1).await.unwrap();
        repo.save_entity(&lib2).await.unwrap();
        repo.save_entity(&concept).await.unwrap();

        let libraries = repo.list_entities_by_type(EntityType::Library).await.unwrap();
        assert_eq!(libraries.len(), 2);

        let concepts = repo.list_entities_by_type(EntityType::Concept).await.unwrap();
        assert_eq!(concepts.len(), 1);

        let frameworks = repo.list_entities_by_type(EntityType::Framework).await.unwrap();
        assert!(frameworks.is_empty());
    }

    #[tokio::test]
    async fn test_delete_entity() {
        let repo = setup_test_db().await;

        let entity = KnowledgeEntity::new("to_delete", EntityType::Concept);
        repo.save_entity(&entity).await.unwrap();

        // Confirm exists
        assert!(repo.get_entity(&entity.id).await.unwrap().is_some());

        // Delete
        let deleted = repo.delete_entity(&entity.id).await.unwrap();
        assert!(deleted);

        // Confirm gone
        assert!(repo.get_entity(&entity.id).await.unwrap().is_none());

        // Deleting non-existent returns false
        let deleted_again = repo.delete_entity(&entity.id).await.unwrap();
        assert!(!deleted_again);
    }

    #[tokio::test]
    async fn test_count_entities() {
        let repo = setup_test_db().await;

        assert_eq!(repo.count_entities().await.unwrap(), 0);

        let e1 = KnowledgeEntity::new("E1", EntityType::Concept);
        let e2 = KnowledgeEntity::new("E2", EntityType::Concept);

        repo.save_entity(&e1).await.unwrap();
        assert_eq!(repo.count_entities().await.unwrap(), 1);

        repo.save_entity(&e2).await.unwrap();
        assert_eq!(repo.count_entities().await.unwrap(), 2);
    }

    #[tokio::test]
    async fn test_get_relationship_between() {
        let repo = setup_test_db().await;

        let e1 = KnowledgeEntity::new("E1", EntityType::Concept);
        let e2 = KnowledgeEntity::new("E2", EntityType::Concept);

        repo.save_entity(&e1).await.unwrap();
        repo.save_entity(&e2).await.unwrap();

        let rel = KnowledgeRelationship::new(&e1.id, &e2.id, RelationshipType::Uses);
        repo.save_relationship(&rel).await.unwrap();

        // Find existing relationship
        let found = repo
            .get_relationship_between(&e1.id, &e2.id, RelationshipType::Uses)
            .await
            .unwrap();
        assert!(found.is_some());
        assert_eq!(found.unwrap().id, rel.id);

        // Wrong type
        let not_found = repo
            .get_relationship_between(&e1.id, &e2.id, RelationshipType::Implements)
            .await
            .unwrap();
        assert!(not_found.is_none());

        // Wrong direction
        let not_found = repo
            .get_relationship_between(&e2.id, &e1.id, RelationshipType::Uses)
            .await
            .unwrap();
        assert!(not_found.is_none());
    }

    #[tokio::test]
    async fn test_list_relationships_for_entity() {
        let repo = setup_test_db().await;

        let e1 = KnowledgeEntity::new("E1", EntityType::Concept);
        let e2 = KnowledgeEntity::new("E2", EntityType::Concept);
        let e3 = KnowledgeEntity::new("E3", EntityType::Concept);

        repo.save_entity(&e1).await.unwrap();
        repo.save_entity(&e2).await.unwrap();
        repo.save_entity(&e3).await.unwrap();

        // E1 -> E2, E3 -> E1
        let rel1 = KnowledgeRelationship::new(&e1.id, &e2.id, RelationshipType::Uses);
        let rel2 = KnowledgeRelationship::new(&e3.id, &e1.id, RelationshipType::RelatedTo);

        repo.save_relationship(&rel1).await.unwrap();
        repo.save_relationship(&rel2).await.unwrap();

        // E1 should have both relationships (outgoing and incoming)
        let rels = repo.list_relationships_for_entity(&e1.id).await.unwrap();
        assert_eq!(rels.len(), 2);

        // E2 should have only one relationship
        let rels = repo.list_relationships_for_entity(&e2.id).await.unwrap();
        assert_eq!(rels.len(), 1);
    }

    #[tokio::test]
    async fn test_list_outgoing_incoming_relationships() {
        let repo = setup_test_db().await;

        let e1 = KnowledgeEntity::new("E1", EntityType::Concept);
        let e2 = KnowledgeEntity::new("E2", EntityType::Concept);
        let e3 = KnowledgeEntity::new("E3", EntityType::Concept);

        repo.save_entity(&e1).await.unwrap();
        repo.save_entity(&e2).await.unwrap();
        repo.save_entity(&e3).await.unwrap();

        // E1 -> E2, E3 -> E1
        let rel1 = KnowledgeRelationship::new(&e1.id, &e2.id, RelationshipType::Uses);
        let rel2 = KnowledgeRelationship::new(&e3.id, &e1.id, RelationshipType::RelatedTo);

        repo.save_relationship(&rel1).await.unwrap();
        repo.save_relationship(&rel2).await.unwrap();

        // E1 outgoing: E1 -> E2
        let outgoing = repo.list_outgoing_relationships(&e1.id).await.unwrap();
        assert_eq!(outgoing.len(), 1);
        assert_eq!(outgoing[0].target_entity_id, e2.id);

        // E1 incoming: E3 -> E1
        let incoming = repo.list_incoming_relationships(&e1.id).await.unwrap();
        assert_eq!(incoming.len(), 1);
        assert_eq!(incoming[0].source_entity_id, e3.id);
    }

    #[tokio::test]
    async fn test_delete_relationship() {
        let repo = setup_test_db().await;

        let e1 = KnowledgeEntity::new("E1", EntityType::Concept);
        let e2 = KnowledgeEntity::new("E2", EntityType::Concept);

        repo.save_entity(&e1).await.unwrap();
        repo.save_entity(&e2).await.unwrap();

        let rel = KnowledgeRelationship::new(&e1.id, &e2.id, RelationshipType::Uses);
        repo.save_relationship(&rel).await.unwrap();

        // Confirm exists
        assert!(repo.get_relationship(&rel.id).await.unwrap().is_some());

        // Delete
        let deleted = repo.delete_relationship(&rel.id).await.unwrap();
        assert!(deleted);

        // Confirm gone
        assert!(repo.get_relationship(&rel.id).await.unwrap().is_none());

        // Deleting non-existent returns false
        let deleted_again = repo.delete_relationship(&rel.id).await.unwrap();
        assert!(!deleted_again);
    }

    #[tokio::test]
    async fn test_count_relationships() {
        let repo = setup_test_db().await;

        assert_eq!(repo.count_relationships().await.unwrap(), 0);

        let e1 = KnowledgeEntity::new("E1", EntityType::Concept);
        let e2 = KnowledgeEntity::new("E2", EntityType::Concept);
        let e3 = KnowledgeEntity::new("E3", EntityType::Concept);

        repo.save_entity(&e1).await.unwrap();
        repo.save_entity(&e2).await.unwrap();
        repo.save_entity(&e3).await.unwrap();

        let rel1 = KnowledgeRelationship::new(&e1.id, &e2.id, RelationshipType::Uses);
        let rel2 = KnowledgeRelationship::new(&e2.id, &e3.id, RelationshipType::Uses);

        repo.save_relationship(&rel1).await.unwrap();
        assert_eq!(repo.count_relationships().await.unwrap(), 1);

        repo.save_relationship(&rel2).await.unwrap();
        assert_eq!(repo.count_relationships().await.unwrap(), 2);
    }

    #[tokio::test]
    async fn test_get_connected_entities() {
        let repo = setup_test_db().await;

        let e1 = KnowledgeEntity::new("E1", EntityType::Concept);
        let e2 = KnowledgeEntity::new("E2", EntityType::Concept);
        let e3 = KnowledgeEntity::new("E3", EntityType::Concept);

        repo.save_entity(&e1).await.unwrap();
        repo.save_entity(&e2).await.unwrap();
        repo.save_entity(&e3).await.unwrap();

        // E1 -> E2 (Uses), E3 -> E1 (Uses)
        let rel1 = KnowledgeRelationship::new(&e1.id, &e2.id, RelationshipType::Uses);
        let rel2 = KnowledgeRelationship::new(&e3.id, &e1.id, RelationshipType::Uses);

        repo.save_relationship(&rel1).await.unwrap();
        repo.save_relationship(&rel2).await.unwrap();

        // Outgoing from E1
        let connected = repo
            .get_connected_entities(&e1.id, RelationshipType::Uses, TraversalDirection::Outgoing)
            .await
            .unwrap();
        assert_eq!(connected.len(), 1);
        assert_eq!(connected[0].name, "E2");

        // Incoming to E1
        let connected = repo
            .get_connected_entities(&e1.id, RelationshipType::Uses, TraversalDirection::Incoming)
            .await
            .unwrap();
        assert_eq!(connected.len(), 1);
        assert_eq!(connected[0].name, "E3");

        // Both directions
        let connected = repo
            .get_connected_entities(&e1.id, RelationshipType::Uses, TraversalDirection::Both)
            .await
            .unwrap();
        assert_eq!(connected.len(), 2);
    }

    #[tokio::test]
    async fn test_link_and_unlink_skill_to_entity() {
        let repo = setup_test_db().await;

        let entity = KnowledgeEntity::new("test_entity", EntityType::Concept);
        repo.save_entity(&entity).await.unwrap();

        // Create skill first (foreign key requirement)
        create_test_skill(&repo, "skill-123").await;

        // Link skill to entity
        repo.link_skill_to_entity("skill-123", &entity.id, 0.8)
            .await
            .unwrap();

        // Verify link exists
        let skills = repo.get_skills_for_entity(&entity.id).await.unwrap();
        assert_eq!(skills.len(), 1);
        assert_eq!(skills[0], "skill-123");

        // Unlink
        let unlinked = repo
            .unlink_skill_from_entity("skill-123", &entity.id)
            .await
            .unwrap();
        assert!(unlinked);

        // Verify unlinked
        let skills = repo.get_skills_for_entity(&entity.id).await.unwrap();
        assert!(skills.is_empty());

        // Unlinking non-existent returns false
        let unlinked_again = repo
            .unlink_skill_from_entity("skill-123", &entity.id)
            .await
            .unwrap();
        assert!(!unlinked_again);
    }

    #[tokio::test]
    async fn test_update_entity_on_conflict() {
        let repo = setup_test_db().await;

        let mut entity = KnowledgeEntity::new("test", EntityType::Concept)
            .with_description("Original")
            .with_confidence(0.5);

        repo.save_entity(&entity).await.unwrap();

        // Update same entity (same ID)
        entity.description = Some("Updated".to_string());
        entity.confidence = 0.9;

        repo.save_entity(&entity).await.unwrap();

        let retrieved = repo.get_entity(&entity.id).await.unwrap().unwrap();
        assert_eq!(retrieved.description, Some("Updated".to_string()));
        assert_eq!(retrieved.confidence, 0.9);

        // Still only one entity
        assert_eq!(repo.count_entities().await.unwrap(), 1);
    }

    #[tokio::test]
    async fn test_update_relationship_on_conflict() {
        let repo = setup_test_db().await;

        let e1 = KnowledgeEntity::new("E1", EntityType::Concept);
        let e2 = KnowledgeEntity::new("E2", EntityType::Concept);

        repo.save_entity(&e1).await.unwrap();
        repo.save_entity(&e2).await.unwrap();

        let mut rel = KnowledgeRelationship::new(&e1.id, &e2.id, RelationshipType::Uses)
            .with_weight(0.5);

        repo.save_relationship(&rel).await.unwrap();

        // Update same relationship (same source, target, type)
        rel.weight = 0.9;
        repo.save_relationship(&rel).await.unwrap();

        let retrieved = repo
            .get_relationship_between(&e1.id, &e2.id, RelationshipType::Uses)
            .await
            .unwrap()
            .unwrap();
        assert_eq!(retrieved.weight, 0.9);

        // Still only one relationship
        assert_eq!(repo.count_relationships().await.unwrap(), 1);
    }

    #[tokio::test]
    async fn test_entity_with_aliases_and_source_skills() {
        let repo = setup_test_db().await;

        let mut entity = KnowledgeEntity::new("tokio", EntityType::Library);
        entity.aliases = vec!["tokio-rs".to_string(), "tokio runtime".to_string()];
        entity.source_skill_ids = vec!["skill-1".to_string(), "skill-2".to_string()];

        repo.save_entity(&entity).await.unwrap();

        let retrieved = repo.get_entity(&entity.id).await.unwrap().unwrap();
        assert_eq!(retrieved.aliases.len(), 2);
        assert!(retrieved.aliases.contains(&"tokio-rs".to_string()));
        assert_eq!(retrieved.source_skill_ids.len(), 2);
    }

    #[tokio::test]
    async fn test_relationship_with_evidence() {
        let repo = setup_test_db().await;

        let e1 = KnowledgeEntity::new("E1", EntityType::Concept);
        let e2 = KnowledgeEntity::new("E2", EntityType::Concept);

        repo.save_entity(&e1).await.unwrap();
        repo.save_entity(&e2).await.unwrap();

        use crate::domain::knowledge::{EvidenceSource, RelationshipEvidence};
        let mut rel = KnowledgeRelationship::new(&e1.id, &e2.id, RelationshipType::Uses);
        rel.evidence = vec![
            RelationshipEvidence {
                source: EvidenceSource::UserInput,
                description: "Found in documentation".to_string(),
                confidence: 0.9,
                timestamp: Utc::now(),
            },
            RelationshipEvidence {
                source: EvidenceSource::UsagePattern,
                description: "Used in examples".to_string(),
                confidence: 0.8,
                timestamp: Utc::now(),
            },
        ];

        repo.save_relationship(&rel).await.unwrap();

        let retrieved = repo.get_relationship(&rel.id).await.unwrap().unwrap();
        assert_eq!(retrieved.evidence.len(), 2);
        assert!(retrieved.evidence.iter().any(|e| e.description == "Found in documentation"));
    }

    #[tokio::test]
    async fn test_get_neighborhood_with_relationship_filter() {
        let repo = setup_test_db().await;

        let a = KnowledgeEntity::new("A", EntityType::Concept);
        let b = KnowledgeEntity::new("B", EntityType::Concept);
        let c = KnowledgeEntity::new("C", EntityType::Concept);

        repo.save_entity(&a).await.unwrap();
        repo.save_entity(&b).await.unwrap();
        repo.save_entity(&c).await.unwrap();

        // A -> B (Uses), A -> C (Implements)
        let rel1 = KnowledgeRelationship::new(&a.id, &b.id, RelationshipType::Uses);
        let rel2 = KnowledgeRelationship::new(&a.id, &c.id, RelationshipType::Implements);

        repo.save_relationship(&rel1).await.unwrap();
        repo.save_relationship(&rel2).await.unwrap();

        // No filter: both B and C reachable
        let neighbors = repo.get_neighborhood(&a.id, 1, None).await.unwrap();
        assert_eq!(neighbors.len(), 2);

        // Filter by Uses: only B reachable
        let neighbors = repo
            .get_neighborhood(&a.id, 1, Some(&[RelationshipType::Uses]))
            .await
            .unwrap();
        assert_eq!(neighbors.len(), 1);
        assert_eq!(neighbors[0].entity.name, "B");
    }

    #[tokio::test]
    async fn test_stats_comprehensive() {
        let repo = setup_test_db().await;

        // Create entities of different types
        let lib = KnowledgeEntity::new("lib", EntityType::Library).with_confidence(0.8);
        let concept = KnowledgeEntity::new("concept", EntityType::Concept).with_confidence(0.6);

        repo.save_entity(&lib).await.unwrap();
        repo.save_entity(&concept).await.unwrap();

        // Create relationships
        let rel = KnowledgeRelationship::new(&lib.id, &concept.id, RelationshipType::Uses)
            .with_weight(0.7);
        repo.save_relationship(&rel).await.unwrap();

        // Create skill first (foreign key requirement)
        create_test_skill(&repo, "skill-1").await;

        // Link skill
        repo.link_skill_to_entity("skill-1", &lib.id, 0.9)
            .await
            .unwrap();

        // Add embedding
        repo.save_entity_embedding(&lib.id, &[0.1, 0.2, 0.3], "test-model")
            .await
            .unwrap();

        let stats = repo.get_stats().await.unwrap();

        assert_eq!(stats.total_entities, 2);
        assert_eq!(stats.total_relationships, 1);
        assert_eq!(stats.total_skill_links, 1);
        assert_eq!(stats.entities_with_embeddings, 1);
        assert!((stats.average_entity_confidence - 0.7).abs() < 0.01); // (0.8 + 0.6) / 2
        assert!((stats.average_relationship_weight - 0.7).abs() < 0.01);
        assert!(stats.entities_by_type.iter().any(|(t, _)| *t == EntityType::Library));
        assert!(stats.entities_by_type.iter().any(|(t, _)| *t == EntityType::Concept));
        assert!(stats.relationships_by_type.iter().any(|(t, _)| *t == RelationshipType::Uses));
    }
}
