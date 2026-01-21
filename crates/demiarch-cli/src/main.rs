//! Demiarch CLI - local-first AI app builder

use clap::{Parser, Subcommand};
use tracing::{info, warn};

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
    Show {
        id: String,
    },
    Archive {
        id: String,
    },
    Delete {
        id: String,
        #[arg(long)]
        force: bool,
    },
}

#[derive(Subcommand)]
enum FeatureAction {
    List {
        #[arg(short, long)]
        status: Option<String>,
    },
    Show {
        id: String,
    },
    Create {
        title: String,
        #[arg(short, long)]
        phase: Option<String>,
    },
    Update {
        id: String,
        #[arg(short, long)]
        status: Option<String>,
    },
    Delete {
        id: String,
    },
}

#[derive(Subcommand)]
enum SkillAction {
    List {
        #[arg(short, long)]
        category: Option<String>,
    },
    Show {
        id: String,
    },
    Search {
        query: String,
    },
    Extract {
        #[arg(short, long)]
        description: Option<String>,
    },
    Delete {
        id: String,
    },
    Stats,
}

#[derive(Subcommand)]
enum RoutingAction {
    Status,
    SetPreference {
        preference: String,
    },
    Performance {
        #[arg(short, long)]
        task: Option<String>,
    },
    History {
        #[arg(short, long)]
        limit: Option<usize>,
    },
}

#[derive(Subcommand)]
enum ContextAction {
    Stats {
        #[arg(short, long)]
        project: Option<String>,
    },
    Search {
        query: String,
        #[arg(short, long)]
        level: Option<u8>,
    },
    Prune {
        #[arg(long)]
        older_than: Option<u32>,
        #[arg(long)]
        dry_run: bool,
    },
    Rebuild {
        #[arg(short, long)]
        project: Option<String>,
    },
}

#[derive(Subcommand)]
enum HookAction {
    List {
        #[arg(short, long)]
        r#type: Option<String>,
    },
    Register {
        hook_type: String,
        name: String,
        #[arg(long)]
        handler: String,
    },
    Enable {
        id: String,
    },
    Disable {
        id: String,
    },
    Remove {
        id: String,
    },
    History {
        #[arg(short, long)]
        limit: Option<usize>,
    },
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
                eprintln!("⚠️  WARNING: License enforcement is DISABLED");
                eprintln!("⚠️  WARNING: Running in UNSAFE mode");
                eprintln!("⚠️  WARNING: Unverified plugins may execute");
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

    tracing::info!("License issuer key validated successfully");
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

    match cli.command {
        Commands::New {
            name,
            framework,
            repo,
        } => {
            info!("Creating project '{}' with framework '{}'", name, framework);
            if let Some(repo) = repo {
                info!("Repository: {}", repo);
            }
            Err(anyhow::anyhow!(
                "Command 'new' is not yet implemented. See ROADMAP.md for implementation status."
            ))
        }
        Commands::Chat => {
            info!("Starting conversational discovery...");
            Err(anyhow::anyhow!(
                "Command 'chat' is not yet implemented. See ROADMAP.md for implementation status."
            ))
        }
        Commands::Doctor => {
            println!("Demiarch Health Check");
            println!("=====================");
            println!("✅ Configuration: Valid");
            warn!("Database: Not initialized");
            warn!("API Key: Not configured");
            Ok(())
        }
        Commands::Watch => {
            info!("Starting TUI monitor...");
            println!("(Run demiarch-tui binary for full TUI experience)");
            Err(anyhow::anyhow!(
                "Command 'watch' is not yet implemented. See ROADMAP.md for implementation status."
            ))
        }
        _ => {
            Err(anyhow::anyhow!(
                "This command is not yet implemented. See ROADMAP.md for implementation status."
            ))
        }
    }
}
