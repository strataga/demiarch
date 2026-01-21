//! Code generation commands
//!
//! Generates code from natural language descriptions using LLM integration.
//! Supports file extraction, dry-run mode, and cost tracking.

use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;

use tracing::{debug, info};

use crate::config::Config;
use crate::cost::CostTracker;
use crate::error::{Error, Result};
use crate::llm::{LlmClient, Message};

/// Result of code generation
#[derive(Debug, Clone)]
pub struct GenerationResult {
    /// Number of new files created
    pub files_created: usize,
    /// Number of existing files modified
    pub files_modified: usize,
    /// Total tokens used for generation
    pub tokens_used: u32,
    /// Estimated cost in USD
    pub cost_usd: f64,
    /// Generated files with their contents
    pub files: Vec<GeneratedFile>,
}

/// A single generated file
#[derive(Debug, Clone)]
pub struct GeneratedFile {
    /// Path where the file should be created/modified
    pub path: PathBuf,
    /// Generated content
    pub content: String,
    /// Whether this is a new file or modification
    pub is_new: bool,
    /// Language/file type
    pub language: Option<String>,
}

/// Code generator that uses LLM to generate code from descriptions
pub struct CodeGenerator {
    llm_client: LlmClient,
    #[allow(dead_code)]
    config: Config,
}

impl CodeGenerator {
    /// Create a new code generator with configuration
    pub fn new(config: Config, cost_tracker: Option<Arc<CostTracker>>) -> Result<Self> {
        let api_key = config
            .llm
            .resolved_api_key()
            .map_err(|e| Error::ConfigError(e.to_string()))?
            .ok_or_else(|| {
                Error::LLMError(
                    "API key not configured. Set DEMIARCH_API_KEY or OPENROUTER_API_KEY environment variable.".to_string()
                )
            })?;

        let mut builder = LlmClient::builder()
            .config(config.llm.clone())
            .api_key(api_key);

        if let Some(tracker) = cost_tracker {
            builder = builder.cost_tracker(tracker);
        }

        let llm_client = builder.build()?;

        Ok(Self { llm_client, config })
    }

    /// Generate code from a natural language description
    pub async fn generate(&self, description: &str, dry_run: bool) -> Result<GenerationResult> {
        info!(description = %description, dry_run = %dry_run, "Starting code generation");

        let messages = self.build_messages(description);

        debug!(message_count = messages.len(), "Sending request to LLM");

        let response = self.llm_client.complete_with_fallback(messages).await?;

        info!(
            tokens = response.tokens_used,
            model = %response.model,
            "Received LLM response"
        );

        let files = self.parse_generated_files(&response.content)?;

        let files_created = files.iter().filter(|f| f.is_new).count();
        let files_modified = files.len() - files_created;

        // Estimate cost (rough approximation based on token counts)
        // Claude 3.5 Sonnet: ~$3/1M input, ~$15/1M output
        let cost_usd = estimate_cost(
            &response.model,
            response.input_tokens,
            response.output_tokens,
        );

        let result = GenerationResult {
            files_created,
            files_modified,
            tokens_used: response.tokens_used,
            cost_usd,
            files,
        };

        if !dry_run {
            self.write_files(&result.files)?;
        } else {
            debug!("Dry run mode - skipping file writes");
        }

        Ok(result)
    }

    /// Build the message sequence for code generation
    fn build_messages(&self, description: &str) -> Vec<Message> {
        vec![
            Message::system(SYSTEM_PROMPT),
            Message::user(format!(
                "Generate code for the following requirement:\n\n{}",
                description
            )),
        ]
    }

    /// Parse the LLM response to extract generated files
    fn parse_generated_files(&self, content: &str) -> Result<Vec<GeneratedFile>> {
        let mut files = Vec::new();
        let mut current_file: Option<FileBuilder> = None;

        for line in content.lines() {
            // Check for file path marker (various formats)
            if let Some(path) = extract_file_path(line) {
                // Save previous file if exists
                if let Some(builder) = current_file.take()
                    && let Some(file) = builder.build()
                {
                    files.push(file);
                }

                // Start new file
                current_file = Some(FileBuilder::new(path));
                continue;
            }

            // Check for code fence start with language
            if line.starts_with("```") {
                if let Some(ref mut builder) = current_file {
                    if builder.in_code_block {
                        // End of code block
                        builder.in_code_block = false;
                    } else {
                        // Start of code block
                        builder.in_code_block = true;
                        let lang = line.trim_start_matches('`').trim();
                        if !lang.is_empty() {
                            builder.language = Some(lang.to_string());
                        }
                    }
                }
                continue;
            }

            // Add content if we're in a file
            if let Some(ref mut builder) = current_file
                && builder.in_code_block
            {
                builder.content.push_str(line);
                builder.content.push('\n');
            }
        }

        // Don't forget the last file
        if let Some(builder) = current_file
            && let Some(file) = builder.build()
        {
            files.push(file);
        }

        if files.is_empty() {
            // Fallback: try to extract any code blocks as a single file
            if let Some(file) = self.extract_single_code_block(content) {
                files.push(file);
            }
        }

        Ok(files)
    }

    /// Fallback extraction for responses with just code blocks
    fn extract_single_code_block(&self, content: &str) -> Option<GeneratedFile> {
        let mut in_block = false;
        let mut language = None;
        let mut code = String::new();

        for line in content.lines() {
            if line.starts_with("```") {
                if in_block {
                    // End of block
                    break;
                } else {
                    // Start of block
                    in_block = true;
                    let lang = line.trim_start_matches('`').trim();
                    if !lang.is_empty() {
                        language = Some(lang.to_string());
                    }
                }
                continue;
            }

            if in_block {
                code.push_str(line);
                code.push('\n');
            }
        }

        if code.is_empty() {
            return None;
        }

        // Infer filename from language
        let ext = language.as_deref().and_then(language_to_extension);
        let filename = format!("generated{}", ext.unwrap_or(""));

        Some(GeneratedFile {
            path: PathBuf::from(filename),
            content: code.trim_end().to_string(),
            is_new: true,
            language,
        })
    }

    /// Write generated files to disk
    fn write_files(&self, files: &[GeneratedFile]) -> Result<()> {
        for file in files {
            info!(path = %file.path.display(), "Writing generated file");

            // Create parent directories if needed
            if let Some(parent) = file.path.parent()
                && !parent.as_os_str().is_empty()
            {
                std::fs::create_dir_all(parent).map_err(Error::Io)?;
            }

            std::fs::write(&file.path, &file.content).map_err(Error::Io)?;
        }
        Ok(())
    }
}

/// Helper struct for building a GeneratedFile
struct FileBuilder {
    path: PathBuf,
    content: String,
    language: Option<String>,
    in_code_block: bool,
}

impl FileBuilder {
    fn new(path: PathBuf) -> Self {
        Self {
            path,
            content: String::new(),
            language: None,
            in_code_block: false,
        }
    }

    fn build(self) -> Option<GeneratedFile> {
        let content = self.content.trim_end().to_string();
        if content.is_empty() {
            return None;
        }

        Some(GeneratedFile {
            path: self.path,
            content,
            is_new: true,
            language: self.language,
        })
    }
}

/// Extract file path from a line (supports multiple formats)
fn extract_file_path(line: &str) -> Option<PathBuf> {
    let line = line.trim();

    // Format: `path/to/file.rs`
    if line.starts_with('`') && line.ends_with('`') && !line.starts_with("```") {
        let path = line.trim_matches('`').trim();
        if looks_like_path(path) {
            return Some(PathBuf::from(path));
        }
    }

    // Format: **path/to/file.rs** or **`path/to/file.rs`**
    if line.starts_with("**") && line.ends_with("**") {
        let inner = line.trim_start_matches("**").trim_end_matches("**");
        let path = inner.trim_matches('`').trim();
        if looks_like_path(path) {
            return Some(PathBuf::from(path));
        }
    }

    // Format: ### path/to/file.rs or ## path/to/file.rs
    if line.starts_with('#') {
        let path = line.trim_start_matches('#').trim().trim_matches('`');
        if looks_like_path(path) {
            return Some(PathBuf::from(path));
        }
    }

    // Format: File: path/to/file.rs or Filename: path/to/file.rs
    let prefixes = ["File:", "Filename:", "Path:"];
    for prefix in prefixes {
        if let Some(rest) = line.strip_prefix(prefix) {
            let path = rest.trim().trim_matches('`');
            if looks_like_path(path) {
                return Some(PathBuf::from(path));
            }
        }
    }

    None
}

/// Check if a string looks like a file path
fn looks_like_path(s: &str) -> bool {
    // Must have an extension or be a recognizable file
    if s.contains('.') {
        // Has extension
        let ext = s.rsplit('.').next().unwrap_or("");
        let common_exts = [
            "rs",
            "py",
            "js",
            "ts",
            "tsx",
            "jsx",
            "go",
            "java",
            "c",
            "cpp",
            "h",
            "hpp",
            "rb",
            "php",
            "swift",
            "kt",
            "scala",
            "clj",
            "ex",
            "exs",
            "erl",
            "hs",
            "ml",
            "fs",
            "cs",
            "vb",
            "lua",
            "r",
            "jl",
            "nim",
            "zig",
            "v",
            "html",
            "css",
            "scss",
            "sass",
            "less",
            "vue",
            "svelte",
            "json",
            "yaml",
            "yml",
            "toml",
            "xml",
            "md",
            "txt",
            "sql",
            "sh",
            "bash",
            "zsh",
            "fish",
            "ps1",
            "bat",
            "cmd",
            "dockerfile",
            "makefile",
            "gitignore",
            "env",
        ];
        return common_exts.iter().any(|&e| ext.eq_ignore_ascii_case(e));
    }

    // Special filenames without extension
    let special_files = [
        "Makefile",
        "Dockerfile",
        "Rakefile",
        "Gemfile",
        "Cargo.toml",
    ];
    special_files.contains(&s)
}

/// Map language identifier to file extension
fn language_to_extension(lang: &str) -> Option<&'static str> {
    let map: HashMap<&str, &str> = [
        ("rust", ".rs"),
        ("python", ".py"),
        ("javascript", ".js"),
        ("typescript", ".ts"),
        ("tsx", ".tsx"),
        ("jsx", ".jsx"),
        ("go", ".go"),
        ("java", ".java"),
        ("c", ".c"),
        ("cpp", ".cpp"),
        ("c++", ".cpp"),
        ("ruby", ".rb"),
        ("php", ".php"),
        ("swift", ".swift"),
        ("kotlin", ".kt"),
        ("scala", ".scala"),
        ("html", ".html"),
        ("css", ".css"),
        ("scss", ".scss"),
        ("json", ".json"),
        ("yaml", ".yaml"),
        ("toml", ".toml"),
        ("xml", ".xml"),
        ("markdown", ".md"),
        ("md", ".md"),
        ("sql", ".sql"),
        ("bash", ".sh"),
        ("shell", ".sh"),
        ("sh", ".sh"),
    ]
    .into_iter()
    .collect();

    map.get(lang.to_lowercase().as_str()).copied()
}

/// Estimate cost based on model and token counts
fn estimate_cost(model: &str, input_tokens: u32, output_tokens: u32) -> f64 {
    // Pricing per million tokens (approximate)
    let (input_price, output_price) = match model {
        m if m.contains("claude-3-5-sonnet") || m.contains("claude-sonnet-4") => (3.0, 15.0),
        m if m.contains("claude-3-5-haiku") || m.contains("claude-3-haiku") => (0.25, 1.25),
        m if m.contains("claude-3-opus") || m.contains("claude-opus-4") => (15.0, 75.0),
        m if m.contains("gpt-4o") => (2.5, 10.0),
        m if m.contains("gpt-4-turbo") => (10.0, 30.0),
        m if m.contains("gpt-3.5") => (0.5, 1.5),
        _ => (3.0, 15.0), // Default to sonnet-like pricing
    };

    let input_cost = (input_tokens as f64 / 1_000_000.0) * input_price;
    let output_cost = (output_tokens as f64 / 1_000_000.0) * output_price;

    input_cost + output_cost
}

/// System prompt for code generation
const SYSTEM_PROMPT: &str = r#"You are an expert software developer. Generate clean, well-documented, production-ready code based on the user's requirements.

## Output Format

For each file you generate, use this format:

**`path/to/filename.ext`**
```language
// file contents here
```

For example:
**`src/lib.rs`**
```rust
pub fn hello() -> &'static str {
    "Hello, world!"
}
```

## Guidelines

1. **Complete Code**: Generate fully working code, not snippets or pseudocode
2. **Best Practices**: Follow language-specific best practices and conventions
3. **Documentation**: Include appropriate comments and documentation
4. **Error Handling**: Implement proper error handling where appropriate
5. **Type Safety**: Use strong typing where the language supports it
6. **Tests**: Include unit tests when appropriate
7. **Security**: Follow secure coding practices, avoid hardcoded secrets

## File Organization

- Use appropriate directory structure for the project type
- Separate concerns into different files/modules
- Include configuration files if needed (e.g., Cargo.toml, package.json)

Be concise in explanations but thorough in code generation."#;

/// Generate code from a natural language description
///
/// This is the main entry point for the generate command.
pub async fn generate(description: &str, dry_run: bool) -> Result<GenerationResult> {
    let config = Config::load().map_err(|e| Error::ConfigError(e.to_string()))?;
    let cost_tracker = Arc::new(CostTracker::from_config(&config.cost));

    let generator = CodeGenerator::new(config, Some(cost_tracker))?;
    generator.generate(description, dry_run).await
}

/// Generate code with an existing database connection for cost tracking
pub async fn generate_with_tracker(
    description: &str,
    dry_run: bool,
    cost_tracker: Arc<CostTracker>,
) -> Result<GenerationResult> {
    let config = Config::load().map_err(|e| Error::ConfigError(e.to_string()))?;

    let generator = CodeGenerator::new(config, Some(cost_tracker))?;
    generator.generate(description, dry_run).await
}

/// Generate code with automatic checkpointing for code safety
///
/// This is the recommended entry point for code generation as it:
/// 1. Creates a checkpoint of the project state before generation
/// 2. Generates the code
/// 3. Returns the checkpoint ID along with the generation result
///
/// If `dry_run` is true, no checkpoint is created and no files are written.
pub async fn generate_with_checkpoint(
    project_id: uuid::Uuid,
    feature_id: Option<uuid::Uuid>,
    feature_name: &str,
    description: &str,
    dry_run: bool,
) -> Result<GenerationResultWithCheckpoint> {
    use crate::domain::recovery::{CheckpointManager, CheckpointSigner};
    use crate::storage::Database;

    let config = Config::load().map_err(|e| Error::ConfigError(e.to_string()))?;
    let cost_tracker = Arc::new(CostTracker::from_config(&config.cost));

    // Create checkpoint before generation (unless dry run)
    let checkpoint_id = if !dry_run {
        let db = Database::default()
            .await
            .map_err(|e| Error::Other(e.to_string()))?;
        let signer = CheckpointSigner::generate();
        let manager = CheckpointManager::new(db.pool().clone(), signer);

        let checkpoint = manager
            .create_before_generation(project_id, feature_id, feature_name)
            .await?;

        info!(
            checkpoint_id = %checkpoint.id,
            size = %checkpoint.display_size(),
            "Created checkpoint before code generation"
        );

        Some(checkpoint.id)
    } else {
        debug!("Dry run mode - skipping checkpoint creation");
        None
    };

    // Generate code
    let generator = CodeGenerator::new(config, Some(cost_tracker))?;
    let result = generator.generate(description, dry_run).await?;

    Ok(GenerationResultWithCheckpoint {
        result,
        checkpoint_id,
    })
}

/// Result of code generation with checkpoint information
#[derive(Debug, Clone)]
pub struct GenerationResultWithCheckpoint {
    /// The generation result
    pub result: GenerationResult,

    /// The checkpoint ID created before generation (None if dry-run)
    pub checkpoint_id: Option<uuid::Uuid>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_file_path_backticks() {
        assert_eq!(
            extract_file_path("`src/main.rs`"),
            Some(PathBuf::from("src/main.rs"))
        );
    }

    #[test]
    fn test_extract_file_path_bold() {
        assert_eq!(
            extract_file_path("**`src/lib.rs`**"),
            Some(PathBuf::from("src/lib.rs"))
        );
        assert_eq!(
            extract_file_path("**src/lib.rs**"),
            Some(PathBuf::from("src/lib.rs"))
        );
    }

    #[test]
    fn test_extract_file_path_heading() {
        assert_eq!(
            extract_file_path("### src/utils.rs"),
            Some(PathBuf::from("src/utils.rs"))
        );
        assert_eq!(
            extract_file_path("## `config.toml`"),
            Some(PathBuf::from("config.toml"))
        );
    }

    #[test]
    fn test_extract_file_path_prefix() {
        assert_eq!(
            extract_file_path("File: src/main.rs"),
            Some(PathBuf::from("src/main.rs"))
        );
        assert_eq!(
            extract_file_path("Filename: `test.py`"),
            Some(PathBuf::from("test.py"))
        );
    }

    #[test]
    fn test_extract_file_path_not_a_path() {
        assert_eq!(extract_file_path("This is some text"), None);
        assert_eq!(extract_file_path("```rust"), None);
        assert_eq!(extract_file_path("# A heading"), None);
    }

    #[test]
    fn test_looks_like_path() {
        assert!(looks_like_path("main.rs"));
        assert!(looks_like_path("src/lib.py"));
        assert!(looks_like_path("Makefile"));
        assert!(looks_like_path("test.tsx"));

        assert!(!looks_like_path("hello world"));
        assert!(!looks_like_path("rust"));
        assert!(!looks_like_path("no_extension"));
    }

    #[test]
    fn test_language_to_extension() {
        assert_eq!(language_to_extension("rust"), Some(".rs"));
        assert_eq!(language_to_extension("RUST"), Some(".rs"));
        assert_eq!(language_to_extension("python"), Some(".py"));
        assert_eq!(language_to_extension("typescript"), Some(".ts"));
        assert_eq!(language_to_extension("unknown"), None);
    }

    #[test]
    fn test_estimate_cost() {
        // 1000 tokens in, 500 out on Claude Sonnet
        let cost = estimate_cost("anthropic/claude-sonnet-4-20250514", 1000, 500);
        // Expected: (1000/1M * 3) + (500/1M * 15) = 0.003 + 0.0075 = 0.0105
        assert!((cost - 0.0105).abs() < 0.0001);
    }

    #[test]
    fn test_parse_generated_files() {
        // We can't easily test parse_generated_files without a full CodeGenerator
        // but we can test the helper functions it uses

        let content = r#"
Here's the code:

**`src/main.rs`**
```rust
fn main() {
    println!("Hello!");
}
```

**`src/lib.rs`**
```rust
pub fn greet() -> &'static str {
    "Hello"
}
```
"#;

        // Test that the file path extraction works on the expected lines
        for line in content.lines() {
            if line.contains("src/main.rs") {
                assert!(extract_file_path(line).is_some());
            }
            if line.contains("src/lib.rs") {
                assert!(extract_file_path(line).is_some());
            }
        }
    }

    #[test]
    fn test_file_builder() {
        let mut builder = FileBuilder::new(PathBuf::from("test.rs"));
        builder.in_code_block = true;
        builder.content.push_str("fn main() {}\n");
        builder.language = Some("rust".to_string());

        let file = builder.build().unwrap();
        assert_eq!(file.path, PathBuf::from("test.rs"));
        assert_eq!(file.content, "fn main() {}");
        assert_eq!(file.language, Some("rust".to_string()));
        assert!(file.is_new);
    }

    #[test]
    fn test_file_builder_empty() {
        let builder = FileBuilder::new(PathBuf::from("empty.rs"));
        assert!(builder.build().is_none());
    }
}
