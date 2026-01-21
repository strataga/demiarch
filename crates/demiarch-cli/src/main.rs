//! Demiarch CLI - local-first AI app builder

use clap::{Parser, Subcommand};
use demiarch_core::commands::{feature, project};
use demiarch_core::config::Config;
use demiarch_core::cost::CostTracker;
use demiarch_core::storage::{Database, DatabaseManager};
use tracing::{info, warn};

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

    /// Generate code for a feature
    Generate {
        /// Feature ID
        feature_id: String,
        /// Dry run (don't write files)
        #[arg(short, long)]
        dry_run: bool,
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

    /// Configuration management
    Config {
        #[command(subcommand)]
        action: ConfigAction,
    },

    /// Run health check
    Doctor,

    /// Open TUI monitor (watch mode)
    Watch,
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
            feature_id,
            dry_run,
        } => cmd_generate(&feature_id, dry_run, cli.quiet).await,

        Commands::Skills { action } => cmd_skills(action, cli.quiet).await,

        Commands::Routing { action } => cmd_routing(action, cli.quiet).await,

        Commands::Context { action } => cmd_context(action, cli.quiet).await,

        Commands::Hooks { action } => cmd_hooks(action, cli.quiet).await,

        Commands::Costs { project } => cmd_costs(project.as_deref(), cli.quiet).await,

        Commands::Sync { action } => cmd_sync(action, cli.quiet).await,

        Commands::Config { action } => cmd_config(action, cli.quiet),

        Commands::Doctor => cmd_doctor(cli.quiet).await,

        Commands::Watch => cmd_watch(cli.quiet),
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

async fn cmd_generate(feature_id: &str, dry_run: bool, quiet: bool) -> anyhow::Result<()> {
    if !quiet {
        if dry_run {
            println!("Dry run: Would generate code for feature '{}'", feature_id);
        } else {
            println!("Generating code for feature '{}'...", feature_id);
        }
        println!("(Code generation not yet implemented)");
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
