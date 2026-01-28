//! Demiarch CLI - local-first AI app builder

use clap::{Parser, Subcommand};
use demiarch_core::agents::{extract_files_from_response, AgentTool, AgentToolResult};
use demiarch_core::commands::{
    chat, checkpoint, document, feature, generate, graph, image, project,
};
use demiarch_core::config::Config;
use demiarch_core::context::ContextManager;
use demiarch_core::cost::CostTracker;
use demiarch_core::domain::knowledge::{EntityType, RelationshipType};
use demiarch_core::domain::locking::{LockConfig, LockManager};
use demiarch_core::domain::memory::{PersistentMemoryStore, RecallQuery};
use demiarch_core::domain::session::{
    SessionManager, SessionStatus, ShutdownConfig, ShutdownHandler,
};
use demiarch_core::llm::{LlmClient, Message, StreamEvent};
use demiarch_core::storage::{self, Database, DatabaseManager};
use demiarch_core::visualization::{HierarchyTree, NodeStyle, RenderOptions, TreeBuilder};
use futures_util::StreamExt;
use rustyline::error::ReadlineError;
use rustyline::DefaultEditor;
use serde_json;
use std::io::{self, Write};
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
        /// Custom project location (default: current directory)
        #[arg(short = 'P', long)]
        path: Option<std::path::PathBuf>,
    },

    /// Initialize demiarch in an existing directory
    Init {
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

    /// Explore knowledge graph (entities, relationships)
    Graph {
        #[command(subcommand)]
        action: GraphAction,
    },

    /// Generate and manipulate images using AI models
    Image {
        #[command(subcommand)]
        action: ImageAction,
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

#[derive(Subcommand)]
enum GraphAction {
    /// Show knowledge graph statistics
    Stats {
        /// Show detailed breakdown by type
        #[arg(long)]
        detailed: bool,
    },

    /// Explore entity relationships and connections
    Explore {
        /// Entity name or ID to explore
        entity: String,

        /// Max relationship depth to traverse (1-5)
        #[arg(short, long, default_value = "2")]
        depth: u32,

        /// Filter by relationship type (uses, depends_on, similar_to, etc.)
        #[arg(short, long)]
        relationship: Option<String>,

        /// Show in tree format
        #[arg(long)]
        tree: bool,
    },

    /// Search for entities by name or description
    Search {
        /// Search query
        query: String,

        /// Maximum results to return
        #[arg(short, long, default_value = "10")]
        limit: usize,
    },

    /// List entities by type
    List {
        /// Entity type (library, concept, pattern, technique, framework, etc.)
        entity_type: String,

        /// Maximum results to return
        #[arg(short, long, default_value = "20")]
        limit: usize,
    },
}

#[derive(Subcommand)]
enum ImageAction {
    /// Generate an image from a text description
    Generate {
        /// Text description of the image to generate
        prompt: String,
        /// Output file path
        #[arg(short, long)]
        output: Option<std::path::PathBuf>,
        /// Image size (square, portrait, landscape, or WxH e.g., 1024x768)
        #[arg(short, long, default_value = "square")]
        size: String,
        /// Image style (vivid, natural, photorealistic, artistic)
        #[arg(long)]
        style: Option<String>,
        /// Model to use for generation
        #[arg(long)]
        model: Option<String>,
        /// Negative prompt (what to avoid)
        #[arg(short, long)]
        negative: Option<String>,
        /// Seed for reproducible generation
        #[arg(long)]
        seed: Option<u64>,
    },
    /// Transform an existing image with a prompt
    Transform {
        /// Path to input image
        input: std::path::PathBuf,
        /// Description of the transformation
        prompt: String,
        /// Output file path
        #[arg(short, long)]
        output: Option<std::path::PathBuf>,
        /// Transformation strength (0.0-1.0, how much to change)
        #[arg(long, default_value = "0.75")]
        strength: f32,
        /// Model to use for transformation
        #[arg(long)]
        model: Option<String>,
    },
    /// Upscale an image to higher resolution
    Upscale {
        /// Path to input image
        input: std::path::PathBuf,
        /// Scale factor (2 or 4)
        #[arg(short, long, default_value = "2")]
        scale: u32,
        /// Output file path
        #[arg(short, long)]
        output: Option<std::path::PathBuf>,
        /// Model to use for upscaling (optional, uses local if not available)
        #[arg(long)]
        model: Option<String>,
    },
    /// Edit a region of an image using a mask (inpainting)
    Inpaint {
        /// Path to input image
        input: std::path::PathBuf,
        /// Path to mask image (white = edit, black = keep)
        mask: std::path::PathBuf,
        /// Description of what to fill in the masked area
        prompt: String,
        /// Output file path
        #[arg(short, long)]
        output: Option<std::path::PathBuf>,
        /// Model to use for inpainting
        #[arg(long)]
        model: Option<String>,
    },
    /// List available image generation models
    Models,
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
    use base64::engine::general_purpose::STANDARD as BASE64_STANDARD;
    use base64::Engine;

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
    // Load .env file if present (silently ignore if not found)
    dotenvy::dotenv().ok();

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
            path,
        } => {
            let db = get_db().await?;
            cmd_new(&db, &name, &framework, repo.as_deref(), path, cli.quiet).await
        }

        Commands::Init { framework, repo } => {
            let db = get_db().await?;
            cmd_init(&db, &framework, repo.as_deref(), cli.quiet).await
        }

        Commands::Chat => cmd_chat(cli.quiet).await,

        Commands::Projects { action } => {
            let db = get_db().await?;
            cmd_projects(&db, action, cli.quiet).await
        }

        Commands::Features { action } => {
            let db = get_db().await?;
            cmd_features(&db, action, cli.quiet).await
        }

        Commands::Generate {
            description,
            dry_run,
        } => cmd_generate(&description, dry_run, cli.quiet).await,

        Commands::Documents { action } => {
            let db = get_db().await?;
            cmd_documents(&db, action, cli.quiet).await
        }

        Commands::Skills { action } => {
            let db = get_db().await?;
            cmd_skills(&db, action, cli.quiet).await
        }

        Commands::Routing { action } => cmd_routing(action, cli.quiet).await,

        Commands::Context { action } => {
            let db = get_db().await?;
            cmd_context(&db, action, cli.quiet).await
        }

        Commands::Hooks { action } => cmd_hooks(action, cli.quiet).await,

        Commands::Costs { project } => cmd_costs(project.as_deref(), cli.quiet).await,

        Commands::Sync { action } => {
            let db = get_db().await?;
            cmd_sync(&db, action, cli.quiet).await
        }

        Commands::Checkpoints { action } => cmd_checkpoints(action, cli.quiet).await,

        Commands::Config { action } => cmd_config(action, cli.quiet),

        Commands::Doctor => cmd_doctor(cli.quiet).await,

        Commands::Watch => cmd_watch(cli.quiet),

        Commands::Agents { action } => cmd_agents(action, cli.quiet),

        Commands::Sessions { action } => {
            let db = get_db().await?;
            cmd_sessions(&db, action, cli.quiet).await
        }

        Commands::Graph { action } => {
            let db = get_db().await?;
            cmd_graph(&db, action, cli.quiet).await
        }

        Commands::Image { action } => cmd_image(action, cli.quiet).await,
    }
}

// ============================================================================
// Command Implementations
// ============================================================================

/// Check if git is available on the system
fn is_git_available() -> bool {
    std::process::Command::new("git")
        .arg("--version")
        .output()
        .map(|output| output.status.success())
        .unwrap_or(false)
}

/// Get framework-specific gitignore content
fn get_gitignore_content(framework: &str) -> &'static str {
    match framework.to_lowercase().as_str() {
        "rust" => "/target\n.env\n*.log\n",
        "node" | "nodejs" | "react" | "vue" | "nextjs" | "next" => {
            "node_modules/\n.env\n.env.local\n*.log\ndist/\nbuild/\n.next/\n"
        }
        "python" | "py" => "__pycache__/\n*.pyc\n.env\nvenv/\n.venv/\n*.log\n",
        "go" | "golang" => "*.exe\n*.log\n.env\nvendor/\n",
        _ => ".env\n*.log\n",
    }
}

async fn cmd_new(
    db: &Database,
    name: &str,
    framework: &str,
    repo: Option<&str>,
    custom_path: Option<std::path::PathBuf>,
    quiet: bool,
) -> anyhow::Result<()> {
    // Determine project path
    let project_path = if let Some(base_path) = custom_path {
        // Use custom path, joining with project name
        base_path.join(name)
    } else {
        // Default: current directory + name
        std::env::current_dir()?.join(name)
    };

    if !quiet {
        println!(
            "Creating project '{}' at {}...",
            name,
            project_path.display()
        );
    }

    // Check if directory already exists
    if project_path.exists() {
        return Err(anyhow::anyhow!(
            "Directory '{}' already exists.\n\
             Hint: Choose a different name, use --path to specify a different location,\n\
             or remove the existing directory.",
            project_path.display()
        ));
    }

    // Check if git is available before proceeding
    let git_available = is_git_available();
    if !git_available && !quiet {
        println!("  Warning: git is not installed or not in PATH. Skipping git initialization.");
    }

    // Create directory structure
    std::fs::create_dir_all(&project_path).map_err(|e| {
        anyhow::anyhow!(
            "Failed to create directory '{}': {}",
            project_path.display(),
            e
        )
    })?;
    std::fs::create_dir_all(project_path.join("src")).map_err(|e| {
        anyhow::anyhow!(
            "Failed to create src directory in '{}': {}",
            project_path.display(),
            e
        )
    })?;

    if !quiet {
        println!("  [ok] Created directory structure");
    }

    // Initialize git repository if available
    if git_available {
        let git_init = std::process::Command::new("git")
            .args(["init"])
            .current_dir(&project_path)
            .output();

        match git_init {
            Ok(output) if output.status.success() => {
                if !quiet {
                    println!("  [ok] Initialized git repository");
                }
            }
            Ok(output) => {
                let stderr = String::from_utf8_lossy(&output.stderr);
                warn!("Git init failed: {}", stderr);
                if !quiet {
                    println!("  [warn] Git init failed: {}", stderr.trim());
                }
            }
            Err(e) => {
                warn!("Could not run git init: {}", e);
                if !quiet {
                    println!("  [warn] Could not run git init: {}", e);
                }
            }
        }
    }

    // Create .gitignore
    let gitignore_content = get_gitignore_content(framework);
    std::fs::write(project_path.join(".gitignore"), gitignore_content)?;

    if !quiet {
        println!("  [ok] Created .gitignore for {} framework", framework);
    }

    let repo_url = repo.unwrap_or("");
    let created_project =
        project::create_with_path(db, name, framework, repo_url, &project_path).await?;

    if !quiet {
        println!("\n[ok] Project created successfully!");
        println!();
        println!("  ID:        {}", created_project.id);
        println!("  Name:      {}", created_project.name);
        println!("  Framework: {}", created_project.framework);
        println!("  Path:      {}", project_path.display());
        if !repo_url.is_empty() {
            println!("  Repo:      {}", repo_url);
        }
        println!();
        println!("Next steps:");
        println!("  1. cd {}", project_path.display());
        println!("  2. Run `demiarch chat` to start conversational discovery");
        println!("  3. Use `/generate` in chat to generate code");
    }

    Ok(())
}

/// Initialize demiarch in an existing directory
///
/// Unlike `new`, this does NOT create directories or initialize git.
/// It registers the current directory as a demiarch project.
async fn cmd_init(
    db: &Database,
    framework: &str,
    repo: Option<&str>,
    quiet: bool,
) -> anyhow::Result<()> {
    let current_dir = std::env::current_dir()?;
    let name = current_dir
        .file_name()
        .and_then(|n| n.to_str())
        .ok_or_else(|| anyhow::anyhow!("Could not determine directory name"))?
        .to_string();

    if !quiet {
        println!("Initializing demiarch in '{}'...", current_dir.display());
    }

    // Check if directory is empty or has content
    let is_empty = current_dir.read_dir()?.next().is_none();
    if is_empty && !quiet {
        println!("  Note: Directory is empty. Consider using `demiarch new` instead.");
    }

    // Check if already a demiarch project
    if let Ok(Some(existing)) = project::find_by_directory(db, &current_dir).await {
        return Err(anyhow::anyhow!(
            "Directory '{}' is already registered as project '{}' (ID: {}).\n\
             Use `demiarch projects show {}` to view details.",
            current_dir.display(),
            existing.name,
            existing.id,
            existing.id
        ));
    }

    // Create .gitignore if it doesn't exist
    let gitignore_path = current_dir.join(".gitignore");
    if !gitignore_path.exists() {
        let gitignore_content = get_gitignore_content(framework);
        std::fs::write(&gitignore_path, gitignore_content)?;
        if !quiet {
            println!("  [ok] Created .gitignore for {} framework", framework);
        }
    } else if !quiet {
        println!("  [ok] Existing .gitignore preserved");
    }

    let repo_url = repo.unwrap_or("");
    let created_project =
        project::create_with_path(db, &name, framework, repo_url, &current_dir).await?;

    if !quiet {
        println!("\n[ok] Project initialized successfully!");
        println!();
        println!("  ID:        {}", created_project.id);
        println!("  Name:      {}", created_project.name);
        println!("  Framework: {}", created_project.framework);
        println!("  Path:      {}", current_dir.display());
        if !repo_url.is_empty() {
            println!("  Repo:      {}", repo_url);
        }
        println!();
        println!("Next steps:");
        println!("  1. Run `demiarch chat` to start conversational discovery");
        println!("  2. Use `/generate` in chat to generate code");
    }

    Ok(())
}

/// Run a task through the agent orchestration system
///
/// This spawns an orchestrator agent which coordinates planners and workers
/// to complete complex code generation tasks.
#[allow(dead_code)]
async fn run_with_agents(task: &str) -> anyhow::Result<AgentToolResult> {
    run_with_agents_in_project(task, None).await
}

/// Run a task through the agent orchestration system with an optional project path
///
/// If a project path is provided, generated files will be written to that directory.
async fn run_with_agents_in_project(
    task: &str,
    project_path: Option<&std::path::Path>,
) -> anyhow::Result<AgentToolResult> {
    let config = Config::load()?;
    let api_key = config
        .llm
        .resolved_api_key()
        .map_err(|e| anyhow::anyhow!("Config error: {}", e))?
        .ok_or_else(|| {
            anyhow::anyhow!(
                "API key not configured. Set DEMIARCH_API_KEY or OPENROUTER_API_KEY environment variable."
            )
        })?;

    let llm_client = Arc::new(
        LlmClient::builder()
            .config(config.llm.clone())
            .api_key(api_key)
            .build()
            .map_err(|e| anyhow::anyhow!("Failed to create LLM client: {}", e))?,
    );

    let mut tool = AgentTool::new(llm_client);
    if let Some(path) = project_path {
        tool = tool.with_project_path(path.to_path_buf());
    }

    let result = tool
        .spawn_orchestrator(task)
        .await
        .map_err(|e| anyhow::anyhow!("Agent execution failed: {}", e))?;

    Ok(result)
}

async fn cmd_chat(quiet: bool) -> anyhow::Result<()> {
    let config = Config::load()?;
    let db = Database::default()
        .await
        .map_err(|e| anyhow::anyhow!("Failed to open database: {}", e))?;
    let context_store = PersistentMemoryStore::new(db.pool().clone());
    let context_manager = ContextManager::new().with_persistent_store(context_store);

    // Try to detect project from current directory first
    let current_dir = std::env::current_dir()?;
    let active_project = if let Some(p) = project::find_by_directory(&db, &current_dir)
        .await
        .map_err(|e| anyhow::anyhow!("Failed to find project: {}", e))?
    {
        if !quiet {
            println!("Detected project '{}' from current directory", p.name);
        }
        p
    } else {
        // Fall back to most recent project
        let project_repo = project::ProjectRepository::new(&db);
        let projects = project_repo.list(None).await?;

        if let Some(p) = projects.first() {
            p.clone()
        } else {
            if !quiet {
                println!("No projects found. Create one first with:");
                println!("  demiarch new <name> --framework <framework>");
            }
            return Ok(());
        }
    };

    // Get API key for LLM
    let api_key = config
        .llm
        .resolved_api_key()
        .map_err(|e| anyhow::anyhow!("Config error: {}", e))?
        .ok_or_else(|| {
            anyhow::anyhow!(
                "API key not configured. Set DEMIARCH_API_KEY or OPENROUTER_API_KEY environment variable."
            )
        })?;

    let llm_client = LlmClient::builder()
        .config(config.llm.clone())
        .api_key(api_key)
        .build()
        .map_err(|e| anyhow::anyhow!("Failed to create LLM client: {}", e))?;

    // Create a new conversation
    let conversation = chat::create_conversation(&db, &active_project.id, Some("Chat Session"))
        .await
        .map_err(|e| anyhow::anyhow!("Failed to create conversation: {}", e))?;

    if !quiet {
        println!("Demiarch Chat - Project: {}", active_project.name);
        println!("Type your message, or use these commands:");
        println!("  /quit      - Exit chat");
        println!("  /generate  - Generate code from the conversation");
        println!("  /clear     - Clear conversation history");
        println!();
    }

    // Set up rustyline editor
    let mut rl = DefaultEditor::new()?;
    let history_path = dirs::data_dir()
        .unwrap_or_else(|| std::path::PathBuf::from("."))
        .join("demiarch")
        .join("chat_history.txt");

    // Try to load history, ignore errors
    let _ = rl.load_history(&history_path);

    // System prompt for chat
    let system_prompt = format!(
        "You are Demiarch, an AI assistant specialized in software development. \
         You're helping with the project '{}' (framework: {}). \
         Help the user design features, write code, and solve problems. \
         When the user wants code generated, describe what you would create and ask if they want to proceed.",
        active_project.name, active_project.framework
    );

    loop {
        let readline = rl.readline("> ");
        match readline {
            Ok(line) => {
                let input = line.trim();

                if input.is_empty() {
                    continue;
                }

                // Add to history
                let _ = rl.add_history_entry(input);

                // Handle commands
                if input.starts_with('/') {
                    match input {
                        "/quit" | "/exit" | "/q" => {
                            if !quiet {
                                println!("Goodbye!");
                            }
                            break;
                        }
                        "/clear" => {
                            // Just create a new conversation
                            let _new_conv = chat::create_conversation(
                                &db,
                                &active_project.id,
                                Some("Chat Session"),
                            )
                            .await
                            .map_err(|e| anyhow::anyhow!("Failed to create conversation: {}", e))?;
                            if !quiet {
                                println!("Conversation cleared. Starting fresh.");
                            }
                            // Note: The current conversation continues - a full /clear would
                            // need to track the conversation ID mutably throughout the loop
                            continue;
                        }
                        "/generate" => {
                            // Get recent history and generate code
                            let history = chat::get_history(&db, &conversation.id, Some(10))
                                .await
                                .map_err(|e| anyhow::anyhow!("Failed to get history: {}", e))?;

                            if history.is_empty() {
                                println!("No conversation history to generate from.");
                                continue;
                            }

                            // Build a task from the conversation
                            let task = history
                                .iter()
                                .filter(|m| m.role == chat::MessageRole::User)
                                .map(|m| m.content.as_str())
                                .collect::<Vec<_>>()
                                .join("\n");

                            // Get project path for writing generated files
                            let project_path =
                                active_project.path.as_ref().map(std::path::PathBuf::from);

                            if !quiet {
                                println!("\nGenerating code based on conversation...");
                                if let Some(ref path) = project_path {
                                    println!("  Output directory: {}\n", path.display());
                                }
                            }

                            match run_with_agents_in_project(&task, project_path.as_deref()).await {
                                Ok(result) => {
                                    if result.success {
                                        println!("Generation complete!");
                                        println!("  Tokens used: {}", result.total_tokens);
                                        println!("  Children spawned: {}", result.children_spawned);

                                        // Write artifacts to project directory if available
                                        if let Some(ref path) = project_path {
                                            // Parse the output to extract code artifacts
                                            // The agent output contains the generated code
                                            let files = extract_files_from_response(&result.output);
                                            if !files.is_empty() {
                                                println!(
                                                    "  Writing {} file(s) to project:",
                                                    files.len()
                                                );
                                                for file in &files {
                                                    let file_path = path.join(&file.path);
                                                    if let Some(parent) = file_path.parent() {
                                                        if !parent.exists() {
                                                            std::fs::create_dir_all(parent)?;
                                                        }
                                                    }
                                                    std::fs::write(&file_path, &file.content)?;
                                                    println!("    {}", file_path.display());
                                                }
                                            }
                                        }
                                    } else {
                                        println!("Generation failed: {}", result.output);
                                    }
                                }
                                Err(e) => {
                                    println!("Error during generation: {}", e);
                                }
                            }
                            continue;
                        }
                        cmd => {
                            println!("Unknown command: {}", cmd);
                            println!("Available commands: /quit, /generate, /clear");
                            continue;
                        }
                    }
                }

                // Save user message
                chat::send_message(&db, &conversation.id, chat::MessageRole::User, input)
                    .await
                    .map_err(|e| anyhow::anyhow!("Failed to save message: {}", e))?;

                // Build messages for LLM including history
                let history = chat::get_history(&db, &conversation.id, Some(20))
                    .await
                    .map_err(|e| anyhow::anyhow!("Failed to get history: {}", e))?;

                let mut messages = vec![Message::system(&system_prompt)];
                for msg in &history {
                    let llm_msg = match msg.role {
                        chat::MessageRole::User => Message::user(&msg.content),
                        chat::MessageRole::Assistant => Message::assistant(&msg.content),
                        chat::MessageRole::System => Message::system(&msg.content),
                    };
                    messages.push(llm_msg);
                }

                // Stream the response
                match llm_client.complete_streaming(messages, None).await {
                    Ok(stream) => {
                        let mut response = String::new();
                        let mut stream = std::pin::pin!(stream);

                        while let Some(event) = stream.next().await {
                            match event {
                                Ok(StreamEvent::Chunk(chunk)) => {
                                    if let Some(content) = chunk.content() {
                                        print!("{}", content);
                                        io::stdout().flush()?;
                                        response.push_str(content);
                                    }
                                }
                                Ok(StreamEvent::Done) => {
                                    break;
                                }
                                Ok(StreamEvent::Error(e)) => {
                                    eprintln!("\nStream error: {}", e);
                                    break;
                                }
                                Err(e) => {
                                    eprintln!("\nError: {}", e);
                                    break;
                                }
                            }
                        }
                        println!(); // New line after response

                        // Save assistant message
                        if !response.is_empty() {
                            let saved = chat::send_message(
                                &db,
                                &conversation.id,
                                chat::MessageRole::Assistant,
                                &response,
                            )
                            .await
                            .map_err(|e| anyhow::anyhow!("Failed to save response: {}", e))?;

                            // Ingest conversation slice into context store for progressive recall
                            let history = chat::get_history(&db, &conversation.id, Some(12))
                                .await
                                .map_err(|e| anyhow::anyhow!("Failed to get history: {}", e))?;
                            let content = format_history_for_context(&history);
                            let _ = context_manager
                                .persistent()
                                .unwrap()
                                .ingest(
                                    &active_project.id,
                                    Some(&conversation.id),
                                    "chat",
                                    Some(&saved.id),
                                    &content,
                                )
                                .await;
                        }

                        // Check if the response suggests code generation
                        if should_offer_generation(&response) && !quiet {
                            println!(
                                "\nHint: Type /generate to create the code, or continue chatting."
                            );
                        }
                    }
                    Err(e) => {
                        eprintln!("Error calling LLM: {}", e);
                    }
                }
            }
            Err(ReadlineError::Interrupted) => {
                // Ctrl-C
                if !quiet {
                    println!("Interrupted. Type /quit to exit.");
                }
            }
            Err(ReadlineError::Eof) => {
                // Ctrl-D
                if !quiet {
                    println!("Goodbye!");
                }
                break;
            }
            Err(err) => {
                eprintln!("Error: {:?}", err);
                break;
            }
        }
    }

    // Save history
    if let Some(parent) = history_path.parent() {
        let _ = std::fs::create_dir_all(parent);
    }
    let _ = rl.save_history(&history_path);

    Ok(())
}

/// Check if the assistant's response suggests code generation would be helpful
fn should_offer_generation(response: &str) -> bool {
    let lower = response.to_lowercase();
    let generation_hints = [
        "would you like me to generate",
        "i can create",
        "shall i implement",
        "i'll create",
        "let me create",
        "here's the code",
        "here is the code",
        "```",
    ];
    generation_hints.iter().any(|hint| lower.contains(hint))
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

async fn cmd_features(db: &Database, action: FeatureAction, quiet: bool) -> anyhow::Result<()> {
    // Get the most recent project as the active project
    let project_repo = project::ProjectRepository::new(db);
    let projects = project_repo.list(None).await?;
    let active_project = projects.first().ok_or_else(|| {
        anyhow::anyhow!(
            "No projects found. Create one with: demiarch new <name> --framework <framework>"
        )
    })?;
    let project_id = &active_project.id;

    match action {
        FeatureAction::List { status } => {
            let status_enum = status.as_deref().and_then(feature::FeatureStatus::parse);
            let features = feature::list_with_db(db, project_id, status_enum).await?;
            if features.is_empty() {
                if !quiet {
                    println!("No features found for project '{}'.", active_project.name);
                    println!("\nCreate one with: demiarch features create <title>");
                }
            } else {
                if !quiet {
                    println!("Features for '{}' ({}):", active_project.name, project_id);
                }
                for f in features {
                    let status_icon = match f.status {
                        feature::FeatureStatus::Backlog => "○",
                        feature::FeatureStatus::Todo => "◐",
                        feature::FeatureStatus::InProgress => "◑",
                        feature::FeatureStatus::Review => "◕",
                        feature::FeatureStatus::Done => "●",
                    };
                    println!(
                        "  {} [{}] {} (P{})",
                        status_icon,
                        &f.id[..8],
                        f.title,
                        f.priority
                    );
                }
            }
        }
        FeatureAction::Show { id } => {
            let repo = feature::FeatureRepository::new(db);
            if let Some(f) = repo.get(&id).await? {
                println!("Feature: {}", f.title);
                println!("  ID: {}", f.id);
                println!("  Project: {}", f.project_id);
                println!("  Status: {}", f.status.as_str());
                println!("  Priority: {}", f.priority);
                if let Some(desc) = &f.description {
                    println!("  Description: {}", desc);
                }
                if let Some(criteria) = &f.acceptance_criteria {
                    println!("  Acceptance Criteria: {}", criteria);
                }
                if let Some(labels) = &f.labels {
                    println!("  Labels: {}", labels.join(", "));
                }
                println!("  Created: {}", f.created_at);
                println!("  Updated: {}", f.updated_at);
            } else {
                println!("Feature not found: {}", id);
            }
        }
        FeatureAction::Create { title, phase } => {
            let f = feature::create_with_db(db, project_id, &title, None, phase.as_deref()).await?;
            if !quiet {
                println!("Feature created: {} ({})", f.title, &f.id[..8]);
                println!(
                    "  Project: {} ({})",
                    active_project.name,
                    &active_project.id[..8]
                );
                println!(
                    "\nNext: Run `demiarch generate \"{}\"` to generate code.",
                    f.title
                );
            }
        }
        FeatureAction::Update { id, status } => {
            let status_enum = status.as_deref().and_then(feature::FeatureStatus::parse);
            feature::update_with_db(db, &id, status_enum, None).await?;
            if !quiet {
                println!("Feature '{}' updated.", id);
            }
        }
        FeatureAction::Delete { id } => {
            feature::delete_with_db(db, &id).await?;
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

async fn cmd_skills(db: &Database, action: SkillAction, quiet: bool) -> anyhow::Result<()> {
    use demiarch_core::skills::{LearnedSkill, SkillCategory, SkillPattern, SkillsManager};

    let manager = SkillsManager::with_database(db.clone());

    match action {
        SkillAction::List { category } => {
            let cat = category.as_deref().map(SkillCategory::parse);
            let skills = manager.list(cat, None).await?;
            if skills.is_empty() {
                if !quiet {
                    println!("No skills learned yet.");
                }
                return Ok(());
            }

            if !quiet {
                println!("Skills ({}):", skills.len());
            }
            for skill in skills {
                println!(
                    "- {} [{}] (used {}x)",
                    skill.name,
                    skill.category.as_str(),
                    skill.usage_stats.times_used
                );
            }
        }
        SkillAction::Search { query } => {
            let skills = manager.search(&query).await?;
            if skills.is_empty() {
                if !quiet {
                    println!("No matching skills for query '{}'.", query);
                }
            } else {
                if !quiet {
                    println!("Found {} matching skills:", skills.len());
                }
                for skill in skills {
                    println!("- {} [{}]", skill.name, skill.category.as_str());
                }
            }
        }
        SkillAction::Extract { description } => {
            let desc = description.unwrap_or_else(|| "Manually added skill".to_string());
            let mut skill = LearnedSkill::new(
                desc.clone(),
                desc.clone(),
                SkillCategory::Other,
                SkillPattern::technique(desc.clone()),
            )
            .with_tags(vec!["manual".into()]);
            manager.save(&skill).await?;
            // Record immediate usage to populate stats slightly
            skill.record_usage(true);
            manager.save(&skill).await?;
            if !quiet {
                println!("Saved manual skill '{}'", skill.name);
            }
        }
        SkillAction::Delete { id } => {
            let removed = manager.delete(&id).await?;
            if !quiet {
                if removed {
                    println!("Deleted skill {}", id);
                } else {
                    println!("Skill {} not found", id);
                }
            }
        }
        SkillAction::Stats => {
            let stats = manager.stats().await?;
            if !quiet {
                println!("Skill Statistics:");
                println!("  Total skills: {}", stats.total_skills);
                println!("  Total uses: {}", stats.total_uses);
                println!("  High-confidence: {}", stats.high_confidence_skills);
                println!("  By category:");
                for (cat, count) in stats.skills_by_category {
                    println!("    - {}: {}", cat.as_str(), count);
                }
            }
        }
        SkillAction::Show { id } => match manager.get(&id).await? {
            Some(skill) => {
                manager.record_usage(&id, true).await.ok();
                let skill = manager.get(&id).await?.unwrap_or(skill);
                if quiet {
                    println!("{}", serde_json::to_string_pretty(&skill)?);
                } else {
                    println!("{}", skill.name);
                    println!("  ID: {}", skill.id);
                    println!("  Category: {}", skill.category.as_str());
                    println!("  Description: {}", skill.description);
                    println!("  Tags: {}", skill.tags.join(", "));
                    println!("  Confidence: {}", skill.confidence.as_str());
                    println!("  Pattern: {}", skill.pattern.pattern_type.as_str());
                    println!("  Template:\n{}", indent(&skill.pattern.template, 4));
                    if !skill.pattern.variables.is_empty() {
                        println!("  Variables:");
                        for v in &skill.pattern.variables {
                            println!("    - {}: {}", v.name, v.description);
                        }
                    }
                    if !skill.pattern.applicability.is_empty() {
                        println!(
                            "  Applicability: {}",
                            skill.pattern.applicability.join("; ")
                        );
                    }
                    if !skill.pattern.limitations.is_empty() {
                        println!("  Limitations: {}", skill.pattern.limitations.join("; "));
                    }
                    println!(
                        "  Usage: {} used, {} successes, {} failures",
                        skill.usage_stats.times_used,
                        skill.usage_stats.success_count,
                        skill.usage_stats.failure_count
                    );
                    if let Some(last) = skill.usage_stats.last_used_at {
                        println!("  Last used: {}", last.to_rfc3339());
                    }
                }
            }
            None => println!("Skill not found: {}", id),
        },
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

async fn cmd_context(db: &Database, action: ContextAction, quiet: bool) -> anyhow::Result<()> {
    use demiarch_core::domain::memory::PersistentMemoryStore;

    let project_repo = project::ProjectRepository::new(db);
    let active_project = project_repo
        .list(None)
        .await?
        .into_iter()
        .find(|p| p.path.is_some())
        .ok_or_else(|| {
            anyhow::anyhow!("No projects found. Create one with: demiarch new <name>")
        })?;

    let store = PersistentMemoryStore::new(db.pool().clone());
    let manager = ContextManager::new().with_persistent_store(store);

    match action {
        ContextAction::Stats { project } => {
            let project_id = project.unwrap_or(active_project.id.clone());
            let stats = if let Some(p) = manager.persistent() {
                p.stats(Some(&project_id)).await?
            } else {
                Default::default()
            };

            if !quiet {
                println!("Context Statistics (project: {}):", project_id);
                println!("  Total entries: {}", stats.stats.total_records);
                println!("  Total tokens: {}", stats.total_tokens);
                if let Some(oldest) = stats.stats.oldest_at {
                    println!("  Oldest: {}", oldest);
                }
                if let Some(newest) = stats.stats.newest_at {
                    println!("  Newest: {}", newest);
                }
            }
        }
        ContextAction::Search { query, level: _ } => {
            let results = if let Some(p) = manager.persistent() {
                p.recall(
                    Some(&active_project.id),
                    RecallQuery {
                        query: query.clone(),
                        ..Default::default()
                    },
                )
                .await?
            } else {
                Vec::new()
            };

            if results.is_empty() {
                if !quiet {
                    println!("No matching context found for '{}'.", query);
                }
            } else {
                if !quiet {
                    println!("Context matches ({}):", results.len());
                }
                for rec in results {
                    println!("- {} | {}", rec.created_at.to_rfc3339(), rec.index_summary);
                }
            }
        }
        ContextAction::Prune {
            older_than,
            dry_run,
        } => {
            let days = older_than.unwrap_or(30);
            let cutoff = chrono::Utc::now() - chrono::Duration::days(days as i64);
            let removed = if let Some(p) = manager.persistent() {
                p.prune(Some(&active_project.id), cutoff, dry_run).await?
            } else {
                0usize
            };
            if !quiet {
                if dry_run {
                    println!(
                        "Dry run: would remove {} entries older than {} days",
                        removed, days
                    );
                } else {
                    println!(
                        "Removed {} context entries older than {} days",
                        removed, days
                    );
                }
            }
        }
        ContextAction::Rebuild { project } => {
            let project_id = project.unwrap_or(active_project.id.clone());
            let updated = if let Some(p) = manager.persistent() {
                p.rebuild(Some(&project_id)).await?
            } else {
                0usize
            };
            if !quiet {
                println!(
                    "Rebuilt {} context entries for project {}",
                    updated, project_id
                );
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

async fn cmd_sync(db: &Database, action: SyncAction, quiet: bool) -> anyhow::Result<()> {
    // Resolve the active project (prefer current directory, fallback to most recent with a path)
    let current_dir = std::env::current_dir()?;
    let project = if let Some(p) = project::find_by_directory(db, &current_dir)
        .await
        .map_err(|e| anyhow::anyhow!("Failed to detect project: {}", e))?
    {
        p
    } else {
        let repo = project::ProjectRepository::new(db);
        repo.list(None)
            .await
            .map_err(|e| anyhow::anyhow!("Failed to list projects: {}", e))?
            .into_iter()
            .find(|p| p.path.is_some())
            .ok_or_else(|| {
                anyhow::anyhow!(
                    "No projects registered. Run `demiarch new` or `demiarch init` in your project directory first."
                )
            })?
    };

    let project_path = project.path.clone().ok_or_else(|| {
        anyhow::anyhow!(
            "Project '{}' has no saved path. Re-run `demiarch init` inside the project directory to register it.",
            project.name
        )
    })?;
    let project_dir = std::path::PathBuf::from(&project_path);

    match action {
        SyncAction::Flush => {
            if !quiet {
                println!(
                    "Flushing SQLite to JSONL for project '{}' ({}):",
                    project.name,
                    project_dir.display()
                );
            }

            let result = storage::export_to_jsonl(db.pool(), &project_dir)
                .await
                .map_err(|e| anyhow::anyhow!("{}", e))?;

            if !quiet {
                println!(
                    "  Wrote {} records to {}",
                    result.metadata.total_records,
                    result.sync_dir.display()
                );
                let mut counts: Vec<_> = result.metadata.record_counts.iter().collect();
                counts.sort_by(|a, b| a.0.cmp(b.0));
                for (table, count) in counts {
                    println!("    {:<24} {}", table, count);
                }
                println!(
                    "  Exported at: {}",
                    result.metadata.exported_at.to_rfc3339()
                );
            }
        }
        SyncAction::Import => {
            let sync_dir = project_dir.join(storage::SYNC_DIR);
            if !sync_dir.exists() {
                return Err(anyhow::anyhow!(
                    "No sync directory found at {}. Run `demiarch sync flush` first.",
                    sync_dir.display()
                ));
            }

            if !quiet {
                println!("Importing JSONL from {}...", sync_dir.display());
            }

            let result = storage::import_from_jsonl(db.pool(), &project_dir)
                .await
                .map_err(|e| anyhow::anyhow!("{}", e))?;

            if !quiet {
                println!("  Imported {} records", result.total_records);
                let mut counts: Vec<_> = result.record_counts.iter().collect();
                counts.sort_by(|a, b| a.0.cmp(b.0));
                for (table, count) in counts {
                    println!("    {:<24} {}", table, count);
                }
                if !result.warnings.is_empty() {
                    println!("Warnings:");
                    for warn in &result.warnings {
                        println!("  - {}", warn);
                    }
                }
            }
        }
        SyncAction::Status => {
            let status = storage::check_sync_status(db.pool(), &project_dir)
                .await
                .map_err(|e| anyhow::anyhow!("{}", e))?;

            if !quiet {
                println!(
                    "Sync status for '{}' ({}):",
                    project.name,
                    project_dir.display()
                );
                println!(
                    "  State: {}",
                    if status.dirty {
                        "needs export"
                    } else {
                        "clean"
                    }
                );
                println!(
                    "  Last sync: {}",
                    status
                        .last_sync_at
                        .as_deref()
                        .unwrap_or("never (no export yet)")
                );
                println!("  Pending changes: {}", status.pending_changes);
                println!("  Note: {}", status.message);
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

fn indent(text: &str, spaces: usize) -> String {
    let pad = " ".repeat(spaces);
    text.lines()
        .map(|line| format!("{}{}", pad, line))
        .collect::<Vec<_>>()
        .join("\n")
}

fn format_history_for_context(messages: &[chat::ChatMessage]) -> String {
    messages
        .iter()
        .map(|m| format!("[{}] {}", m.role.as_str(), m.content))
        .collect::<Vec<_>>()
        .join("\n")
}

// ============================================================================
// Knowledge Graph Commands
// ============================================================================

async fn cmd_graph(db: &Database, action: GraphAction, quiet: bool) -> anyhow::Result<()> {
    let pool = db.pool();

    match action {
        GraphAction::Stats { detailed } => {
            let stats = graph::get_stats(pool).await?;

            if !quiet {
                println!("Knowledge Graph Statistics");
                println!("==========================");
                println!();
                println!("Total Entities:      {}", stats.total_entities);
                println!("Total Relationships: {}", stats.total_relationships);
                println!("Linked Skills:       {}", stats.linked_skills);
                println!(
                    "Average Confidence:  {:.1}%",
                    stats.average_confidence * 100.0
                );

                if detailed && !stats.entities_by_type.is_empty() {
                    println!();
                    println!("Entities by Type:");
                    let mut sorted: Vec<_> = stats.entities_by_type.iter().collect();
                    sorted.sort_by(|a, b| b.1.cmp(a.1));
                    for (entity_type, count) in sorted {
                        println!("  {:20} {}", entity_type.as_str(), count);
                    }
                }

                if detailed && !stats.relationships_by_type.is_empty() {
                    println!();
                    println!("Relationships by Type:");
                    let mut sorted: Vec<_> = stats.relationships_by_type.iter().collect();
                    sorted.sort_by(|a, b| b.1.cmp(a.1));
                    for (rel_type, count) in sorted {
                        println!("  {:20} {}", rel_type.as_str(), count);
                    }
                }
            }
        }

        GraphAction::Explore {
            entity,
            depth,
            relationship,
            tree,
        } => {
            let rel_filter = relationship
                .as_ref()
                .and_then(|r| RelationshipType::parse(r));

            match graph::explore_entity(pool, &entity, depth, rel_filter).await? {
                Some(result) => {
                    if !quiet {
                        if tree {
                            println!("{}", graph::format_explore_tree(&result, depth));
                        } else {
                            println!("{}", graph::format_explore_list(&result));
                        }
                    }
                }
                None => {
                    if !quiet {
                        println!("Entity '{}' not found.", entity);
                        println!();
                        println!("Try searching for entities: demiarch graph search <query>");
                    }
                }
            }
        }

        GraphAction::Search { query, limit } => {
            let results = graph::search_entities(pool, &query, limit).await?;

            if results.is_empty() {
                if !quiet {
                    println!("No entities found matching '{}'.", query);
                }
            } else {
                if !quiet {
                    println!("Found {} entities:", results.len());
                    println!();
                }
                for entity in results {
                    let desc = entity
                        .description
                        .as_ref()
                        .map(|d| format!(": {}", truncate_str(d, 60)))
                        .unwrap_or_default();
                    println!(
                        "  {} ({}) [conf: {:.0}%]{}",
                        entity.name,
                        entity.entity_type.as_str(),
                        entity.confidence * 100.0,
                        desc
                    );
                }
            }
        }

        GraphAction::List { entity_type, limit } => {
            let parsed_type = parse_entity_type(&entity_type)?;
            let entities = graph::list_entities_by_type(pool, parsed_type).await?;

            if entities.is_empty() {
                if !quiet {
                    println!("No {} entities found.", entity_type);
                }
            } else {
                let display_count = entities.len().min(limit);
                if !quiet {
                    println!("{} entities ({}):", entity_type, entities.len());
                    println!();
                }
                for entity in entities.iter().take(limit) {
                    let desc = entity
                        .description
                        .as_ref()
                        .map(|d| format!(": {}", truncate_str(d, 50)))
                        .unwrap_or_default();
                    println!(
                        "  {} [conf: {:.0}%]{}",
                        entity.name,
                        entity.confidence * 100.0,
                        desc
                    );
                }
                if entities.len() > limit && !quiet {
                    println!();
                    println!("  ... and {} more", entities.len() - display_count);
                }
            }
        }
    }

    Ok(())
}

/// Parse entity type from string
fn parse_entity_type(s: &str) -> anyhow::Result<EntityType> {
    EntityType::parse(s).ok_or_else(|| {
        anyhow::anyhow!(
            "Unknown entity type '{}'. Valid types: library, concept, pattern, technique, \
             framework, language, tool, domain, api, data_structure, algorithm",
            s
        )
    })
}

/// Truncate a string to max length with ellipsis
fn truncate_str(s: &str, max_len: usize) -> String {
    if s.len() <= max_len {
        s.to_string()
    } else {
        format!("{}...", &s[..max_len.saturating_sub(3)])
    }
}

// ============================================================================
// Image Generation Commands
// ============================================================================

async fn cmd_image(action: ImageAction, quiet: bool) -> anyhow::Result<()> {
    match action {
        ImageAction::Generate {
            prompt,
            output,
            size,
            style,
            model,
            negative,
            seed,
        } => {
            if !quiet {
                println!("Generating image...");
                println!("  Prompt: {}", truncate_str(&prompt, 60));
                if let Some(s) = &style {
                    println!("  Style: {}", s);
                }
                println!();
            }

            let output_path =
                image::generate(prompt, output, Some(size), style, model, negative, seed)
                    .await
                    .map_err(|e| anyhow::anyhow!("{}", e))?;

            if !quiet {
                println!("Image saved to: {}", output_path.display());
            } else {
                println!("{}", output_path.display());
            }
        }

        ImageAction::Transform {
            input,
            prompt,
            output,
            strength,
            model,
        } => {
            if !quiet {
                println!("Transforming image...");
                println!("  Input: {}", input.display());
                println!("  Prompt: {}", truncate_str(&prompt, 60));
                println!("  Strength: {:.1}", strength);
                println!();
            }

            let output_path = image::transform(input, prompt, output, Some(strength), model)
                .await
                .map_err(|e| anyhow::anyhow!("{}", e))?;

            if !quiet {
                println!("Transformed image saved to: {}", output_path.display());
            } else {
                println!("{}", output_path.display());
            }
        }

        ImageAction::Upscale {
            input,
            scale,
            output,
            model,
        } => {
            if !quiet {
                println!("Upscaling image...");
                println!("  Input: {}", input.display());
                println!("  Scale: {}x", scale);
                println!();
            }

            let output_path = image::upscale(input, scale, output, model)
                .await
                .map_err(|e| anyhow::anyhow!("{}", e))?;

            if !quiet {
                println!("Upscaled image saved to: {}", output_path.display());
            } else {
                println!("{}", output_path.display());
            }
        }

        ImageAction::Inpaint {
            input,
            mask,
            prompt,
            output,
            model,
        } => {
            if !quiet {
                println!("Inpainting image...");
                println!("  Input: {}", input.display());
                println!("  Mask: {}", mask.display());
                println!("  Prompt: {}", truncate_str(&prompt, 60));
                println!();
            }

            let output_path = image::inpaint(input, mask, prompt, output, model)
                .await
                .map_err(|e| anyhow::anyhow!("{}", e))?;

            if !quiet {
                println!("Inpainted image saved to: {}", output_path.display());
            } else {
                println!("{}", output_path.display());
            }
        }

        ImageAction::Models => {
            let models = image::list_models();

            if !quiet {
                println!("Available Image Generation Models");
                println!("==================================");
                println!();
            }

            for model in models {
                println!("{}", model.name);
                println!("  ID: {}", model.id);
                println!("  {}", model.description);
                println!("  Capabilities: {}", model.capabilities_string());
                println!("  Cost: {}", model.cost_string());
                println!();
            }
        }
    }

    Ok(())
}
