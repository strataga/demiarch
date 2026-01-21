//! Demiarch CLI - local-first AI app builder

use clap::{Parser, Subcommand};
use demiarch_core::commands::{checkpoint, document, feature, generate, project};
use demiarch_core::config::Config;
use demiarch_core::cost::CostTracker;
use demiarch_core::domain::locking::{LockConfig, LockManager};
use demiarch_core::domain::session::{
    SessionManager, SessionStatus, ShutdownConfig, ShutdownHandler,
};
use demiarch_core::storage::{Database, DatabaseManager};
use demiarch_core::visualization::{HierarchyTree, NodeStyle, RenderOptions, TreeBuilder};
use std::sync::Arc;
use tracing::{info, warn};
use uuid::Uuid;

#[derive(Parser)]
#[command(name = "demiarch")]
#[command(author, version, about = "Local-first AI app builder", long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Commands,

    /// Output format (text or json)
    #[arg(long, global = true, default_value = "text")]
    format: OutputFormat,

    /// Quiet mode (minimal output)
    #[arg(short, long, global = true)]
    quiet: bool,
}

#[derive(Clone, Copy, Default, clap::ValueEnum)]
enum OutputFormat {
    #[default]
    Text,
    Json,
}

#[derive(Subcommand)]
enum Commands {
    /// Create a new project
    New {
        /// Project name
        name: String,
        /// Framework (nextjs, react, vue, etc.)
        #[arg(short, long)]
        framework: String,
        /// Git repository URL
        #[arg(short, long)]
        repo: Option<String>,
    },

    /// Start conversational discovery
    Chat,

    /// Manage projects
    Projects {
        #[command(subcommand)]
        action: ProjectAction,
    },

    /// Manage features
    Features {
        #[command(subcommand)]
        action: FeatureAction,
    },

    /// Generate code from a natural language description
    Generate {
        /// Natural language description of what to generate
        description: String,
        /// Dry run (preview without writing files)
        #[arg(short, long)]
        dry_run: bool,
    },

    /// Generate and manage documents (PRD, Architecture, etc.)
    Documents {
        #[command(subcommand)]
        action: DocumentAction,
    },

    /// Manage learned skills
    Skills {
        #[command(subcommand)]
        action: SkillAction,
    },

    /// Model routing configuration
    Routing {
        #[command(subcommand)]
        action: RoutingAction,
    },

    /// Context and memory management
    Context {
        #[command(subcommand)]
        action: ContextAction,
    },

    /// Lifecycle hooks management
    Hooks {
        #[command(subcommand)]
        action: HookAction,
    },

    /// View costs and usage
    Costs {
        /// Project ID (optional, defaults to current)
        #[arg(short, long)]
        project: Option<String>,
    },

    /// Sync SQLite <-> JSONL
    Sync {
        #[command(subcommand)]
        action: SyncAction,
    },

    /// Manage checkpoints for code safety and recovery
    Checkpoints {
        #[command(subcommand)]
        action: CheckpointAction,
    },

    /// Configuration management
    Config {
        #[command(subcommand)]
        action: ConfigAction,
    },

    /// Run health check
    Doctor,

    /// Open TUI monitor (watch mode)
    Watch,

    /// View agent hierarchy and status
    Agents {
        #[command(subcommand)]
        action: AgentAction,
    },

    /// Manage global sessions
    Sessions {
        #[command(subcommand)]
        action: SessionAction,
    },
}

#[derive(Subcommand)]
enum ProjectAction {
    /// List all projects
    List,
    /// Show project details
    Show { id: String },
    /// Archive a project
    Archive { id: String },
    /// Delete a project
    Delete {
        id: String,
        #[arg(long)]
        force: bool,
    },
}

#[derive(Subcommand)]
enum FeatureAction {
    /// List features
    List {
        #[arg(short, long)]
        status: Option<String>,
    },
    /// Show feature details
    Show { id: String },
    /// Create a new feature
    Create {
        title: String,
        #[arg(short, long)]
        phase: Option<String>,
    },
    /// Update a feature
    Update {
        id: String,
        #[arg(short, long)]
        status: Option<String>,
    },
    /// Delete a feature
    Delete { id: String },
}

#[derive(Subcommand)]
enum DocumentAction {
    /// Generate a PRD for a project
    GeneratePrd {
        /// Project ID
        #[arg(short, long)]
        project: String,
    },
    /// Generate an architecture document for a project
    GenerateArchitecture {
        /// Project ID
        #[arg(short, long)]
        project: String,
    },
    /// List documents for a project
    List {
        /// Project ID
        #[arg(short, long)]
        project: String,
        /// Document type (prd, architecture, design, tech_spec)
        #[arg(short, long)]
        doc_type: Option<String>,
    },
    /// Show document details
    Show { id: String },
    /// Update document status
    UpdateStatus {
        /// Document ID
        id: String,
        /// New status (draft, review, final, archived)
        #[arg(short, long)]
        status: String,
    },
    /// Export a document to a file
    Export {
        /// Document ID
        id: String,
        /// Output file path
        #[arg(short, long)]
        output: String,
    },
    /// Delete a document
    Delete { id: String },
}

#[derive(Subcommand)]
enum SkillAction {
    /// List all skills
    List {
        #[arg(short, long)]
        category: Option<String>,
    },
    /// Show skill details
    Show { id: String },
    /// Search skills
    Search { query: String },
    /// Extract skills from current context
    Extract {
        #[arg(short, long)]
        description: Option<String>,
    },
    /// Delete a skill
    Delete { id: String },
    /// Show skill statistics
    Stats,
}

#[derive(Subcommand)]
enum RoutingAction {
    /// Show routing status
    Status,
    /// Set routing preference
    SetPreference { preference: String },
    /// Show model performance
    Performance {
        #[arg(short, long)]
        task: Option<String>,
    },
    /// Show routing history
    History {
        #[arg(short, long)]
        limit: Option<usize>,
    },
}

#[derive(Subcommand)]
enum ContextAction {
    /// Show context statistics
    Stats {
        #[arg(short, long)]
        project: Option<String>,
    },
    /// Search context
    Search {
        query: String,
        #[arg(short, long)]
        level: Option<u8>,
    },
    /// Prune old context
    Prune {
        #[arg(long)]
        older_than: Option<u32>,
        #[arg(long)]
        dry_run: bool,
    },
    /// Rebuild context index
    Rebuild {
        #[arg(short, long)]
        project: Option<String>,
    },
}

#[derive(Subcommand)]
enum HookAction {
    /// List hooks
    List {
        #[arg(short, long)]
        r#type: Option<String>,
    },
    /// Register a new hook
    Register {
        hook_type: String,
        name: String,
        #[arg(long)]
        handler: String,
    },
    /// Enable a hook
    Enable { id: String },
    /// Disable a hook
    Disable { id: String },
    /// Remove a hook
    Remove { id: String },
    /// Show hook execution history
    History {
        #[arg(short, long)]
        limit: Option<usize>,
    },
}

#[derive(Subcommand)]
enum SyncAction {
    /// Flush SQLite to JSONL
    Flush,
    /// Import JSONL to SQLite
    Import,
    /// Show sync status
    Status,
}

#[derive(Subcommand)]
enum CheckpointAction {
    /// List checkpoints for a project
    List {
        /// Project ID
        #[arg(short, long)]
        project: String,
    },
    /// Show checkpoint statistics for a project
    Stats {
        /// Project ID
        #[arg(short, long)]
        project: String,
    },
    /// Create a manual checkpoint
    Create {
        /// Project ID
        #[arg(short, long)]
        project: String,
        /// Description for the checkpoint
        #[arg(short, long)]
        description: String,
        /// Optional feature ID
        #[arg(short, long)]
        feature: Option<String>,
    },
    /// Restore project state to a checkpoint
    Restore {
        /// Checkpoint ID to restore
        id: String,
        /// Skip confirmation prompt
        #[arg(long)]
        force: bool,
    },
    /// Verify a checkpoint's integrity
    Verify {
        /// Checkpoint ID
        id: String,
    },
    /// Delete a checkpoint
    Delete {
        /// Checkpoint ID
        id: String,
    },
    /// Delete all checkpoints for a project
    DeleteAll {
        /// Project ID
        #[arg(short, long)]
        project: String,
        /// Force deletion without confirmation
        #[arg(long)]
        force: bool,
    },
}

#[derive(Subcommand)]
enum ConfigAction {
    /// Get a configuration value
    Get { key: String },
    /// Set a configuration value
    Set { key: String, value: String },
    /// List all configuration values
    List,
    /// Reset configuration to defaults
    Reset,
    /// Show config file path
    Path,
}

#[derive(Subcommand)]
enum AgentAction {
    /// Show agent hierarchy tree (demo/example)
    Tree {
        /// Use ASCII characters instead of Unicode
        #[arg(long)]
        ascii: bool,
        /// Show agent IDs
        #[arg(long, default_value = "true")]
        show_ids: bool,
        /// Show token usage
        #[arg(long, default_value = "true")]
        show_tokens: bool,
        /// Maximum depth to display (-1 for unlimited)
        #[arg(long, default_value = "-1")]
        max_depth: i32,
        /// Show minimal output (no icons)
        #[arg(long)]
        minimal: bool,
    },
    /// Show agent hierarchy as compact single-line status
    Status,
    /// List agent types and their capabilities
    Types,
}

#[derive(Subcommand)]
enum SessionAction {
    /// List all sessions
    List {
        /// Filter by status (active, paused, completed, abandoned)
        #[arg(short, long)]
        status: Option<String>,
        /// Maximum number of sessions to show
        #[arg(short, long)]
        limit: Option<i32>,
    },
    /// Show session details
    Show { id: String },
    /// Start a new session
    Start {
        /// Project ID to associate with the session
        #[arg(short, long)]
        project: Option<String>,
        /// Description of what you're working on
        #[arg(short, long)]
        description: Option<String>,
    },
    /// Pause the current session
    Pause {
        /// Session ID (defaults to active session)
        #[arg(short, long)]
        id: Option<String>,
    },
    /// Resume a paused session
    Resume {
        /// Session ID to resume
        id: String,
    },
    /// Complete a session
    Complete {
        /// Session ID (defaults to active session)
        #[arg(short, long)]
        id: Option<String>,
    },
    /// Abandon a session
    Abandon {
        /// Session ID (defaults to active session)
        #[arg(short, long)]
        id: Option<String>,
    },
    /// Show current active session
    Current,
    /// Show session statistics
    Stats,
    /// Show session events/history
    Events {
        /// Session ID
        id: String,
        /// Maximum number of events to show
        #[arg(short, long)]
        limit: Option<i32>,
    },
    /// Clean up old sessions
    Cleanup {
        /// Delete sessions older than this many days
        #[arg(long, default_value = "30")]
        days: i64,
        /// Also clean up old session events
        #[arg(long)]
        events: bool,
        /// Days threshold for events (defaults to same as sessions)
        #[arg(long)]
        event_days: Option<i64>,
    },
    /// End the current session gracefully (with full cleanup)
    End {
        /// Complete the session (default) vs abandon it
        #[arg(long)]
        abandon: bool,
        /// Run cleanup operations during shutdown
        #[arg(long)]
        cleanup: bool,
        /// Days threshold for cleanup (if --cleanup is set)
        #[arg(long, default_value = "30")]
        cleanup_days: i64,
    },
}

fn validate_license_key_on_startup() -> anyhow::Result<()> {
    use std::env;

    // Check if license enforcement is enabled (default: true)
    let enforcement_enabled = match env::var("DEMIARCH_REQUIRE_LICENSE") {
        Err(_) => true, // Default to enabled
        Ok(val) if val == "0" || val.to_lowercase() == "false" => {
            // Check if unsafe mode is explicitly enabled
            let unsafe_mode = match env::var("DEMIARCH_UNSAFE_ALLOW_UNLICENSED") {
                Err(_) => false,
                Ok(v) => v == "1" || v.to_lowercase() == "true",
            };

            if unsafe_mode {
                eprintln!("Warning: License enforcement is DISABLED");
                eprintln!("Warning: Running in UNSAFE mode");
                eprintln!("Warning: Unverified plugins may execute");
                return Ok(());
            } else {
                return Err(anyhow::anyhow!(
                    "License enforcement is disabled but UNSAFE_ALLOW_UNLICENSED is not set. \
                     Set DEMIARCH_UNSAFE_ALLOW_UNLICENSED=1 to proceed in unsafe mode."
                ));
            }
        }
        Ok(_) => true, // Explicitly enabled
    };

    if !enforcement_enabled {
        return Ok(());
    }

    // Validate license issuer key exists and is valid
    let key_b64 = env::var("DEMIARCH_LICENSE_ISSUER_KEY").map_err(|_| {
        anyhow::anyhow!(
            "License enforcement is enabled, but DEMIARCH_LICENSE_ISSUER_KEY is not set. \
             Set a valid 32-byte Ed25519 public key (base64-encoded)."
        )
    })?;

    // Validate key format (must be base64, decode to 32 bytes)
    use base64::Engine;
    use base64::engine::general_purpose::STANDARD as BASE64_STANDARD;

    let key_bytes = BASE64_STANDARD
        .decode(&key_b64)
        .map_err(|e| anyhow::anyhow!("Invalid license issuer key encoding: {}", e))?;

    if key_bytes.len() != 32 {
        return Err(anyhow::anyhow!(
            "License issuer key must be exactly 32 bytes (Ed25519 public key). Got {} bytes.",
            key_bytes.len()
        ));
    }

    // Try to parse as Ed25519 public key to verify it's valid
    ed25519_dalek::VerifyingKey::from_bytes(&key_bytes.try_into().map_err(|_| {
        anyhow::anyhow!("License issuer key could not be converted to 32-byte array")
    })?)
    .map_err(|e| anyhow::anyhow!("Invalid Ed25519 public key: {}", e))?;

    tracing::debug!("License issuer key validated successfully");
    Ok(())
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Initialize tracing
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::from_default_env()
                .add_directive("demiarch=info".parse()?),
        )
        .init();

    // Validate license issuer key early if license enforcement is enabled
    validate_license_key_on_startup()?;

    let cli = Cli::parse();

    // Initialize database manager for commands that need it
    // We lazily initialize it only when needed to avoid startup overhead
    let get_db = || async { DatabaseManager::new().await.map(|mgr| mgr.global().clone()) };

    match cli.command {
        Commands::New {
            name,
            framework,
            repo,
        } => {
            let db = get_db().await?;
            cmd_new(&db, &name, &framework, repo.as_deref(), cli.quiet).await
        }

        Commands::Chat => cmd_chat(cli.quiet).await,

        Commands::Projects { action } => {
            let db = get_db().await?;
            cmd_projects(&db, action, cli.quiet).await
        }

        Commands::Features { action } => cmd_features(action, cli.quiet).await,

        Commands::Generate {
            description,
            dry_run,
        } => cmd_generate(&description, dry_run, cli.quiet).await,

        Commands::Documents { action } => {
            let db = get_db().await?;
            cmd_documents(&db, action, cli.quiet).await
        }

        Commands::Skills { action } => cmd_skills(action, cli.quiet).await,

        Commands::Routing { action } => cmd_routing(action, cli.quiet).await,

        Commands::Context { action } => cmd_context(action, cli.quiet).await,

        Commands::Hooks { action } => cmd_hooks(action, cli.quiet).await,

        Commands::Costs { project } => cmd_costs(project.as_deref(), cli.quiet).await,

        Commands::Sync { action } => cmd_sync(action, cli.quiet).await,

        Commands::Checkpoints { action } => cmd_checkpoints(action, cli.quiet).await,

        Commands::Config { action } => cmd_config(action, cli.quiet),

        Commands::Doctor => cmd_doctor(cli.quiet).await,

        Commands::Watch => cmd_watch(cli.quiet),

        Commands::Agents { action } => cmd_agents(action, cli.quiet),

        Commands::Sessions { action } => {
            let db = get_db().await?;
            cmd_sessions(&db, action, cli.quiet).await
        }
    }
}

// ============================================================================
// Command Implementations
// ============================================================================

async fn cmd_new(
    db: &Database,
    name: &str,
    framework: &str,
    repo: Option<&str>,
    quiet: bool,
) -> anyhow::Result<()> {
    if !quiet {
        println!("Creating project '{}'...", name);
    }

    let repo_url = repo.unwrap_or("");
    let created_project = project::create_with_db(db, name, framework, repo_url).await?;

    if !quiet {
        println!("Project created successfully!");
        println!("  ID: {}", created_project.id);
        println!("  Name: {}", created_project.name);
        println!("  Framework: {}", created_project.framework);
        if !repo_url.is_empty() {
            println!("  Repository: {}", repo_url);
        }
        println!("\nNext steps:");
        println!("  1. cd into your project directory");
        println!("  2. Run `demiarch chat` to start conversational discovery");
        println!("  3. Run `demiarch features create <title>` to add features");
    }

    Ok(())
}

async fn cmd_chat(quiet: bool) -> anyhow::Result<()> {
    if !quiet {
        println!("Starting conversational discovery...");
        println!("(Chat interface not yet implemented)");
        println!("\nUse `demiarch features create <title>` to manually add features.");
    }
    Ok(())
}

async fn cmd_projects(db: &Database, action: ProjectAction, quiet: bool) -> anyhow::Result<()> {
    match action {
        ProjectAction::List => {
            let projects = project::list_with_db(db, Some(project::ProjectStatus::Active)).await?;
            if projects.is_empty() {
                if !quiet {
                    println!("No projects found.");
                    println!("\nCreate one with: demiarch new <name> --framework <framework>");
                }
            } else {
                if !quiet {
                    println!("Projects:");
                }
                for p in projects {
                    let status_indicator = match p.status {
                        project::ProjectStatus::Active => "",
                        project::ProjectStatus::Archived => " [archived]",
                        project::ProjectStatus::Deleted => " [deleted]",
                    };
                    println!(
                        "  {} - {} ({}){}",
                        &p.id[..8],
                        p.name,
                        p.framework,
                        status_indicator
                    );
                }
            }
        }
        ProjectAction::Show { id } => {
            // Try to find by ID or name
            let found_project = if let Some(p) = project::get_with_db(db, &id).await? {
                Some(p)
            } else {
                // Try to find by name
                let repo = project::ProjectRepository::new(db);
                repo.get_by_name(&id).await?
            };

            match found_project {
                Some(p) => {
                    println!("Project: {}", p.name);
                    println!("  ID: {}", p.id);
                    println!("  Framework: {}", p.framework);
                    println!("  Status: {}", p.status.as_str());
                    if !p.repo_url.is_empty() {
                        println!("  Repository: {}", p.repo_url);
                    }
                    if let Some(desc) = &p.description {
                        println!("  Description: {}", desc);
                    }
                    println!("  Created: {}", p.created_at.format("%Y-%m-%d %H:%M:%S"));
                    println!("  Updated: {}", p.updated_at.format("%Y-%m-%d %H:%M:%S"));
                }
                None => {
                    return Err(anyhow::anyhow!(
                        "Project '{}' not found. Run `demiarch projects list` to see all projects.",
                        id
                    ));
                }
            }
        }
        ProjectAction::Archive { id } => {
            project::archive_with_db(db, &id).await?;
            if !quiet {
                println!("Project '{}' archived.", id);
            }
        }
        ProjectAction::Delete { id, force } => {
            if !force && !quiet {
                println!("Warning: This will permanently delete project '{}'.", id);
                println!("Use --force to confirm deletion.");
                return Ok(());
            }
            project::delete_with_db(db, &id, force).await?;
            if !quiet {
                if force {
                    println!("Project '{}' permanently deleted.", id);
                } else {
                    println!("Project '{}' marked as deleted.", id);
                }
            }
        }
    }
    Ok(())
}

async fn cmd_features(action: FeatureAction, quiet: bool) -> anyhow::Result<()> {
    // TODO: Get current project ID from context
    let project_id = "current";

    match action {
        FeatureAction::List { status } => {
            let features = feature::list(project_id, status.as_deref()).await?;
            if features.is_empty() {
                if !quiet {
                    println!("No features found.");
                    println!("\nCreate one with: demiarch features create <title>");
                }
            } else {
                if !quiet {
                    println!("Features:");
                }
                for f in features {
                    println!("  - {}", f);
                }
            }
        }
        FeatureAction::Show { id } => {
            // TODO: Implement feature::get
            println!("Feature: {}", id);
            println!("(Feature details not yet implemented)");
        }
        FeatureAction::Create { title, phase } => {
            let feature_id = feature::create(project_id, &title, phase.as_deref()).await?;
            if !quiet {
                println!("Feature created: {}", feature_id);
                println!(
                    "\nNext: Run `demiarch generate {}` to generate code.",
                    feature_id
                );
            }
        }
        FeatureAction::Update { id, status } => {
            feature::update(&id, status.as_deref(), None).await?;
            if !quiet {
                println!("Feature '{}' updated.", id);
            }
        }
        FeatureAction::Delete { id } => {
            feature::delete(&id).await?;
            if !quiet {
                println!("Feature '{}' deleted.", id);
            }
        }
    }
    Ok(())
}

async fn cmd_generate(description: &str, dry_run: bool, quiet: bool) -> anyhow::Result<()> {
    if !quiet {
        if dry_run {
            println!("Dry run: Generating code for: {}", description);
        } else {
            println!("Generating code for: {}", description);
        }
        println!();
    }

    let result = generate::generate(description, dry_run)
        .await
        .map_err(|e| anyhow::anyhow!("{}", e))?;

    if !quiet {
        println!("Generation complete!");
        println!();
        println!("  Files created: {}", result.files_created);
        println!("  Files modified: {}", result.files_modified);
        println!("  Tokens used: {}", result.tokens_used);
        println!("  Estimated cost: ${:.4}", result.cost_usd);

        if !result.files.is_empty() {
            println!();
            println!("Generated files:");
            for file in &result.files {
                let status = if file.is_new { "new" } else { "modified" };
                let lang = file
                    .language
                    .as_ref()
                    .map(|l| format!(" ({})", l))
                    .unwrap_or_default();
                if dry_run {
                    println!("  [would create] {}{}", file.path.display(), lang);
                } else {
                    println!("  [{}] {}{}", status, file.path.display(), lang);
                }
            }
        }

        if dry_run {
            println!();
            println!("Dry run complete. No files were written.");
            println!("Run without --dry-run to create the files.");
        }
    }

    Ok(())
}

async fn cmd_documents(db: &Database, action: DocumentAction, quiet: bool) -> anyhow::Result<()> {
    let config = Config::load()?;
    let cost_tracker = Arc::new(CostTracker::from_config(&config.cost));

    match action {
        DocumentAction::GeneratePrd { project } => {
            if !quiet {
                println!("Generating PRD for project '{}'...", project);
                println!();
            }

            let doc = document::generate_prd(db, &project, Some(cost_tracker.clone()))
                .await
                .map_err(|e| anyhow::anyhow!("{}", e))?;

            if !quiet {
                println!("PRD generated successfully!");
                println!();
                println!("  Document ID: {}", doc.id);
                println!("  Title: {}", doc.title);
                println!("  Status: {:?}", doc.status);
                if let Some(model) = &doc.model_used {
                    println!("  Model: {}", model);
                }
                if let Some(tokens) = doc.tokens_used {
                    println!("  Tokens used: {}", tokens);
                }
                if let Some(cost) = doc.generation_cost_usd {
                    println!("  Generation cost: ${:.4}", cost);
                }
                println!();
                println!("View with: demiarch documents show {}", doc.id);
                println!(
                    "Export with: demiarch documents export {} --output prd.md",
                    doc.id
                );
            }
        }

        DocumentAction::GenerateArchitecture { project } => {
            if !quiet {
                println!(
                    "Generating architecture document for project '{}'...",
                    project
                );
                println!();
            }

            let doc = document::generate_architecture(db, &project, Some(cost_tracker.clone()))
                .await
                .map_err(|e| anyhow::anyhow!("{}", e))?;

            if !quiet {
                println!("Architecture document generated successfully!");
                println!();
                println!("  Document ID: {}", doc.id);
                println!("  Title: {}", doc.title);
                println!("  Status: {:?}", doc.status);
                if let Some(model) = &doc.model_used {
                    println!("  Model: {}", model);
                }
                if let Some(tokens) = doc.tokens_used {
                    println!("  Tokens used: {}", tokens);
                }
                if let Some(cost) = doc.generation_cost_usd {
                    println!("  Generation cost: ${:.4}", cost);
                }
                println!();
                println!("View with: demiarch documents show {}", doc.id);
                println!(
                    "Export with: demiarch documents export {} --output architecture.md",
                    doc.id
                );
            }
        }

        DocumentAction::List { project, doc_type } => {
            let dt = doc_type
                .as_ref()
                .and_then(|s| document::DocumentType::parse(s));

            let docs = document::list_documents(db, &project, dt)
                .await
                .map_err(|e| anyhow::anyhow!("{}", e))?;

            if docs.is_empty() {
                if !quiet {
                    println!("No documents found for project '{}'.", project);
                    println!();
                    println!("Generate one with:");
                    println!("  demiarch documents generate-prd --project {}", project);
                    println!(
                        "  demiarch documents generate-architecture --project {}",
                        project
                    );
                }
            } else {
                if !quiet {
                    println!("Documents for project '{}':", project);
                    println!();
                }
                for doc in docs {
                    println!(
                        "  {} - {} ({:?}) [{}]",
                        &doc.id[..8],
                        doc.title,
                        doc.doc_type,
                        doc.status.as_str()
                    );
                }
            }
        }

        DocumentAction::Show { id } => {
            let doc = document::get_document(db, &id)
                .await
                .map_err(|e| anyhow::anyhow!("{}", e))?
                .ok_or_else(|| anyhow::anyhow!("Document '{}' not found", id))?;

            println!("Document: {}", doc.title);
            println!("=========={}", "=".repeat(doc.title.len()));
            println!();
            println!("ID: {}", doc.id);
            println!("Type: {}", doc.doc_type.display_name());
            println!("Status: {:?}", doc.status);
            println!("Version: {}", doc.version);
            if let Some(model) = &doc.model_used {
                println!("Model: {}", model);
            }
            if let Some(tokens) = doc.tokens_used {
                println!("Tokens: {}", tokens);
            }
            if let Some(cost) = doc.generation_cost_usd {
                println!("Cost: ${:.4}", cost);
            }
            println!("Created: {}", doc.created_at.format("%Y-%m-%d %H:%M:%S"));
            println!("Updated: {}", doc.updated_at.format("%Y-%m-%d %H:%M:%S"));
            println!();
            println!("Content:");
            println!("--------");
            println!("{}", doc.content);
        }

        DocumentAction::UpdateStatus { id, status } => {
            let new_status = document::DocumentStatus::parse(&status).ok_or_else(|| {
                anyhow::anyhow!(
                    "Invalid status '{}'. Use: draft, review, final, archived",
                    status
                )
            })?;

            document::update_document_status(db, &id, new_status)
                .await
                .map_err(|e| anyhow::anyhow!("{}", e))?;

            if !quiet {
                println!("Document '{}' status updated to '{}'.", id, status);
            }
        }

        DocumentAction::Export { id, output } => {
            let path = std::path::PathBuf::from(&output);

            document::export_document(db, &id, &path)
                .await
                .map_err(|e| anyhow::anyhow!("{}", e))?;

            if !quiet {
                println!("Document '{}' exported to '{}'.", id, output);
            }
        }

        DocumentAction::Delete { id } => {
            document::delete_document(db, &id)
                .await
                .map_err(|e| anyhow::anyhow!("{}", e))?;

            if !quiet {
                println!("Document '{}' deleted.", id);
            }
        }
    }

    Ok(())
}

async fn cmd_skills(action: SkillAction, quiet: bool) -> anyhow::Result<()> {
    match action {
        SkillAction::List { category } => {
            if !quiet {
                println!("Skills:");
                if let Some(cat) = category {
                    println!("  (filtered by category: {})", cat);
                }
                println!("  (No skills learned yet)");
            }
        }
        SkillAction::Show { id } => {
            println!("Skill: {}", id);
            println!("(Skill details not yet implemented)");
        }
        SkillAction::Search { query } => {
            if !quiet {
                println!("Searching skills for: {}", query);
                println!("  (No matching skills found)");
            }
        }
        SkillAction::Extract { description } => {
            if !quiet {
                println!("Extracting skills from current context...");
                if let Some(desc) = description {
                    println!("  Description: {}", desc);
                }
                println!("(Skill extraction not yet implemented)");
            }
        }
        SkillAction::Delete { id } => {
            if !quiet {
                println!("Deleting skill: {}", id);
                println!("(Skill deletion not yet implemented)");
            }
        }
        SkillAction::Stats => {
            if !quiet {
                println!("Skill Statistics:");
                println!("  Total skills: 0");
                println!("  By category: (none)");
            }
        }
    }
    Ok(())
}

async fn cmd_routing(action: RoutingAction, quiet: bool) -> anyhow::Result<()> {
    let config = Config::load()?;

    match action {
        RoutingAction::Status => {
            if !quiet {
                println!("Routing Status:");
                println!("  Preference: {}", config.routing.preference);
                println!("  Default model: {}", config.llm.default_model);
                println!(
                    "  Fallback models: {}",
                    config.llm.fallback_models.join(", ")
                );
            }
        }
        RoutingAction::SetPreference { preference } => {
            let mut config = config;
            config.set("routing.preference", &preference)?;
            config.save()?;
            if !quiet {
                println!("Routing preference set to: {}", preference);
            }
        }
        RoutingAction::Performance { task } => {
            if !quiet {
                println!("Model Performance:");
                if let Some(t) = task {
                    println!("  (filtered by task: {})", t);
                }
                println!("  (No performance data yet)");
            }
        }
        RoutingAction::History { limit } => {
            if !quiet {
                let limit = limit.unwrap_or(10);
                println!("Routing History (last {}):", limit);
                println!("  (No routing history yet)");
            }
        }
    }
    Ok(())
}

async fn cmd_context(action: ContextAction, quiet: bool) -> anyhow::Result<()> {
    match action {
        ContextAction::Stats { project } => {
            if !quiet {
                println!("Context Statistics:");
                if let Some(p) = project {
                    println!("  Project: {}", p);
                }
                println!("  Total entries: 0");
                println!("  Total tokens: 0");
            }
        }
        ContextAction::Search { query, level } => {
            if !quiet {
                println!("Searching context for: {}", query);
                if let Some(l) = level {
                    println!("  Detail level: {}", l);
                }
                println!("  (No matching context found)");
            }
        }
        ContextAction::Prune {
            older_than,
            dry_run,
        } => {
            if !quiet {
                let days = older_than.unwrap_or(30);
                if dry_run {
                    println!("Dry run: Would prune context older than {} days", days);
                } else {
                    println!("Pruning context older than {} days...", days);
                }
                println!("  (No context to prune)");
            }
        }
        ContextAction::Rebuild { project } => {
            if !quiet {
                println!("Rebuilding context index...");
                if let Some(p) = project {
                    println!("  Project: {}", p);
                }
                println!("  (Context rebuild not yet implemented)");
            }
        }
    }
    Ok(())
}

async fn cmd_hooks(action: HookAction, quiet: bool) -> anyhow::Result<()> {
    match action {
        HookAction::List { r#type } => {
            if !quiet {
                println!("Registered Hooks:");
                if let Some(t) = r#type {
                    println!("  (filtered by type: {})", t);
                }
                println!("  (No hooks registered)");
            }
        }
        HookAction::Register {
            hook_type,
            name,
            handler,
        } => {
            if !quiet {
                println!("Registering hook:");
                println!("  Type: {}", hook_type);
                println!("  Name: {}", name);
                println!("  Handler: {}", handler);
                println!("  (Hook registration not yet implemented)");
            }
        }
        HookAction::Enable { id } => {
            if !quiet {
                println!("Enabling hook: {}", id);
                println!("  (Hook not found)");
            }
        }
        HookAction::Disable { id } => {
            if !quiet {
                println!("Disabling hook: {}", id);
                println!("  (Hook not found)");
            }
        }
        HookAction::Remove { id } => {
            if !quiet {
                println!("Removing hook: {}", id);
                println!("  (Hook not found)");
            }
        }
        HookAction::History { limit } => {
            if !quiet {
                let limit = limit.unwrap_or(10);
                println!("Hook Execution History (last {}):", limit);
                println!("  (No hook executions yet)");
            }
        }
    }
    Ok(())
}

async fn cmd_costs(project: Option<&str>, quiet: bool) -> anyhow::Result<()> {
    let config = Config::load()?;
    let tracker = CostTracker::from_config(&config.cost);

    if !quiet {
        println!("Cost Summary:");
        if let Some(p) = project {
            println!("  Project: {}", p);
        }
        println!();

        // Get today's summary from the tracker
        let today_total = tracker.today_total();
        let remaining = tracker.remaining_budget();

        println!("  Today: ${:.4}", today_total);

        // Show summary if we have one
        if let Some(summary) = tracker.today_summary() {
            println!("    Calls: {}", summary.call_count);
            println!(
                "    Tokens: {} input, {} output",
                summary.total_input_tokens, summary.total_output_tokens
            );
            if !summary.by_model.is_empty() {
                println!("    By model:");
                for (model, model_summary) in &summary.by_model {
                    println!(
                        "      {}: ${:.4} ({} calls)",
                        model, model_summary.total_cost_usd, model_summary.call_count
                    );
                }
            }
        }

        println!();
        println!("  Daily limit: ${:.2}", tracker.daily_limit());
        println!("  Remaining: ${:.2}", remaining);
        println!(
            "  Alert threshold: {:.0}%",
            config.cost.alert_threshold * 100.0
        );

        // Show warnings if approaching or over limit
        if tracker.is_over_limit() {
            println!();
            println!("  [WARNING] Daily limit exceeded!");
        } else if tracker.is_approaching_limit() {
            println!();
            println!(
                "  [WARNING] Approaching daily limit ({}% used)",
                ((today_total / tracker.daily_limit()) * 100.0) as u32
            );
        }
    }
    Ok(())
}

async fn cmd_sync(action: SyncAction, quiet: bool) -> anyhow::Result<()> {
    match action {
        SyncAction::Flush => {
            if !quiet {
                println!("Flushing SQLite to JSONL...");
                println!("  (Sync not yet implemented)");
            }
        }
        SyncAction::Import => {
            if !quiet {
                println!("Importing JSONL to SQLite...");
                println!("  (Sync not yet implemented)");
            }
        }
        SyncAction::Status => {
            if !quiet {
                println!("Sync Status:");
                println!("  SQLite: up to date");
                println!("  JSONL: up to date");
                println!("  Last sync: never");
            }
        }
    }
    Ok(())
}

async fn cmd_checkpoints(action: CheckpointAction, quiet: bool) -> anyhow::Result<()> {
    match action {
        CheckpointAction::List { project } => {
            let project_id = uuid::Uuid::parse_str(&project)
                .map_err(|_| anyhow::anyhow!("Invalid project ID: {}", project))?;

            let checkpoints = checkpoint::list_checkpoints(project_id)
                .await
                .map_err(|e| anyhow::anyhow!("{}", e))?;

            if checkpoints.is_empty() {
                if !quiet {
                    println!("No checkpoints found for project '{}'.", project);
                    println!();
                    println!("Checkpoints are created automatically before code generation.");
                    println!("You can also create a manual checkpoint with:");
                    println!(
                        "  demiarch checkpoints create --project {} --description \"Before my changes\"",
                        project
                    );
                }
            } else {
                if !quiet {
                    println!("Checkpoints for project '{}':", &project[..8]);
                    println!();
                }
                for cp in checkpoints {
                    let feature_info = cp
                        .feature_id
                        .map(|f| format!(" [feature: {}]", &f.to_string()[..8]))
                        .unwrap_or_default();
                    println!(
                        "  {} - {} ({}){}",
                        &cp.id.to_string()[..8],
                        cp.description,
                        cp.display_size(),
                        feature_info
                    );
                    println!(
                        "       Created: {}",
                        cp.created_at.format("%Y-%m-%d %H:%M:%S")
                    );
                }
            }
        }

        CheckpointAction::Stats { project } => {
            let project_id = uuid::Uuid::parse_str(&project)
                .map_err(|_| anyhow::anyhow!("Invalid project ID: {}", project))?;

            let stats = checkpoint::get_checkpoint_stats(project_id)
                .await
                .map_err(|e| anyhow::anyhow!("{}", e))?;

            if !quiet {
                println!("Checkpoint Statistics:");
                println!("  Project: {}", &project[..8]);
                println!("  Total checkpoints: {}", stats.total_count);
                println!("  Total size: {}", stats.display_total_size());
                if let Some(oldest) = stats.oldest_checkpoint {
                    println!("  Oldest: {}", oldest.format("%Y-%m-%d %H:%M:%S"));
                }
                if let Some(newest) = stats.newest_checkpoint {
                    println!("  Newest: {}", newest.format("%Y-%m-%d %H:%M:%S"));
                }
            }
        }

        CheckpointAction::Create {
            project,
            description,
            feature,
        } => {
            let project_id = uuid::Uuid::parse_str(&project)
                .map_err(|_| anyhow::anyhow!("Invalid project ID: {}", project))?;

            let feature_id = feature
                .map(|f| uuid::Uuid::parse_str(&f))
                .transpose()
                .map_err(|_| anyhow::anyhow!("Invalid feature ID"))?;

            if !quiet {
                println!("Creating checkpoint...");
            }

            let cp = checkpoint::create_checkpoint(project_id, description, feature_id)
                .await
                .map_err(|e| anyhow::anyhow!("{}", e))?;

            if !quiet {
                println!("Checkpoint created successfully!");
                println!("  ID: {}", cp.id);
                println!("  Description: {}", cp.description);
                println!("  Size: {}", cp.display_size());
            }
        }

        CheckpointAction::Restore { id, force } => {
            let checkpoint_id = uuid::Uuid::parse_str(&id)
                .map_err(|_| anyhow::anyhow!("Invalid checkpoint ID: {}", id))?;

            if !force && !quiet {
                println!(
                    "Warning: This will restore project state to checkpoint '{}'.",
                    &id[..8]
                );
                println!("Current project state will be backed up automatically.");
                println!("Use --force to skip this confirmation.");
                return Ok(());
            }

            if !quiet {
                println!("Restoring checkpoint '{}'...", &id[..8]);
            }

            let result = checkpoint::restore_checkpoint(checkpoint_id)
                .await
                .map_err(|e| anyhow::anyhow!("{}", e))?;

            if !quiet {
                println!();
                println!("Checkpoint restored successfully!");
                println!();
                println!(
                    "  Restored to: {}",
                    result.checkpoint_timestamp.format("%Y-%m-%d %H:%M:%S")
                );
                println!("  Description: {}", result.checkpoint_description);
                println!();
                println!("  Phases restored: {}", result.phases_restored);
                println!("  Features restored: {}", result.features_restored);
                println!("  Messages restored: {}", result.messages_restored);
                if result.files_restored > 0 {
                    println!("  Files restored: {}", result.files_restored);
                }
                println!();
                println!(
                    "  Safety backup created: {}",
                    &result.safety_backup_id.to_string()[..8]
                );
                println!();
                println!("To undo this restore, run:");
                println!(
                    "  demiarch checkpoints restore {} --force",
                    &result.safety_backup_id.to_string()[..8]
                );
            }
        }

        CheckpointAction::Verify { id } => {
            let checkpoint_id = uuid::Uuid::parse_str(&id)
                .map_err(|_| anyhow::anyhow!("Invalid checkpoint ID: {}", id))?;

            let is_valid = checkpoint::verify_checkpoint(checkpoint_id)
                .await
                .map_err(|e| anyhow::anyhow!("{}", e))?;

            if !quiet {
                if is_valid {
                    println!("[OK] Checkpoint '{}' signature is valid.", &id[..8]);
                } else {
                    println!(
                        "[!!] Checkpoint '{}' signature verification FAILED.",
                        &id[..8]
                    );
                    println!("     The checkpoint data may have been corrupted or tampered with.");
                }
            }
        }

        CheckpointAction::Delete { id } => {
            let checkpoint_id = uuid::Uuid::parse_str(&id)
                .map_err(|_| anyhow::anyhow!("Invalid checkpoint ID: {}", id))?;

            let deleted = checkpoint::delete_checkpoint(checkpoint_id)
                .await
                .map_err(|e| anyhow::anyhow!("{}", e))?;

            if !quiet {
                if deleted {
                    println!("Checkpoint '{}' deleted.", &id[..8]);
                } else {
                    println!("Checkpoint '{}' not found.", &id[..8]);
                }
            }
        }

        CheckpointAction::DeleteAll { project, force } => {
            let project_id = uuid::Uuid::parse_str(&project)
                .map_err(|_| anyhow::anyhow!("Invalid project ID: {}", project))?;

            if !force && !quiet {
                println!(
                    "Warning: This will delete ALL checkpoints for project '{}'.",
                    &project[..8]
                );
                println!("Use --force to confirm deletion.");
                return Ok(());
            }

            let deleted = checkpoint::delete_all_checkpoints(project_id)
                .await
                .map_err(|e| anyhow::anyhow!("{}", e))?;

            if !quiet {
                println!(
                    "Deleted {} checkpoint(s) for project '{}'.",
                    deleted,
                    &project[..8]
                );
            }
        }
    }
    Ok(())
}

fn cmd_config(action: ConfigAction, quiet: bool) -> anyhow::Result<()> {
    match action {
        ConfigAction::Get { key } => {
            let config = Config::load()?;
            let value = config.get(&key)?;
            println!("{}", value);
        }
        ConfigAction::Set { key, value } => {
            let mut config = Config::load()?;
            config.set(&key, &value)?;
            config.save()?;
            if !quiet {
                println!("Set {} = {}", key, value);
            }
        }
        ConfigAction::List => {
            let config = Config::load()?;
            let items = config.list()?;
            for (key, value) in items {
                println!("{} = {}", key, value);
            }
        }
        ConfigAction::Reset => {
            Config::reset()?;
            if !quiet {
                println!("Configuration reset to defaults.");
            }
        }
        ConfigAction::Path => {
            let path = Config::config_path()?;
            println!("{}", path.display());
        }
    }
    Ok(())
}

async fn cmd_doctor(quiet: bool) -> anyhow::Result<()> {
    use std::env;

    if !quiet {
        println!("Demiarch Health Check");
        println!("=====================");
        println!();
    }

    let mut all_ok = true;

    // Check configuration
    match Config::load() {
        Ok(config) => {
            if !quiet {
                println!("[OK] Configuration: Valid");
            }

            // Check API key
            match config.llm.resolved_api_key() {
                Ok(Some(_)) => {
                    if !quiet {
                        let redacted = config.llm.redacted_api_key()?.unwrap_or_default();
                        println!("[OK] API Key: Configured ({})", redacted);
                    }
                }
                Ok(None) => {
                    all_ok = false;
                    if !quiet {
                        warn!("API Key: Not configured");
                        println!("[!!] API Key: Not configured");
                        println!(
                            "     Set DEMIARCH_API_KEY or OPENROUTER_API_KEY environment variable"
                        );
                    }
                }
                Err(e) => {
                    all_ok = false;
                    if !quiet {
                        println!("[!!] API Key: Error - {}", e);
                    }
                }
            }
        }
        Err(e) => {
            all_ok = false;
            if !quiet {
                println!("[!!] Configuration: Error - {}", e);
            }
        }
    }

    // Check license
    match env::var("DEMIARCH_LICENSE_ISSUER_KEY") {
        Ok(_) => {
            if !quiet {
                println!("[OK] License: Configured");
            }
        }
        Err(_) => {
            let unsafe_mode = env::var("DEMIARCH_UNSAFE_ALLOW_UNLICENSED")
                .map(|v| v == "1" || v.to_lowercase() == "true")
                .unwrap_or(false);

            if unsafe_mode {
                if !quiet {
                    println!("[!!] License: Running in UNSAFE mode");
                }
            } else {
                all_ok = false;
                if !quiet {
                    println!("[!!] License: Not configured");
                    println!("     Set DEMIARCH_LICENSE_ISSUER_KEY environment variable");
                }
            }
        }
    }

    // Check config file location
    if !quiet {
        match Config::config_path() {
            Ok(path) => {
                if path.exists() {
                    println!("[OK] Config file: {}", path.display());
                } else {
                    println!("[--] Config file: {} (using defaults)", path.display());
                }
            }
            Err(e) => {
                println!("[!!] Config file: Error - {}", e);
            }
        }
    }

    // Check database
    if !quiet {
        match DatabaseManager::new().await {
            Ok(manager) => {
                let db = manager.global();
                match db.health_check().await {
                    Ok(()) => {
                        println!("[OK] Database: Connected");
                        println!("     Path: {}", db.path().display());

                        // Check migration status
                        match db.migration_status().await {
                            Ok(status) => {
                                if status.needs_migration {
                                    println!(
                                        "[!!] Database: Migrations pending (v{} -> v{})",
                                        status.current_version, status.target_version
                                    );
                                } else {
                                    println!("[OK] Database: Schema v{}", status.current_version);
                                }
                            }
                            Err(e) => {
                                println!("[!!] Database: Migration check failed - {}", e);
                            }
                        }

                        // Show project count
                        let projects = project::list_with_db(db, None).await.unwrap_or_default();
                        println!("     Projects: {}", projects.len());
                    }
                    Err(e) => {
                        all_ok = false;
                        println!("[!!] Database: Health check failed - {}", e);
                    }
                }
            }
            Err(e) => {
                all_ok = false;
                println!("[!!] Database: Failed to initialize - {}", e);
            }
        }
    }

    // Summary
    if !quiet {
        println!();
        if all_ok {
            println!("All checks passed!");
        } else {
            println!("Some checks failed. See above for details.");
        }
    }

    Ok(())
}

fn cmd_watch(quiet: bool) -> anyhow::Result<()> {
    if !quiet {
        info!("Starting TUI monitor...");
    }

    // Try to run the TUI binary
    let result = std::process::Command::new("demiarch-tui").status();

    match result {
        Ok(status) if status.success() => Ok(()),
        Ok(_) => {
            if !quiet {
                println!("TUI exited with an error.");
            }
            Ok(())
        }
        Err(_) => {
            if !quiet {
                println!("Could not start TUI monitor.");
                println!();
                println!("The TUI binary 'demiarch-tui' is not in your PATH.");
                println!("Either:");
                println!("  1. Add the target/debug or target/release directory to PATH");
                println!("  2. Run `cargo run --bin demiarch-tui` from the project root");
                println!("  3. Install with `cargo install --path crates/demiarch-tui`");
            }
            Ok(())
        }
    }
}

fn cmd_agents(action: AgentAction, quiet: bool) -> anyhow::Result<()> {
    use demiarch_core::agents::AgentType;

    match action {
        AgentAction::Tree {
            ascii,
            show_ids,
            show_tokens,
            max_depth,
            minimal,
        } => {
            // Build a demo tree to show the hierarchy structure
            let tree = TreeBuilder::demo_tree();

            // Configure render options
            let options = if minimal {
                RenderOptions::minimal().with_max_depth(max_depth)
            } else {
                let style = if ascii {
                    NodeStyle::Ascii
                } else {
                    NodeStyle::Unicode
                };
                RenderOptions::default()
                    .with_style(style)
                    .with_max_depth(max_depth)
            };

            // Override specific options
            let mut options = options;
            options.show_ids = show_ids;
            options.show_tokens = show_tokens;

            let renderer = HierarchyTree::with_options(tree, options);

            if !quiet {
                println!("{}", renderer.render_with_summary());
                println!();
                println!("Note: This is a demo tree showing the agent hierarchy structure.");
                println!("During code generation, you'll see actual agents in this tree.");
            } else {
                println!("{}", renderer.render());
            }
        }

        AgentAction::Status => {
            let tree = TreeBuilder::demo_tree();
            let renderer = HierarchyTree::new(tree);

            if !quiet {
                println!("Agent Status: {}", renderer.render_compact());
            } else {
                println!("{}", renderer.render_compact());
            }
        }

        AgentAction::Types => {
            if !quiet {
                println!("Agent Types and Hierarchy");
                println!("=========================");
                println!();
                println!("Level 1 (Director):");
                println!(
                    "  {} - Session coordinator, manages overall workflow",
                    format_agent_type(AgentType::Orchestrator)
                );
                println!("        Can spawn: Planner");
                println!();
                println!("Level 2 (Coordinator):");
                println!(
                    "  {} - Decomposes features into tasks, creates execution plans",
                    format_agent_type(AgentType::Planner)
                );
                println!("        Can spawn: Coder, Reviewer, Tester");
                println!();
                println!("Level 3 (Workers - leaf nodes):");
                println!(
                    "  {} - Generates code implementations",
                    format_agent_type(AgentType::Coder)
                );
                println!(
                    "  {} - Reviews code for quality and correctness",
                    format_agent_type(AgentType::Reviewer)
                );
                println!(
                    "  {} - Creates and validates tests",
                    format_agent_type(AgentType::Tester)
                );
                println!();
                println!("Execution Flow:");
                println!("  1. User submits feature request");
                println!("  2. Orchestrator receives request, spawns Planner");
                println!("  3. Planner decomposes into tasks, spawns worker agents");
                println!("  4. Workers execute tasks (code, review, test)");
                println!("  5. Results bubble up through hierarchy");
                println!("  6. Orchestrator returns complete implementation");
            }
        }
    }
    Ok(())
}

fn format_agent_type(agent_type: demiarch_core::agents::AgentType) -> String {
    use demiarch_core::agents::AgentType;
    match agent_type {
        AgentType::Orchestrator => "🎭 Orchestrator".to_string(),
        AgentType::Planner => "📋 Planner".to_string(),
        AgentType::Coder => "💻 Coder".to_string(),
        AgentType::Reviewer => "🔍 Reviewer".to_string(),
        AgentType::Tester => "🧪 Tester".to_string(),
    }
}

// ============================================================================
// Session Commands
// ============================================================================

async fn cmd_sessions(db: &Database, action: SessionAction, quiet: bool) -> anyhow::Result<()> {
    let manager = SessionManager::new(db.pool().clone());

    match action {
        SessionAction::List { status, limit } => {
            let sessions = if let Some(status_str) = status {
                let status = SessionStatus::parse(&status_str).ok_or_else(|| {
                    anyhow::anyhow!(
                        "Invalid status: {}. Use: active, paused, completed, abandoned",
                        status_str
                    )
                })?;
                manager.list_by_status(status).await?
            } else {
                manager.list(limit).await?
            };

            if sessions.is_empty() {
                if !quiet {
                    println!("No sessions found.");
                    println!("\nStart a new session with: demiarch sessions start");
                }
            } else {
                if !quiet {
                    println!("Sessions:");
                    println!();
                }
                for s in sessions {
                    let status_icon = match s.status {
                        SessionStatus::Active => "🟢",
                        SessionStatus::Paused => "⏸️ ",
                        SessionStatus::Completed => "✅",
                        SessionStatus::Abandoned => "❌",
                    };
                    let desc = s.description.as_deref().unwrap_or("(no description)");
                    let age = format_duration(chrono::Utc::now() - s.created_at);
                    println!(
                        "  {} {} - {} ({} ago)",
                        status_icon,
                        &s.id.to_string()[..8],
                        desc,
                        age
                    );
                }
            }
        }

        SessionAction::Show { id } => {
            let session_id = parse_session_id(&manager, &id).await?;
            let session = manager
                .get(session_id)
                .await?
                .ok_or_else(|| anyhow::anyhow!("Session '{}' not found", id))?;

            println!("Session: {}", session.id);
            println!("  Status: {}", session.status);
            println!("  Phase: {}", session.phase);
            if let Some(desc) = &session.description {
                println!("  Description: {}", desc);
            }
            if let Some(project_id) = session.current_project_id {
                println!("  Current Project: {}", project_id);
            }
            if let Some(feature_id) = session.current_feature_id {
                println!("  Current Feature: {}", feature_id);
            }
            if let Some(checkpoint_id) = session.last_checkpoint_id {
                println!("  Last Checkpoint: {}", checkpoint_id);
            }
            println!(
                "  Created: {}",
                session.created_at.format("%Y-%m-%d %H:%M:%S")
            );
            println!(
                "  Last Activity: {}",
                session.last_activity.format("%Y-%m-%d %H:%M:%S")
            );
            let duration = format_duration(session.duration());
            println!("  Duration: {}", duration);
        }

        SessionAction::Start {
            project,
            description,
        } => {
            let project_id = if let Some(p) = project {
                Some(
                    Uuid::parse_str(&p)
                        .map_err(|_| anyhow::anyhow!("Invalid project ID: {}", p))?,
                )
            } else {
                None
            };

            let session = manager.create(project_id, None, description).await?;

            if !quiet {
                println!("Session started: {}", session.id);
                if let Some(desc) = &session.description {
                    println!("  Description: {}", desc);
                }
                println!("\nCommands:");
                println!("  demiarch sessions pause     - Pause this session");
                println!("  demiarch sessions complete  - Complete this session");
                println!("  demiarch sessions current   - Show current session");
            }
        }

        SessionAction::Pause { id } => {
            let session_id = if let Some(id_str) = id {
                parse_session_id(&manager, &id_str).await?
            } else {
                manager
                    .get_active()
                    .await?
                    .map(|s| s.id)
                    .ok_or_else(|| anyhow::anyhow!("No active session to pause"))?
            };

            let session = manager.pause(session_id).await?;

            if !quiet {
                println!("Session paused: {}", session.id);
                println!(
                    "\nResume with: demiarch sessions resume {}",
                    &session.id.to_string()[..8]
                );
            }
        }

        SessionAction::Resume { id } => {
            let session_id = parse_session_id(&manager, &id).await?;
            let session = manager.resume(session_id).await?;

            if !quiet {
                println!("Session resumed: {}", session.id);
            }
        }

        SessionAction::Complete { id } => {
            let session_id = if let Some(id_str) = id {
                parse_session_id(&manager, &id_str).await?
            } else {
                manager
                    .get_active()
                    .await?
                    .map(|s| s.id)
                    .ok_or_else(|| anyhow::anyhow!("No active session to complete"))?
            };

            let session = manager.complete(session_id).await?;

            if !quiet {
                println!("Session completed: {}", session.id);
                let duration = format_duration(session.duration());
                println!("  Duration: {}", duration);
            }
        }

        SessionAction::Abandon { id } => {
            let session_id = if let Some(id_str) = id {
                parse_session_id(&manager, &id_str).await?
            } else {
                manager
                    .get_active()
                    .await?
                    .map(|s| s.id)
                    .ok_or_else(|| anyhow::anyhow!("No active session to abandon"))?
            };

            let session = manager.abandon(session_id).await?;

            if !quiet {
                println!("Session abandoned: {}", session.id);
            }
        }

        SessionAction::Current => match manager.get_active().await? {
            Some(session) => {
                println!("Current session: {}", session.id);
                println!("  Status: {}", session.status);
                println!("  Phase: {}", session.phase);
                if let Some(desc) = &session.description {
                    println!("  Description: {}", desc);
                }
                if let Some(project_id) = session.current_project_id {
                    println!("  Project: {}", project_id);
                }
                let duration = format_duration(session.duration());
                println!("  Duration: {}", duration);
            }
            None => {
                if !quiet {
                    println!("No active session.");
                    println!("\nStart one with: demiarch sessions start");
                }
            }
        },

        SessionAction::Stats => {
            let stats = manager.stats().await?;

            println!("Session Statistics:");
            println!("  Active:    {}", stats.active);
            println!("  Paused:    {}", stats.paused);
            println!("  Completed: {}", stats.completed);
            println!("  Abandoned: {}", stats.abandoned);
            println!("  ─────────────");
            println!("  Total:     {}", stats.total);
        }

        SessionAction::Events { id, limit } => {
            let session_id = parse_session_id(&manager, &id).await?;
            let events = manager.get_events(session_id, limit).await?;

            if events.is_empty() {
                if !quiet {
                    println!("No events found for session.");
                }
            } else {
                println!("Session Events (newest first):");
                println!();
                for event in events {
                    let time = event.created_at.format("%Y-%m-%d %H:%M:%S");
                    println!("  [{}] {}", time, event.event_type);
                    if let Some(data) = &event.data {
                        // Pretty print the data if it's not too large
                        let data_str = serde_json::to_string_pretty(data).unwrap_or_default();
                        if data_str.len() < 200 {
                            for line in data_str.lines() {
                                println!("    {}", line);
                            }
                        }
                    }
                }
            }
        }

        SessionAction::Cleanup {
            days,
            events,
            event_days,
        } => {
            let summary = if events {
                // Run full cleanup including events
                manager.full_cleanup(days, event_days).await?
            } else {
                // Just clean up sessions
                let sessions_deleted = manager.cleanup_old_sessions(days).await?;
                demiarch_core::domain::session::CleanupSummary {
                    sessions_deleted,
                    events_deleted: 0,
                    session_days: days,
                    event_days: 0,
                }
            };

            if !quiet {
                if summary.had_cleanup() {
                    println!("{}", summary.summary());
                } else {
                    println!("No old sessions or events to clean up.");
                }
            }
        }

        SessionAction::End {
            abandon,
            cleanup,
            cleanup_days,
        } => {
            // Create lock manager for shutdown handler
            let lock_dir = dirs::config_dir()
                .unwrap_or_default()
                .join("demiarch")
                .join("locks");
            let lock_config = LockConfig::default().with_lock_dir(lock_dir);
            let lock_manager = Arc::new(LockManager::new(lock_config));
            lock_manager.initialize().await?;

            // Create shutdown configuration
            let config = if cleanup {
                ShutdownConfig::with_cleanup(cleanup_days, cleanup_days)
            } else {
                ShutdownConfig::quick()
            };

            let handler = ShutdownHandler::new(manager.clone(), lock_manager, db.clone(), config);

            // Perform shutdown
            let result = if abandon {
                handler.abandon_session().await?
            } else {
                handler.end_session().await?
            };

            if !quiet {
                if let Some(session_id) = result.session_id {
                    let action = if abandon { "abandoned" } else { "completed" };
                    println!("Session {} {}", &session_id.to_string()[..8], action);
                } else {
                    println!("No active session to end.");
                }

                if result.locks_released > 0 {
                    println!("  Released {} locks", result.locks_released);
                }

                if result.sessions_cleaned > 0 || result.events_cleaned > 0 {
                    println!(
                        "  Cleaned up {} sessions, {} events",
                        result.sessions_cleaned, result.events_cleaned
                    );
                }

                if result.has_warnings() {
                    println!();
                    println!("Warnings:");
                    for warning in &result.warnings {
                        println!("  - {}", warning);
                    }
                }
            }
        }
    }

    Ok(())
}

/// Parse a session ID, supporting both full UUIDs and short prefixes
async fn parse_session_id(manager: &SessionManager, id: &str) -> anyhow::Result<Uuid> {
    // Try to parse as full UUID first
    if let Ok(uuid) = Uuid::parse_str(id) {
        return Ok(uuid);
    }

    // Try to find by prefix
    let sessions = manager.list(Some(100)).await?;
    let matches: Vec<_> = sessions
        .iter()
        .filter(|s| s.id.to_string().starts_with(id))
        .collect();

    match matches.len() {
        0 => Err(anyhow::anyhow!("No session found matching '{}'", id)),
        1 => Ok(matches[0].id),
        _ => Err(anyhow::anyhow!(
            "Ambiguous session ID '{}' matches {} sessions. Use a longer prefix.",
            id,
            matches.len()
        )),
    }
}

/// Format a duration in human-readable form
fn format_duration(duration: chrono::Duration) -> String {
    let total_secs = duration.num_seconds();
    if total_secs < 60 {
        format!("{}s", total_secs)
    } else if total_secs < 3600 {
        format!("{}m", total_secs / 60)
    } else if total_secs < 86400 {
        let hours = total_secs / 3600;
        let mins = (total_secs % 3600) / 60;
        if mins > 0 {
            format!("{}h {}m", hours, mins)
        } else {
            format!("{}h", hours)
        }
    } else {
        let days = total_secs / 86400;
        let hours = (total_secs % 86400) / 3600;
        if hours > 0 {
            format!("{}d {}h", days, hours)
        } else {
            format!("{}d", days)
        }
    }
}
