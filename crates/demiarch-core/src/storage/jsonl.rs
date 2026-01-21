//! JSONL export/import for git-friendly sync
//!
//! This module provides JSONL (JSON Lines) export and import functionality for
//! synchronizing SQLite database contents with git repositories. The format is
//! designed to be:
//!
//! - **Git-friendly**: One record per line for clean diffs
//! - **Incremental**: Only changed records are modified
//! - **Deterministic**: Sorted output for consistent diffs
//! - **Self-describing**: Each line includes table name and record type
//!
//! # File Structure
//!
//! Exports are organized into separate `.jsonl` files per table:
//! ```text
//! .demiarch/sync/
//! ├── projects.jsonl
//! ├── features.jsonl
//! ├── phases.jsonl
//! ├── conversations.jsonl
//! ├── messages.jsonl
//! ├── checkpoints.jsonl
//! ├── generated_files.jsonl
//! ├── documents.jsonl
//! ├── document_versions.jsonl
//! ├── llm_costs.jsonl
//! ├── daily_cost_summaries.jsonl
//! └── feature_extraction_history.jsonl
//! ```
//!
//! Note: `encrypted_keys` and `phase_templates` are NOT exported for security
//! and because phase_templates are built-in defaults.

use base64::Engine;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::{FromRow, SqlitePool};
use std::collections::HashMap;
use std::fs::{self, File};
use std::io::{BufRead, BufReader, BufWriter, Write};
use std::path::{Path, PathBuf};

use crate::error::Error;
use crate::Result;

/// Default sync directory name within project
pub const SYNC_DIR: &str = ".demiarch/sync";

/// Tables that are exported to JSONL (in dependency order for import)
pub const EXPORTABLE_TABLES: &[&str] = &[
    "projects",
    "phases",
    "features",
    "conversations",
    "messages",
    "checkpoints",
    "generated_files",
    "documents",
    "document_versions",
    "llm_costs",
    "daily_cost_summaries",
    "feature_extraction_history",
    "learned_skills",
];

// =============================================================================
// Record Types - One struct per table for type-safe export/import
// =============================================================================

/// Project record for JSONL export
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct ProjectRecord {
    pub id: String,
    pub name: String,
    pub framework: String,
    pub repo_url: String,
    pub status: String,
    pub description: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

/// Phase record for JSONL export
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct PhaseRecord {
    pub id: String,
    pub project_id: String,
    pub name: String,
    pub description: Option<String>,
    pub order_index: i32,
    pub status: String,
    pub created_at: String,
    pub updated_at: String,
}

/// Feature record for JSONL export
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct FeatureRecord {
    pub id: String,
    pub project_id: String,
    pub title: String,
    pub description: Option<String>,
    pub phase_id: Option<String>,
    pub status: String,
    pub priority: i32,
    pub acceptance_criteria: Option<String>,
    pub labels: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

/// Conversation record for JSONL export
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct ConversationRecord {
    pub id: String,
    pub project_id: String,
    pub title: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

/// Message record for JSONL export
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct MessageRecord {
    pub id: String,
    pub conversation_id: String,
    pub role: String,
    pub content: String,
    pub model: Option<String>,
    pub tokens_used: Option<i32>,
    pub created_at: String,
}

/// Checkpoint record for JSONL export
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CheckpointRecord {
    pub id: String,
    pub project_id: String,
    pub feature_id: Option<String>,
    pub description: String,
    pub snapshot_data: String,
    pub size_bytes: i64,
    /// Base64-encoded signature
    pub signature: String,
    pub created_at: String,
}

/// Raw checkpoint row from database (with BLOB signature)
#[derive(Debug, Clone, FromRow)]
struct CheckpointRow {
    id: String,
    project_id: String,
    feature_id: Option<String>,
    description: String,
    snapshot_data: String,
    size_bytes: i64,
    signature: Vec<u8>,
    created_at: String,
}

/// Generated file record for JSONL export
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GeneratedFileRecord {
    pub id: String,
    pub project_id: String,
    pub feature_id: Option<String>,
    pub file_path: String,
    pub content_hash: String,
    pub generation_timestamp: String,
    pub last_verified_hash: Option<String>,
    pub last_verified_at: Option<String>,
    pub edit_detected: bool,
}

/// Raw generated file row from database (with integer edit_detected)
#[derive(Debug, Clone, FromRow)]
struct GeneratedFileRow {
    id: String,
    project_id: String,
    feature_id: Option<String>,
    file_path: String,
    content_hash: String,
    generation_timestamp: String,
    last_verified_hash: Option<String>,
    last_verified_at: Option<String>,
    edit_detected: i32,
}

/// Document record for JSONL export
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct DocumentRecord {
    pub id: String,
    pub project_id: String,
    pub doc_type: String,
    pub title: String,
    pub description: Option<String>,
    pub content: String,
    pub format: String,
    pub version: i32,
    pub status: String,
    pub model_used: Option<String>,
    pub tokens_used: Option<i32>,
    pub generation_cost_usd: Option<f64>,
    pub created_at: String,
    pub updated_at: String,
}

/// Document version record for JSONL export
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct DocumentVersionRecord {
    pub id: String,
    pub document_id: String,
    pub version_number: i32,
    pub content: String,
    pub change_summary: Option<String>,
    pub model_used: Option<String>,
    pub created_at: String,
}

/// LLM cost record for JSONL export
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct LlmCostRecord {
    pub id: String,
    pub project_id: Option<String>,
    pub model: String,
    pub input_tokens: i32,
    pub output_tokens: i32,
    pub input_cost_usd: f64,
    pub output_cost_usd: f64,
    pub context: Option<String>,
    pub created_at: String,
}

/// Daily cost summary record for JSONL export
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct DailyCostSummaryRecord {
    pub date: String,
    pub project_id: String,
    pub model: String,
    pub total_cost_usd: f64,
    pub total_input_tokens: i32,
    pub total_output_tokens: i32,
    pub call_count: i32,
    pub updated_at: String,
}

/// Feature extraction history record for JSONL export
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct FeatureExtractionHistoryRecord {
    pub id: String,
    pub project_id: String,
    pub conversation_id: Option<String>,
    pub model_used: String,
    pub tokens_used: Option<i32>,
    pub cost_usd: Option<f64>,
    pub phases_created: i32,
    pub features_created: i32,
    pub raw_response: Option<String>,
    pub created_at: String,
}

/// Learned skill record for JSONL export
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct LearnedSkillRecord {
    pub id: String,
    pub name: String,
    pub description: String,
    pub category: String,
    pub pattern_type: String,
    pub pattern_template: String,
    pub pattern_variables: Option<String>,
    pub pattern_applicability: Option<String>,
    pub pattern_limitations: Option<String>,
    pub source_project_id: Option<String>,
    pub source_feature_id: Option<String>,
    pub source_agent_type: Option<String>,
    pub source_original_task: Option<String>,
    pub source_model_used: Option<String>,
    pub source_tokens_used: Option<i32>,
    pub confidence: String,
    pub tags: Option<String>,
    pub times_used: i32,
    pub success_count: i32,
    pub failure_count: i32,
    pub last_used_at: Option<String>,
    pub metadata: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

// =============================================================================
// Export Status and Metadata
// =============================================================================

/// Status of a sync operation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SyncMetadata {
    /// When the export was created
    pub exported_at: DateTime<Utc>,
    /// Schema version of the database
    pub schema_version: i32,
    /// Number of records per table
    pub record_counts: HashMap<String, usize>,
    /// Total records exported
    pub total_records: usize,
}

/// Result of an export operation
#[derive(Debug, Clone)]
pub struct ExportResult {
    /// Path to the sync directory
    pub sync_dir: PathBuf,
    /// Metadata about the export
    pub metadata: SyncMetadata,
    /// Files that were written
    pub files_written: Vec<PathBuf>,
}

/// Result of an import operation
#[derive(Debug, Clone)]
pub struct ImportResult {
    /// Number of records imported per table
    pub record_counts: HashMap<String, usize>,
    /// Total records imported
    pub total_records: usize,
    /// Any warnings during import
    pub warnings: Vec<String>,
}

// =============================================================================
// Export Functions
// =============================================================================

/// Export all database tables to JSONL files in the sync directory
///
/// # Arguments
/// * `pool` - SQLite connection pool
/// * `project_dir` - Root directory of the project
///
/// # Returns
/// Export result with metadata and file paths
pub async fn export_to_jsonl(pool: &SqlitePool, project_dir: &Path) -> Result<ExportResult> {
    let sync_dir = project_dir.join(SYNC_DIR);

    // Create sync directory if it doesn't exist
    fs::create_dir_all(&sync_dir).map_err(Error::Io)?;

    let mut record_counts = HashMap::new();
    let mut files_written = Vec::new();
    let mut total_records = 0usize;

    // Export each table
    for table in EXPORTABLE_TABLES {
        let file_path = sync_dir.join(format!("{}.jsonl", table));
        let count = export_table(pool, table, &file_path).await?;
        record_counts.insert(table.to_string(), count);
        total_records += count;
        files_written.push(file_path);
    }

    // Write metadata file
    let metadata = SyncMetadata {
        exported_at: Utc::now(),
        schema_version: crate::storage::CURRENT_VERSION,
        record_counts: record_counts.clone(),
        total_records,
    };

    let metadata_path = sync_dir.join("_metadata.json");
    let metadata_file = File::create(&metadata_path).map_err(Error::Io)?;
    serde_json::to_writer_pretty(metadata_file, &metadata)
        .map_err(|e| Error::Other(format!("Failed to write metadata: {}", e)))?;
    files_written.push(metadata_path);

    Ok(ExportResult {
        sync_dir,
        metadata,
        files_written,
    })
}

/// Export a single table to a JSONL file
async fn export_table(pool: &SqlitePool, table: &str, file_path: &Path) -> Result<usize> {
    let file = File::create(file_path).map_err(Error::Io)?;
    let mut writer = BufWriter::new(file);

    let count = match table {
        "projects" => export_projects(pool, &mut writer).await?,
        "phases" => export_phases(pool, &mut writer).await?,
        "features" => export_features(pool, &mut writer).await?,
        "conversations" => export_conversations(pool, &mut writer).await?,
        "messages" => export_messages(pool, &mut writer).await?,
        "checkpoints" => export_checkpoints(pool, &mut writer).await?,
        "generated_files" => export_generated_files(pool, &mut writer).await?,
        "documents" => export_documents(pool, &mut writer).await?,
        "document_versions" => export_document_versions(pool, &mut writer).await?,
        "llm_costs" => export_llm_costs(pool, &mut writer).await?,
        "daily_cost_summaries" => export_daily_cost_summaries(pool, &mut writer).await?,
        "feature_extraction_history" => export_feature_extraction_history(pool, &mut writer).await?,
        "learned_skills" => export_learned_skills(pool, &mut writer).await?,
        _ => return Err(Error::Other(format!("Unknown table: {}", table))),
    };

    writer.flush().map_err(Error::Io)?;
    Ok(count)
}

async fn export_projects<W: Write>(pool: &SqlitePool, writer: &mut W) -> Result<usize> {
    let rows: Vec<ProjectRecord> = sqlx::query_as(
        r#"
        SELECT id, name, framework, repo_url, status, description,
               created_at, updated_at
        FROM projects
        ORDER BY id
        "#,
    )
    .fetch_all(pool)
    .await?;

    let count = rows.len();
    for record in rows {
        serde_json::to_writer(&mut *writer, &record)
            .map_err(|e| Error::Other(format!("JSON serialization error: {}", e)))?;
        writeln!(writer).map_err(Error::Io)?;
    }
    Ok(count)
}

async fn export_phases<W: Write>(pool: &SqlitePool, writer: &mut W) -> Result<usize> {
    let rows: Vec<PhaseRecord> = sqlx::query_as(
        r#"
        SELECT id, project_id, name, description, order_index, status,
               created_at, updated_at
        FROM phases
        ORDER BY project_id, order_index, id
        "#,
    )
    .fetch_all(pool)
    .await?;

    let count = rows.len();
    for record in rows {
        serde_json::to_writer(&mut *writer, &record)
            .map_err(|e| Error::Other(format!("JSON serialization error: {}", e)))?;
        writeln!(writer).map_err(Error::Io)?;
    }
    Ok(count)
}

async fn export_features<W: Write>(pool: &SqlitePool, writer: &mut W) -> Result<usize> {
    let rows: Vec<FeatureRecord> = sqlx::query_as(
        r#"
        SELECT id, project_id, title, description, phase_id, status, priority,
               acceptance_criteria, labels, created_at, updated_at
        FROM features
        ORDER BY project_id, phase_id, priority DESC, id
        "#,
    )
    .fetch_all(pool)
    .await?;

    let count = rows.len();
    for record in rows {
        serde_json::to_writer(&mut *writer, &record)
            .map_err(|e| Error::Other(format!("JSON serialization error: {}", e)))?;
        writeln!(writer).map_err(Error::Io)?;
    }
    Ok(count)
}

async fn export_conversations<W: Write>(pool: &SqlitePool, writer: &mut W) -> Result<usize> {
    let rows: Vec<ConversationRecord> = sqlx::query_as(
        r#"
        SELECT id, project_id, title, created_at, updated_at
        FROM conversations
        ORDER BY project_id, created_at, id
        "#,
    )
    .fetch_all(pool)
    .await?;

    let count = rows.len();
    for record in rows {
        serde_json::to_writer(&mut *writer, &record)
            .map_err(|e| Error::Other(format!("JSON serialization error: {}", e)))?;
        writeln!(writer).map_err(Error::Io)?;
    }
    Ok(count)
}

async fn export_messages<W: Write>(pool: &SqlitePool, writer: &mut W) -> Result<usize> {
    let rows: Vec<MessageRecord> = sqlx::query_as(
        r#"
        SELECT id, conversation_id, role, content, model, tokens_used, created_at
        FROM messages
        ORDER BY conversation_id, created_at, id
        "#,
    )
    .fetch_all(pool)
    .await?;

    let count = rows.len();
    for record in rows {
        serde_json::to_writer(&mut *writer, &record)
            .map_err(|e| Error::Other(format!("JSON serialization error: {}", e)))?;
        writeln!(writer).map_err(Error::Io)?;
    }
    Ok(count)
}

async fn export_checkpoints<W: Write>(pool: &SqlitePool, writer: &mut W) -> Result<usize> {
    let rows: Vec<CheckpointRow> = sqlx::query_as(
        r#"
        SELECT id, project_id, feature_id, description, snapshot_data,
               size_bytes, signature, created_at
        FROM checkpoints
        ORDER BY project_id, created_at, id
        "#,
    )
    .fetch_all(pool)
    .await?;

    let count = rows.len();
    for row in rows {
        let record = CheckpointRecord {
            id: row.id,
            project_id: row.project_id,
            feature_id: row.feature_id,
            description: row.description,
            snapshot_data: row.snapshot_data,
            size_bytes: row.size_bytes,
            signature: base64::engine::general_purpose::STANDARD.encode(&row.signature),
            created_at: row.created_at,
        };
        serde_json::to_writer(&mut *writer, &record)
            .map_err(|e| Error::Other(format!("JSON serialization error: {}", e)))?;
        writeln!(writer).map_err(Error::Io)?;
    }
    Ok(count)
}

async fn export_generated_files<W: Write>(pool: &SqlitePool, writer: &mut W) -> Result<usize> {
    let rows: Vec<GeneratedFileRow> = sqlx::query_as(
        r#"
        SELECT id, project_id, feature_id, file_path, content_hash,
               generation_timestamp, last_verified_hash, last_verified_at, edit_detected
        FROM generated_files
        ORDER BY project_id, file_path, id
        "#,
    )
    .fetch_all(pool)
    .await?;

    let count = rows.len();
    for row in rows {
        let record = GeneratedFileRecord {
            id: row.id,
            project_id: row.project_id,
            feature_id: row.feature_id,
            file_path: row.file_path,
            content_hash: row.content_hash,
            generation_timestamp: row.generation_timestamp,
            last_verified_hash: row.last_verified_hash,
            last_verified_at: row.last_verified_at,
            edit_detected: row.edit_detected != 0,
        };
        serde_json::to_writer(&mut *writer, &record)
            .map_err(|e| Error::Other(format!("JSON serialization error: {}", e)))?;
        writeln!(writer).map_err(Error::Io)?;
    }
    Ok(count)
}

async fn export_documents<W: Write>(pool: &SqlitePool, writer: &mut W) -> Result<usize> {
    let rows: Vec<DocumentRecord> = sqlx::query_as(
        r#"
        SELECT id, project_id, doc_type, title, description, content, format,
               version, status, model_used, tokens_used, generation_cost_usd,
               created_at, updated_at
        FROM documents
        ORDER BY project_id, doc_type, id
        "#,
    )
    .fetch_all(pool)
    .await?;

    let count = rows.len();
    for record in rows {
        serde_json::to_writer(&mut *writer, &record)
            .map_err(|e| Error::Other(format!("JSON serialization error: {}", e)))?;
        writeln!(writer).map_err(Error::Io)?;
    }
    Ok(count)
}

async fn export_document_versions<W: Write>(pool: &SqlitePool, writer: &mut W) -> Result<usize> {
    let rows: Vec<DocumentVersionRecord> = sqlx::query_as(
        r#"
        SELECT id, document_id, version_number, content, change_summary,
               model_used, created_at
        FROM document_versions
        ORDER BY document_id, version_number, id
        "#,
    )
    .fetch_all(pool)
    .await?;

    let count = rows.len();
    for record in rows {
        serde_json::to_writer(&mut *writer, &record)
            .map_err(|e| Error::Other(format!("JSON serialization error: {}", e)))?;
        writeln!(writer).map_err(Error::Io)?;
    }
    Ok(count)
}

async fn export_llm_costs<W: Write>(pool: &SqlitePool, writer: &mut W) -> Result<usize> {
    let rows: Vec<LlmCostRecord> = sqlx::query_as(
        r#"
        SELECT id, project_id, model, input_tokens, output_tokens,
               input_cost_usd, output_cost_usd, context, created_at
        FROM llm_costs
        ORDER BY created_at, id
        "#,
    )
    .fetch_all(pool)
    .await?;

    let count = rows.len();
    for record in rows {
        serde_json::to_writer(&mut *writer, &record)
            .map_err(|e| Error::Other(format!("JSON serialization error: {}", e)))?;
        writeln!(writer).map_err(Error::Io)?;
    }
    Ok(count)
}

async fn export_daily_cost_summaries<W: Write>(pool: &SqlitePool, writer: &mut W) -> Result<usize> {
    let rows: Vec<DailyCostSummaryRecord> = sqlx::query_as(
        r#"
        SELECT date, project_id, model, total_cost_usd, total_input_tokens,
               total_output_tokens, call_count, updated_at
        FROM daily_cost_summaries
        ORDER BY date, project_id, model
        "#,
    )
    .fetch_all(pool)
    .await?;

    let count = rows.len();
    for record in rows {
        serde_json::to_writer(&mut *writer, &record)
            .map_err(|e| Error::Other(format!("JSON serialization error: {}", e)))?;
        writeln!(writer).map_err(Error::Io)?;
    }
    Ok(count)
}

async fn export_feature_extraction_history<W: Write>(
    pool: &SqlitePool,
    writer: &mut W,
) -> Result<usize> {
    let rows: Vec<FeatureExtractionHistoryRecord> = sqlx::query_as(
        r#"
        SELECT id, project_id, conversation_id, model_used, tokens_used,
               cost_usd, phases_created, features_created, raw_response, created_at
        FROM feature_extraction_history
        ORDER BY project_id, created_at, id
        "#,
    )
    .fetch_all(pool)
    .await?;

    let count = rows.len();
    for record in rows {
        serde_json::to_writer(&mut *writer, &record)
            .map_err(|e| Error::Other(format!("JSON serialization error: {}", e)))?;
        writeln!(writer).map_err(Error::Io)?;
    }
    Ok(count)
}

async fn export_learned_skills<W: Write>(pool: &SqlitePool, writer: &mut W) -> Result<usize> {
    let rows: Vec<LearnedSkillRecord> = sqlx::query_as(
        r#"
        SELECT id, name, description, category,
               pattern_type, pattern_template, pattern_variables,
               pattern_applicability, pattern_limitations,
               source_project_id, source_feature_id, source_agent_type,
               source_original_task, source_model_used, source_tokens_used,
               confidence, tags,
               times_used, success_count, failure_count, last_used_at,
               metadata, created_at, updated_at
        FROM learned_skills
        ORDER BY created_at, id
        "#,
    )
    .fetch_all(pool)
    .await?;

    let count = rows.len();
    for record in rows {
        serde_json::to_writer(&mut *writer, &record)
            .map_err(|e| Error::Other(format!("JSON serialization error: {}", e)))?;
        writeln!(writer).map_err(Error::Io)?;
    }
    Ok(count)
}

// =============================================================================
// Import Functions
// =============================================================================

/// Import JSONL files from the sync directory into the database
///
/// # Arguments
/// * `pool` - SQLite connection pool
/// * `project_dir` - Root directory of the project
///
/// # Returns
/// Import result with counts and any warnings
pub async fn import_from_jsonl(pool: &SqlitePool, project_dir: &Path) -> Result<ImportResult> {
    let sync_dir = project_dir.join(SYNC_DIR);

    if !sync_dir.exists() {
        return Err(Error::NotFound(format!(
            "Sync directory not found: {}",
            sync_dir.display()
        )));
    }

    let mut record_counts = HashMap::new();
    let mut total_records = 0usize;
    let mut warnings = Vec::new();

    // Import tables in dependency order (parents before children)
    for table in EXPORTABLE_TABLES {
        let file_path = sync_dir.join(format!("{}.jsonl", table));
        if file_path.exists() {
            match import_table(pool, table, &file_path).await {
                Ok(count) => {
                    record_counts.insert(table.to_string(), count);
                    total_records += count;
                }
                Err(e) => {
                    warnings.push(format!("Failed to import {}: {}", table, e));
                }
            }
        } else {
            warnings.push(format!("File not found: {}", file_path.display()));
        }
    }

    Ok(ImportResult {
        record_counts,
        total_records,
        warnings,
    })
}

/// Import a single table from a JSONL file
async fn import_table(pool: &SqlitePool, table: &str, file_path: &Path) -> Result<usize> {
    let file = File::open(file_path).map_err(Error::Io)?;
    let reader = BufReader::new(file);

    match table {
        "projects" => import_projects(pool, reader).await,
        "phases" => import_phases(pool, reader).await,
        "features" => import_features(pool, reader).await,
        "conversations" => import_conversations(pool, reader).await,
        "messages" => import_messages(pool, reader).await,
        "checkpoints" => import_checkpoints(pool, reader).await,
        "generated_files" => import_generated_files(pool, reader).await,
        "documents" => import_documents(pool, reader).await,
        "document_versions" => import_document_versions(pool, reader).await,
        "llm_costs" => import_llm_costs(pool, reader).await,
        "daily_cost_summaries" => import_daily_cost_summaries(pool, reader).await,
        "feature_extraction_history" => import_feature_extraction_history(pool, reader).await,
        "learned_skills" => import_learned_skills(pool, reader).await,
        _ => Err(Error::Other(format!("Unknown table: {}", table))),
    }
}

async fn import_projects<R: BufRead>(pool: &SqlitePool, reader: R) -> Result<usize> {
    let mut count = 0;
    for line in reader.lines() {
        let line = line.map_err(Error::Io)?;
        if line.trim().is_empty() {
            continue;
        }
        let record: ProjectRecord =
            serde_json::from_str(&line).map_err(|e| Error::Parse(format!("Invalid JSON: {}", e)))?;

        sqlx::query(
            r#"
            INSERT OR REPLACE INTO projects
            (id, name, framework, repo_url, status, description, created_at, updated_at)
            VALUES (?, ?, ?, ?, ?, ?, ?, ?)
            "#,
        )
        .bind(&record.id)
        .bind(&record.name)
        .bind(&record.framework)
        .bind(&record.repo_url)
        .bind(&record.status)
        .bind(&record.description)
        .bind(&record.created_at)
        .bind(&record.updated_at)
        .execute(pool)
        .await?;

        count += 1;
    }
    Ok(count)
}

async fn import_phases<R: BufRead>(pool: &SqlitePool, reader: R) -> Result<usize> {
    let mut count = 0;
    for line in reader.lines() {
        let line = line.map_err(Error::Io)?;
        if line.trim().is_empty() {
            continue;
        }
        let record: PhaseRecord =
            serde_json::from_str(&line).map_err(|e| Error::Parse(format!("Invalid JSON: {}", e)))?;

        sqlx::query(
            r#"
            INSERT OR REPLACE INTO phases
            (id, project_id, name, description, order_index, status, created_at, updated_at)
            VALUES (?, ?, ?, ?, ?, ?, ?, ?)
            "#,
        )
        .bind(&record.id)
        .bind(&record.project_id)
        .bind(&record.name)
        .bind(&record.description)
        .bind(record.order_index)
        .bind(&record.status)
        .bind(&record.created_at)
        .bind(&record.updated_at)
        .execute(pool)
        .await?;

        count += 1;
    }
    Ok(count)
}

async fn import_features<R: BufRead>(pool: &SqlitePool, reader: R) -> Result<usize> {
    let mut count = 0;
    for line in reader.lines() {
        let line = line.map_err(Error::Io)?;
        if line.trim().is_empty() {
            continue;
        }
        let record: FeatureRecord =
            serde_json::from_str(&line).map_err(|e| Error::Parse(format!("Invalid JSON: {}", e)))?;

        sqlx::query(
            r#"
            INSERT OR REPLACE INTO features
            (id, project_id, title, description, phase_id, status, priority,
             acceptance_criteria, labels, created_at, updated_at)
            VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
            "#,
        )
        .bind(&record.id)
        .bind(&record.project_id)
        .bind(&record.title)
        .bind(&record.description)
        .bind(&record.phase_id)
        .bind(&record.status)
        .bind(record.priority)
        .bind(&record.acceptance_criteria)
        .bind(&record.labels)
        .bind(&record.created_at)
        .bind(&record.updated_at)
        .execute(pool)
        .await?;

        count += 1;
    }
    Ok(count)
}

async fn import_conversations<R: BufRead>(pool: &SqlitePool, reader: R) -> Result<usize> {
    let mut count = 0;
    for line in reader.lines() {
        let line = line.map_err(Error::Io)?;
        if line.trim().is_empty() {
            continue;
        }
        let record: ConversationRecord =
            serde_json::from_str(&line).map_err(|e| Error::Parse(format!("Invalid JSON: {}", e)))?;

        sqlx::query(
            r#"
            INSERT OR REPLACE INTO conversations
            (id, project_id, title, created_at, updated_at)
            VALUES (?, ?, ?, ?, ?)
            "#,
        )
        .bind(&record.id)
        .bind(&record.project_id)
        .bind(&record.title)
        .bind(&record.created_at)
        .bind(&record.updated_at)
        .execute(pool)
        .await?;

        count += 1;
    }
    Ok(count)
}

async fn import_messages<R: BufRead>(pool: &SqlitePool, reader: R) -> Result<usize> {
    let mut count = 0;
    for line in reader.lines() {
        let line = line.map_err(Error::Io)?;
        if line.trim().is_empty() {
            continue;
        }
        let record: MessageRecord =
            serde_json::from_str(&line).map_err(|e| Error::Parse(format!("Invalid JSON: {}", e)))?;

        sqlx::query(
            r#"
            INSERT OR REPLACE INTO messages
            (id, conversation_id, role, content, model, tokens_used, created_at)
            VALUES (?, ?, ?, ?, ?, ?, ?)
            "#,
        )
        .bind(&record.id)
        .bind(&record.conversation_id)
        .bind(&record.role)
        .bind(&record.content)
        .bind(&record.model)
        .bind(record.tokens_used)
        .bind(&record.created_at)
        .execute(pool)
        .await?;

        count += 1;
    }
    Ok(count)
}

async fn import_checkpoints<R: BufRead>(pool: &SqlitePool, reader: R) -> Result<usize> {
    let mut count = 0;
    for line in reader.lines() {
        let line = line.map_err(Error::Io)?;
        if line.trim().is_empty() {
            continue;
        }
        let record: CheckpointRecord =
            serde_json::from_str(&line).map_err(|e| Error::Parse(format!("Invalid JSON: {}", e)))?;

        // Decode base64 signature back to bytes
        let signature = base64::engine::general_purpose::STANDARD
            .decode(&record.signature)
            .map_err(|e| Error::Parse(format!("Invalid base64 signature: {}", e)))?;

        sqlx::query(
            r#"
            INSERT OR REPLACE INTO checkpoints
            (id, project_id, feature_id, description, snapshot_data, size_bytes, signature, created_at)
            VALUES (?, ?, ?, ?, ?, ?, ?, ?)
            "#,
        )
        .bind(&record.id)
        .bind(&record.project_id)
        .bind(&record.feature_id)
        .bind(&record.description)
        .bind(&record.snapshot_data)
        .bind(record.size_bytes)
        .bind(&signature)
        .bind(&record.created_at)
        .execute(pool)
        .await?;

        count += 1;
    }
    Ok(count)
}

async fn import_generated_files<R: BufRead>(pool: &SqlitePool, reader: R) -> Result<usize> {
    let mut count = 0;
    for line in reader.lines() {
        let line = line.map_err(Error::Io)?;
        if line.trim().is_empty() {
            continue;
        }
        let record: GeneratedFileRecord =
            serde_json::from_str(&line).map_err(|e| Error::Parse(format!("Invalid JSON: {}", e)))?;

        let edit_detected: i32 = if record.edit_detected { 1 } else { 0 };

        sqlx::query(
            r#"
            INSERT OR REPLACE INTO generated_files
            (id, project_id, feature_id, file_path, content_hash, generation_timestamp,
             last_verified_hash, last_verified_at, edit_detected)
            VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?)
            "#,
        )
        .bind(&record.id)
        .bind(&record.project_id)
        .bind(&record.feature_id)
        .bind(&record.file_path)
        .bind(&record.content_hash)
        .bind(&record.generation_timestamp)
        .bind(&record.last_verified_hash)
        .bind(&record.last_verified_at)
        .bind(edit_detected)
        .execute(pool)
        .await?;

        count += 1;
    }
    Ok(count)
}

async fn import_documents<R: BufRead>(pool: &SqlitePool, reader: R) -> Result<usize> {
    let mut count = 0;
    for line in reader.lines() {
        let line = line.map_err(Error::Io)?;
        if line.trim().is_empty() {
            continue;
        }
        let record: DocumentRecord =
            serde_json::from_str(&line).map_err(|e| Error::Parse(format!("Invalid JSON: {}", e)))?;

        sqlx::query(
            r#"
            INSERT OR REPLACE INTO documents
            (id, project_id, doc_type, title, description, content, format, version,
             status, model_used, tokens_used, generation_cost_usd, created_at, updated_at)
            VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
            "#,
        )
        .bind(&record.id)
        .bind(&record.project_id)
        .bind(&record.doc_type)
        .bind(&record.title)
        .bind(&record.description)
        .bind(&record.content)
        .bind(&record.format)
        .bind(record.version)
        .bind(&record.status)
        .bind(&record.model_used)
        .bind(record.tokens_used)
        .bind(record.generation_cost_usd)
        .bind(&record.created_at)
        .bind(&record.updated_at)
        .execute(pool)
        .await?;

        count += 1;
    }
    Ok(count)
}

async fn import_document_versions<R: BufRead>(pool: &SqlitePool, reader: R) -> Result<usize> {
    let mut count = 0;
    for line in reader.lines() {
        let line = line.map_err(Error::Io)?;
        if line.trim().is_empty() {
            continue;
        }
        let record: DocumentVersionRecord =
            serde_json::from_str(&line).map_err(|e| Error::Parse(format!("Invalid JSON: {}", e)))?;

        sqlx::query(
            r#"
            INSERT OR REPLACE INTO document_versions
            (id, document_id, version_number, content, change_summary, model_used, created_at)
            VALUES (?, ?, ?, ?, ?, ?, ?)
            "#,
        )
        .bind(&record.id)
        .bind(&record.document_id)
        .bind(record.version_number)
        .bind(&record.content)
        .bind(&record.change_summary)
        .bind(&record.model_used)
        .bind(&record.created_at)
        .execute(pool)
        .await?;

        count += 1;
    }
    Ok(count)
}

async fn import_llm_costs<R: BufRead>(pool: &SqlitePool, reader: R) -> Result<usize> {
    let mut count = 0;
    for line in reader.lines() {
        let line = line.map_err(Error::Io)?;
        if line.trim().is_empty() {
            continue;
        }
        let record: LlmCostRecord =
            serde_json::from_str(&line).map_err(|e| Error::Parse(format!("Invalid JSON: {}", e)))?;

        sqlx::query(
            r#"
            INSERT OR REPLACE INTO llm_costs
            (id, project_id, model, input_tokens, output_tokens, input_cost_usd,
             output_cost_usd, context, created_at)
            VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?)
            "#,
        )
        .bind(&record.id)
        .bind(&record.project_id)
        .bind(&record.model)
        .bind(record.input_tokens)
        .bind(record.output_tokens)
        .bind(record.input_cost_usd)
        .bind(record.output_cost_usd)
        .bind(&record.context)
        .bind(&record.created_at)
        .execute(pool)
        .await?;

        count += 1;
    }
    Ok(count)
}

async fn import_daily_cost_summaries<R: BufRead>(pool: &SqlitePool, reader: R) -> Result<usize> {
    let mut count = 0;
    for line in reader.lines() {
        let line = line.map_err(Error::Io)?;
        if line.trim().is_empty() {
            continue;
        }
        let record: DailyCostSummaryRecord =
            serde_json::from_str(&line).map_err(|e| Error::Parse(format!("Invalid JSON: {}", e)))?;

        sqlx::query(
            r#"
            INSERT OR REPLACE INTO daily_cost_summaries
            (date, project_id, model, total_cost_usd, total_input_tokens,
             total_output_tokens, call_count, updated_at)
            VALUES (?, ?, ?, ?, ?, ?, ?, ?)
            "#,
        )
        .bind(&record.date)
        .bind(&record.project_id)
        .bind(&record.model)
        .bind(record.total_cost_usd)
        .bind(record.total_input_tokens)
        .bind(record.total_output_tokens)
        .bind(record.call_count)
        .bind(&record.updated_at)
        .execute(pool)
        .await?;

        count += 1;
    }
    Ok(count)
}

async fn import_feature_extraction_history<R: BufRead>(
    pool: &SqlitePool,
    reader: R,
) -> Result<usize> {
    let mut count = 0;
    for line in reader.lines() {
        let line = line.map_err(Error::Io)?;
        if line.trim().is_empty() {
            continue;
        }
        let record: FeatureExtractionHistoryRecord =
            serde_json::from_str(&line).map_err(|e| Error::Parse(format!("Invalid JSON: {}", e)))?;

        sqlx::query(
            r#"
            INSERT OR REPLACE INTO feature_extraction_history
            (id, project_id, conversation_id, model_used, tokens_used, cost_usd,
             phases_created, features_created, raw_response, created_at)
            VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
            "#,
        )
        .bind(&record.id)
        .bind(&record.project_id)
        .bind(&record.conversation_id)
        .bind(&record.model_used)
        .bind(record.tokens_used)
        .bind(record.cost_usd)
        .bind(record.phases_created)
        .bind(record.features_created)
        .bind(&record.raw_response)
        .bind(&record.created_at)
        .execute(pool)
        .await?;

        count += 1;
    }
    Ok(count)
}

async fn import_learned_skills<R: BufRead>(pool: &SqlitePool, reader: R) -> Result<usize> {
    let mut count = 0;
    for line in reader.lines() {
        let line = line.map_err(Error::Io)?;
        if line.trim().is_empty() {
            continue;
        }
        let record: LearnedSkillRecord =
            serde_json::from_str(&line).map_err(|e| Error::Parse(format!("Invalid JSON: {}", e)))?;

        sqlx::query(
            r#"
            INSERT OR REPLACE INTO learned_skills (
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
            "#,
        )
        .bind(&record.id)
        .bind(&record.name)
        .bind(&record.description)
        .bind(&record.category)
        .bind(&record.pattern_type)
        .bind(&record.pattern_template)
        .bind(&record.pattern_variables)
        .bind(&record.pattern_applicability)
        .bind(&record.pattern_limitations)
        .bind(&record.source_project_id)
        .bind(&record.source_feature_id)
        .bind(&record.source_agent_type)
        .bind(&record.source_original_task)
        .bind(&record.source_model_used)
        .bind(record.source_tokens_used)
        .bind(&record.confidence)
        .bind(&record.tags)
        .bind(record.times_used)
        .bind(record.success_count)
        .bind(record.failure_count)
        .bind(&record.last_used_at)
        .bind(&record.metadata)
        .bind(&record.created_at)
        .bind(&record.updated_at)
        .execute(pool)
        .await?;

        count += 1;
    }
    Ok(count)
}

// =============================================================================
// Utility Functions
// =============================================================================

/// Check if the sync directory has pending changes compared to database
pub async fn check_sync_status(pool: &SqlitePool, project_dir: &Path) -> Result<SyncStatus> {
    let sync_dir = project_dir.join(SYNC_DIR);
    let metadata_path = sync_dir.join("_metadata.json");

    if !metadata_path.exists() {
        return Ok(SyncStatus {
            dirty: true,
            last_sync_at: None,
            pending_changes: 0,
            message: "No previous export found".to_string(),
        });
    }

    // Read metadata
    let metadata_file = File::open(&metadata_path).map_err(Error::Io)?;
    let metadata: SyncMetadata = serde_json::from_reader(metadata_file)
        .map_err(|e| Error::Other(format!("Failed to read metadata: {}", e)))?;

    // Count current records in database
    let mut current_counts = HashMap::new();
    for table in EXPORTABLE_TABLES {
        let count: (i64,) = sqlx::query_as(&format!("SELECT COUNT(*) FROM {}", table))
            .fetch_one(pool)
            .await?;
        current_counts.insert(table.to_string(), count.0 as usize);
    }

    // Compare counts
    let mut pending_changes = 0usize;
    for (table, &current) in &current_counts {
        let previous = metadata.record_counts.get(table).copied().unwrap_or(0);
        if current != previous {
            pending_changes += current.abs_diff(previous);
        }
    }

    Ok(SyncStatus {
        dirty: pending_changes > 0,
        last_sync_at: Some(metadata.exported_at.to_rfc3339()),
        pending_changes,
        message: if pending_changes > 0 {
            format!("{} record(s) changed since last export", pending_changes)
        } else {
            "Up to date".to_string()
        },
    })
}

/// Sync status information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SyncStatus {
    /// Whether there are pending changes
    pub dirty: bool,
    /// When the last sync occurred
    pub last_sync_at: Option<String>,
    /// Number of pending changes
    pub pending_changes: usize,
    /// Human-readable status message
    pub message: String,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::storage::Database;
    use tempfile::TempDir;

    async fn setup_test_db() -> (Database, TempDir) {
        let db = Database::in_memory()
            .await
            .expect("Failed to create test database");
        let temp_dir = TempDir::new().expect("Failed to create temp directory");
        (db, temp_dir)
    }

    #[tokio::test]
    async fn test_export_empty_database() {
        let (db, temp_dir) = setup_test_db().await;

        let result = export_to_jsonl(db.pool(), temp_dir.path()).await.unwrap();

        assert_eq!(result.metadata.total_records, 0);
        assert!(result.sync_dir.exists());

        // Check all files were created
        for table in EXPORTABLE_TABLES {
            let file_path = result.sync_dir.join(format!("{}.jsonl", table));
            assert!(
                file_path.exists(),
                "Expected {} to exist",
                file_path.display()
            );
        }
    }

    #[tokio::test]
    async fn test_export_and_import_project() {
        let (db, temp_dir) = setup_test_db().await;

        // Insert a project
        let project_id = uuid::Uuid::new_v4().to_string();
        sqlx::query(
            r#"
            INSERT INTO projects (id, name, framework, status)
            VALUES (?, ?, ?, ?)
            "#,
        )
        .bind(&project_id)
        .bind("Test Project")
        .bind("rust")
        .bind("active")
        .execute(db.pool())
        .await
        .unwrap();

        // Export
        let export_result = export_to_jsonl(db.pool(), temp_dir.path()).await.unwrap();
        assert_eq!(
            export_result.metadata.record_counts.get("projects"),
            Some(&1)
        );

        // Clear database
        sqlx::query("DELETE FROM projects")
            .execute(db.pool())
            .await
            .unwrap();

        // Verify empty
        let count: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM projects")
            .fetch_one(db.pool())
            .await
            .unwrap();
        assert_eq!(count.0, 0);

        // Import
        let import_result = import_from_jsonl(db.pool(), temp_dir.path()).await.unwrap();
        assert_eq!(import_result.record_counts.get("projects"), Some(&1));

        // Verify imported
        let (name,): (String,) = sqlx::query_as("SELECT name FROM projects WHERE id = ?")
            .bind(&project_id)
            .fetch_one(db.pool())
            .await
            .unwrap();
        assert_eq!(name, "Test Project");
    }

    #[tokio::test]
    async fn test_sync_status_no_previous_export() {
        let (db, temp_dir) = setup_test_db().await;

        let status = check_sync_status(db.pool(), temp_dir.path())
            .await
            .unwrap();

        assert!(status.dirty);
        assert!(status.last_sync_at.is_none());
    }

    #[tokio::test]
    async fn test_sync_status_up_to_date() {
        let (db, temp_dir) = setup_test_db().await;

        // Export first
        export_to_jsonl(db.pool(), temp_dir.path()).await.unwrap();

        // Check status (should be clean)
        let status = check_sync_status(db.pool(), temp_dir.path())
            .await
            .unwrap();

        assert!(!status.dirty);
        assert!(status.last_sync_at.is_some());
        assert_eq!(status.pending_changes, 0);
    }

    #[tokio::test]
    async fn test_sync_status_with_changes() {
        let (db, temp_dir) = setup_test_db().await;

        // Export empty database
        export_to_jsonl(db.pool(), temp_dir.path()).await.unwrap();

        // Add a project
        let project_id = uuid::Uuid::new_v4().to_string();
        sqlx::query("INSERT INTO projects (id, name, framework, status) VALUES (?, ?, ?, ?)")
            .bind(&project_id)
            .bind("New Project")
            .bind("rust")
            .bind("active")
            .execute(db.pool())
            .await
            .unwrap();

        // Check status (should be dirty)
        let status = check_sync_status(db.pool(), temp_dir.path())
            .await
            .unwrap();

        assert!(status.dirty);
        assert_eq!(status.pending_changes, 1);
    }

    #[tokio::test]
    async fn test_jsonl_format_is_valid() {
        let (db, temp_dir) = setup_test_db().await;

        // Insert test data
        let project_id = uuid::Uuid::new_v4().to_string();
        sqlx::query(
            "INSERT INTO projects (id, name, framework, status, description) VALUES (?, ?, ?, ?, ?)",
        )
        .bind(&project_id)
        .bind("Test Project")
        .bind("rust")
        .bind("active")
        .bind("A test project with \"quotes\" and special chars: <>&")
        .execute(db.pool())
        .await
        .unwrap();

        // Export
        export_to_jsonl(db.pool(), temp_dir.path()).await.unwrap();

        // Read and validate JSONL format
        let jsonl_path = temp_dir.path().join(SYNC_DIR).join("projects.jsonl");
        let content = fs::read_to_string(&jsonl_path).unwrap();

        // Should be exactly one line
        let lines: Vec<&str> = content.lines().collect();
        assert_eq!(lines.len(), 1);

        // Should be valid JSON
        let record: ProjectRecord = serde_json::from_str(lines[0]).unwrap();
        assert_eq!(record.name, "Test Project");
        assert!(record.description.unwrap().contains("\"quotes\""));
    }
}
