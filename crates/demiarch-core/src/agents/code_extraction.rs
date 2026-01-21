//! Code block extraction utilities
//!
//! Provides utilities for extracting code blocks from markdown-formatted text
//! and determining file extensions for various programming languages.

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
}
