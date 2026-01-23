//! Code block extraction utilities
//!
//! Provides utilities for extracting code blocks from markdown-formatted text,
//! extracting file paths from LLM responses, and determining file extensions
//! for various programming languages.

use std::path::PathBuf;

/// A code block extracted from markdown
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CodeBlock {
    /// The language identifier (e.g., "rust", "python")
    pub language: String,
    /// The code content
    pub code: String,
}

impl CodeBlock {
    /// Create a new code block
    pub fn new(language: impl Into<String>, code: impl Into<String>) -> Self {
        Self {
            language: language.into(),
            code: code.into(),
        }
    }

    /// Get the file extension for this code block's language
    pub fn extension(&self) -> &str {
        language_to_extension(&self.language)
    }

    /// Get the test file extension for this code block's language
    pub fn test_extension(&self) -> &str {
        language_to_test_extension(&self.language)
    }
}

/// Extract code blocks from markdown-formatted text
///
/// Parses markdown code fences (```) and returns each code block
/// with its language identifier and content.
pub fn extract_code_blocks(content: &str) -> Vec<CodeBlock> {
    let mut blocks = Vec::new();
    let mut in_block = false;
    let mut current_language = String::new();
    let mut current_code = String::new();

    for line in content.lines() {
        if line.starts_with("```") {
            if in_block {
                // End of block
                if !current_code.trim().is_empty() {
                    blocks.push(CodeBlock::new(
                        current_language.clone(),
                        current_code.trim().to_string(),
                    ));
                }
                current_language.clear();
                current_code.clear();
                in_block = false;
            } else {
                // Start of block
                current_language = line.trim_start_matches('`').trim().to_string();
                if current_language.is_empty() {
                    current_language = "txt".to_string();
                }
                in_block = true;
            }
        } else if in_block {
            current_code.push_str(line);
            current_code.push('\n');
        }
    }

    blocks
}

/// Get file extension for a programming language
pub fn language_to_extension(language: &str) -> &'static str {
    match language.to_lowercase().as_str() {
        "rust" | "rs" => "rs",
        "python" | "py" => "py",
        "javascript" | "js" => "js",
        "typescript" | "ts" => "ts",
        "go" | "golang" => "go",
        "java" => "java",
        "c" => "c",
        "cpp" | "c++" => "cpp",
        "csharp" | "c#" => "cs",
        "ruby" | "rb" => "rb",
        "php" => "php",
        "swift" => "swift",
        "kotlin" | "kt" => "kt",
        "scala" => "scala",
        "html" => "html",
        "css" => "css",
        "scss" | "sass" => "scss",
        "json" => "json",
        "yaml" | "yml" => "yaml",
        "toml" => "toml",
        "sql" => "sql",
        "bash" | "sh" | "shell" => "sh",
        "markdown" | "md" => "md",
        _ => "txt",
    }
}

/// Get test file extension for a programming language
///
/// For languages with conventions for test file naming (like JS/TS),
/// returns the test-specific extension.
pub fn language_to_test_extension(language: &str) -> &'static str {
    match language.to_lowercase().as_str() {
        "rust" | "rs" => "rs",
        "python" | "py" => "py",
        "javascript" | "js" => "test.js",
        "typescript" | "ts" => "test.ts",
        "go" | "golang" => "_test.go",
        "java" => "java",
        "csharp" | "c#" => "cs",
        "ruby" | "rb" => "rb",
        "php" => "php",
        _ => "txt",
    }
}

/// A file extracted from LLM response with path and content
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExtractedFile {
    /// The file path
    pub path: PathBuf,
    /// The file content
    pub content: String,
    /// The language (if detected)
    pub language: Option<String>,
}

impl ExtractedFile {
    /// Create a new extracted file
    pub fn new(path: PathBuf, content: String, language: Option<String>) -> Self {
        Self {
            path,
            content,
            language,
        }
    }
}

/// Extract files from LLM response with their paths
///
/// Parses an LLM response to find file paths and their associated code blocks.
/// Supports various path formats:
/// - `path/to/file.rs`
/// - **path/to/file.rs**
/// - ### path/to/file.rs
/// - File: path/to/file.rs
pub fn extract_files_from_response(content: &str) -> Vec<ExtractedFile> {
    let mut files = Vec::new();
    let mut current_path: Option<PathBuf> = None;
    let mut current_language: Option<String> = None;
    let mut current_content = String::new();
    let mut in_code_block = false;

    for line in content.lines() {
        // Check for file path marker
        if let Some(path) = extract_file_path(line) {
            // Save previous file if exists
            if let Some(prev_path) = current_path.take() {
                let content = current_content.trim().to_string();
                if !content.is_empty() {
                    files.push(ExtractedFile::new(prev_path, content, current_language.take()));
                }
            }
            current_path = Some(path);
            current_content.clear();
            current_language = None;
            continue;
        }

        // Check for code fence
        if line.starts_with("```") {
            if in_code_block {
                // End of code block
                in_code_block = false;
            } else {
                // Start of code block
                in_code_block = true;
                let lang = line.trim_start_matches('`').trim();
                if !lang.is_empty() {
                    current_language = Some(lang.to_string());
                }
            }
            continue;
        }

        // Add content if we're in a file
        if in_code_block && current_path.is_some() {
            current_content.push_str(line);
            current_content.push('\n');
        }
    }

    // Don't forget the last file
    if let Some(path) = current_path {
        let content = current_content.trim().to_string();
        if !content.is_empty() {
            files.push(ExtractedFile::new(path, content, current_language));
        }
    }

    files
}

/// Extract file path from a line (supports multiple formats)
pub fn extract_file_path(line: &str) -> Option<PathBuf> {
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
pub fn looks_like_path(s: &str) -> bool {
    // Must have an extension or be a recognizable file
    if s.contains('.') {
        // Has extension
        let ext = s.rsplit('.').next().unwrap_or("");
        let common_exts = [
            "rs", "py", "js", "ts", "tsx", "jsx", "go", "java", "c", "cpp", "h", "hpp",
            "rb", "php", "swift", "kt", "scala", "clj", "ex", "exs", "erl", "hs", "ml",
            "fs", "cs", "vb", "lua", "r", "jl", "nim", "zig", "v", "html", "css", "scss",
            "sass", "less", "vue", "svelte", "json", "yaml", "yml", "toml", "xml", "md",
            "txt", "sql", "sh", "bash", "zsh", "fish", "ps1", "bat", "cmd", "dockerfile",
            "makefile", "gitignore", "env",
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_code_blocks() {
        let content = r#"
Here's the implementation:

```rust
fn hello() {
    println!("Hello, world!");
}
```

And some JavaScript:

```javascript
function greet() {
    console.log("Hello!");
}
```
"#;

        let blocks = extract_code_blocks(content);
        assert_eq!(blocks.len(), 2);
        assert_eq!(blocks[0].language, "rust");
        assert!(blocks[0].code.contains("fn hello()"));
        assert_eq!(blocks[1].language, "javascript");
        assert!(blocks[1].code.contains("function greet()"));
    }

    #[test]
    fn test_extract_code_blocks_empty() {
        let content = "No code blocks here.";
        let blocks = extract_code_blocks(content);
        assert!(blocks.is_empty());
    }

    #[test]
    fn test_extract_code_blocks_no_language() {
        let content = "```\nsome code\n```";
        let blocks = extract_code_blocks(content);
        assert_eq!(blocks.len(), 1);
        assert_eq!(blocks[0].language, "txt");
    }

    #[test]
    fn test_language_to_extension() {
        assert_eq!(language_to_extension("rust"), "rs");
        assert_eq!(language_to_extension("Rust"), "rs");
        assert_eq!(language_to_extension("python"), "py");
        assert_eq!(language_to_extension("javascript"), "js");
        assert_eq!(language_to_extension("typescript"), "ts");
        assert_eq!(language_to_extension("unknown"), "txt");
    }

    #[test]
    fn test_language_to_test_extension() {
        assert_eq!(language_to_test_extension("rust"), "rs");
        assert_eq!(language_to_test_extension("javascript"), "test.js");
        assert_eq!(language_to_test_extension("typescript"), "test.ts");
        assert_eq!(language_to_test_extension("go"), "_test.go");
    }

    #[test]
    fn test_code_block_extension() {
        let block = CodeBlock::new("rust", "fn main() {}");
        assert_eq!(block.extension(), "rs");
        assert_eq!(block.test_extension(), "rs");

        let js_block = CodeBlock::new("javascript", "console.log('hi')");
        assert_eq!(js_block.extension(), "js");
        assert_eq!(js_block.test_extension(), "test.js");
    }

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
        // Valid paths
        assert!(looks_like_path("src/main.rs"));
        assert!(looks_like_path("config.toml"));
        assert!(looks_like_path("test.py"));
        assert!(looks_like_path("Makefile"));
        assert!(looks_like_path("Dockerfile"));

        // Invalid paths
        assert!(!looks_like_path("just text"));
        assert!(!looks_like_path("some.unknownext"));
    }

    #[test]
    fn test_extract_files_from_response() {
        let response = r#"
Here's the implementation:

`src/main.rs`
```rust
fn main() {
    println!("Hello!");
}
```

`src/lib.rs`
```rust
pub fn greet() {
    println!("Hi!");
}
```
"#;

        let files = extract_files_from_response(response);
        assert_eq!(files.len(), 2);

        assert_eq!(files[0].path, PathBuf::from("src/main.rs"));
        assert!(files[0].content.contains("fn main()"));
        assert_eq!(files[0].language, Some("rust".to_string()));

        assert_eq!(files[1].path, PathBuf::from("src/lib.rs"));
        assert!(files[1].content.contains("pub fn greet()"));
        assert_eq!(files[1].language, Some("rust".to_string()));
    }

    #[test]
    fn test_extract_files_from_response_no_paths() {
        let response = r#"
Here's some code:

```rust
fn main() {}
```
"#;
        let files = extract_files_from_response(response);
        assert!(files.is_empty());
    }

    #[test]
    fn test_extract_files_from_response_mixed_formats() {
        let response = r#"
**src/utils.rs**
```rust
pub fn helper() {}
```

File: src/config.rs
```rust
pub mod config {}
```
"#;

        let files = extract_files_from_response(response);
        assert_eq!(files.len(), 2);
        assert_eq!(files[0].path, PathBuf::from("src/utils.rs"));
        assert_eq!(files[1].path, PathBuf::from("src/config.rs"));
    }

    #[test]
    fn test_extract_files_nested_paths() {
        let response = r#"
`src/components/button.tsx`
```typescript
export const Button = () => <button>Click</button>;
```
"#;
        let files = extract_files_from_response(response);
        assert_eq!(files.len(), 1);
        assert_eq!(files[0].path, PathBuf::from("src/components/button.tsx"));
        assert_eq!(files[0].language, Some("typescript".to_string()));
    }

    #[test]
    fn test_extract_files_deeply_nested_paths() {
        let response = r#"
`src/app/features/auth/components/LoginForm.tsx`
```typescript
export const LoginForm = () => {
    return <form>Login</form>;
};
```
"#;
        let files = extract_files_from_response(response);
        assert_eq!(files.len(), 1);
        assert_eq!(
            files[0].path,
            PathBuf::from("src/app/features/auth/components/LoginForm.tsx")
        );
    }

    #[test]
    fn test_extract_files_multiple_code_blocks_after_path() {
        // Current behavior: all code blocks after a path marker are captured
        // until a new path marker is encountered. Only content inside code
        // fences is captured.
        let response = r#"
`src/main.rs`
```rust
fn main() {}
```

Some explanation text...

```rust
fn helper() {}
```
"#;
        let files = extract_files_from_response(response);
        // Both code blocks are associated with the path (concatenated)
        assert_eq!(files.len(), 1);
        assert!(files[0].content.contains("fn main()"));
        // The second code block is also captured since there's no new path marker
        assert!(files[0].content.contains("fn helper()"));
    }

    #[test]
    fn test_extract_files_path_variations() {
        // Test various path format variations
        let test_cases = vec![
            ("`Cargo.toml`", "Cargo.toml"),
            ("**`package.json`**", "package.json"),
            ("### app.py", "app.py"),
            ("File: index.html", "index.html"),
            ("Filename: `style.css`", "style.css"),
            ("Path: config.yaml", "config.yaml"),
        ];

        for (input, expected) in test_cases {
            let path = extract_file_path(input);
            assert_eq!(
                path,
                Some(PathBuf::from(expected)),
                "Failed for input: {}",
                input
            );
        }
    }

    #[test]
    fn test_extract_files_empty_code_block() {
        let response = r#"
`src/empty.rs`
```rust
```
"#;
        let files = extract_files_from_response(response);
        // Empty code blocks should not create files
        assert!(files.is_empty());
    }

    #[test]
    fn test_extract_files_whitespace_only_content() {
        let response = r#"
`src/whitespace.rs`
```rust


```
"#;
        let files = extract_files_from_response(response);
        // Whitespace-only content should not create files
        assert!(files.is_empty());
    }

    #[test]
    fn test_extract_files_preserves_indentation() {
        let response = r#"
`src/indented.py`
```python
def hello():
    if True:
        print("Hello!")
```
"#;
        let files = extract_files_from_response(response);
        assert_eq!(files.len(), 1);
        // Check that indentation is preserved
        assert!(files[0].content.contains("    if True:"));
        assert!(files[0].content.contains("        print"));
    }

    #[test]
    fn test_extract_files_consecutive_files() {
        let response = r#"
`src/a.rs`
```rust
fn a() {}
```
`src/b.rs`
```rust
fn b() {}
```
`src/c.rs`
```rust
fn c() {}
```
"#;
        let files = extract_files_from_response(response);
        assert_eq!(files.len(), 3);
        assert_eq!(files[0].path, PathBuf::from("src/a.rs"));
        assert_eq!(files[1].path, PathBuf::from("src/b.rs"));
        assert_eq!(files[2].path, PathBuf::from("src/c.rs"));
    }

    #[test]
    fn test_looks_like_path_edge_cases() {
        // Valid paths with various extensions
        assert!(looks_like_path("file.tsx"));
        assert!(looks_like_path("file.jsx"));
        assert!(looks_like_path("file.vue"));
        assert!(looks_like_path("file.svelte"));
        assert!(looks_like_path(".gitignore"));
        assert!(looks_like_path(".env"));

        // Special files without extensions
        assert!(looks_like_path("Dockerfile"));
        assert!(looks_like_path("Makefile"));

        // Invalid paths
        assert!(!looks_like_path(""));
        assert!(!looks_like_path("no-extension"));
        assert!(!looks_like_path("file.xyz123"));
    }
}
