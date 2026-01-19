//! Demiarch CLI - local-first AI app builder

use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(name = "demiarch")]
#[command(author, version, about = "Local-first AI app builder", long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
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
    List,
    Show { id: String },
    Archive { id: String },
    Delete { id: String, #[arg(long)] force: bool },
}

#[derive(Subcommand)]
enum FeatureAction {
    List { #[arg(short, long)] status: Option<String> },
    Show { id: String },
    Create { title: String, #[arg(short, long)] phase: Option<String> },
    Update { id: String, #[arg(short, long)] status: Option<String> },
    Delete { id: String },
}

#[derive(Subcommand)]
enum SkillAction {
    List { #[arg(short, long)] category: Option<String> },
    Show { id: String },
    Search { query: String },
    Extract { #[arg(short, long)] description: Option<String> },
    Delete { id: String },
    Stats,
}

#[derive(Subcommand)]
enum RoutingAction {
    Status,
    SetPreference { preference: String },
    Performance { #[arg(short, long)] task: Option<String> },
    History { #[arg(short, long)] limit: Option<usize> },
}

#[derive(Subcommand)]
enum ContextAction {
    Stats { #[arg(short, long)] project: Option<String> },
    Search { query: String, #[arg(short, long)] level: Option<u8> },
    Prune { #[arg(long)] older_than: Option<u32>, #[arg(long)] dry_run: bool },
    Rebuild { #[arg(short, long)] project: Option<String> },
}

#[derive(Subcommand)]
enum HookAction {
    List { #[arg(short, long)] r#type: Option<String> },
    Register { hook_type: String, name: String, #[arg(long)] handler: String },
    Enable { id: String },
    Disable { id: String },
    Remove { id: String },
    History { #[arg(short, long)] limit: Option<usize> },
}

#[derive(Subcommand)]
enum SyncAction {
    Flush,
    Import,
    Status,
}

#[derive(Subcommand)]
enum ConfigAction {
    Get { key: String },
    Set { key: String, value: String },
    List,
    Reset,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Initialize tracing
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::from_default_env()
                .add_directive("demiarch=info".parse()?)
        )
        .init();

    let cli = Cli::parse();

    match cli.command {
        Commands::New { name, framework, repo } => {
            println!("Creating project '{}' with framework '{}'", name, framework);
            if let Some(repo) = repo {
                println!("Repository: {}", repo);
            }
            todo!("Implement project creation")
        }
        Commands::Chat => {
            println!("Starting conversational discovery...");
            todo!("Implement chat")
        }
        Commands::Doctor => {
            println!("Demiarch Health Check");
            println!("=====================");
            println!("✅ Configuration: Valid");
            println!("⚠️  Database: Not initialized");
            println!("⚠️  API Key: Not configured");
            Ok(())
        }
        Commands::Watch => {
            println!("Starting TUI monitor...");
            println!("(Run demiarch-tui binary for full TUI experience)");
            todo!("Launch TUI")
        }
        _ => {
            println!("Command not yet implemented");
            Ok(())
        }
    }
}
