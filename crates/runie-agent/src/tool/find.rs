//! Find tool — searches for files matching a pattern using native Rust.
//!
//! Replaces shell-outs to `fd`/`find` with `ignore` for directory traversal.

use crate::tool::constants::FIND_DEFAULT_LIMIT;
use crate::tool::{ToolContext, ToolOutput, ToolStatus};
use ignore::WalkBuilder;
use regex::RegexBuilder;
use runie_core::tool::resolve_path;
use runie_core::tool::ToolDef;
use schemars::JsonSchema;
use serde::Deserialize;
use serde::Serialize;
use std::path::Path;
use std::time::Instant;

/// Input parameters for find tool.
#[derive(Debug, Deserialize, Serialize, JsonSchema)]
pub struct FindInput {
    /// File pattern to search for
    pub pattern: String,
    /// Root directory to search (default: current directory)
    #[serde(default)]
    pub path: Option<String>,
    /// Maximum number of results (default: 100)
    #[serde(default)]
    pub limit: Option<usize>,
}

pub struct FindTool;

impl ToolDef for FindTool {
    type Input = FindInput;

    const NAME: &'static str = "find";
    const DESCRIPTION: &'static str =
        "Find files matching a pattern using native Rust directory traversal.";
    const READ_ONLY: bool = true;
    const REQUIRES_APPROVAL: bool = false;

    async fn execute(input: Self::Input, ctx: &ToolContext) -> ToolOutput {
        let start = Instant::now();
        let path_str = input.path.as_deref().unwrap_or(".");
        let full_path = resolve_path(path_str, &ctx.working_dir);
        let limit = input.limit.unwrap_or(FIND_DEFAULT_LIMIT);

        let content = run_find(&input.pattern, &full_path, limit);

        let status = determine_find_status(&content);

        ToolOutput {
            tool_name: "find".to_owned(),
            tool_args: serde_json::json!({ "path": path_str, "pattern": input.pattern }),
            content,
            bytes_transferred: None,
            duration: start.elapsed(),
            status,
        }
    }
}

fn determine_find_status(content: &str) -> ToolStatus {
    if content.starts_with("Error") || content.is_empty() {
        ToolStatus::Error
    } else {
        ToolStatus::Success
    }
}

fn run_find(pattern: &str, path: &Path, limit: usize) -> String {
    // Build walker for directory traversal
    let walker = WalkBuilder::new(path)
        .hidden(true)
        .git_global(false)
        .git_ignore(false)
        .git_exclude(false)
        .parents(false)
        .max_depth(Some(100))
        .build();

    let mut results: Vec<String> = Vec::new();
    let pattern = pattern.trim();

    // Convert glob pattern to regex
    let regex_pattern = glob_to_regex(pattern);
    let regex = match RegexBuilder::new(&regex_pattern)
        .case_insensitive(false)
        .build()
    {
        Ok(r) => r,
        Err(_) => {
            // If regex conversion fails, use simple string matching
            return run_find_simple(pattern, path, limit);
        }
    };

    for entry in walker.flatten() {
        if !entry.file_type().is_some_and(|ft| ft.is_file()) {
            continue;
        }

        let file_path = entry.path();
        // Get the relative path from the search root
        let relative_path = match file_path.strip_prefix(path) {
            Ok(rel) => rel,
            Err(_) => file_path,
        };

        let path_str = relative_path.to_string_lossy();

        // Check if the path matches the pattern
        if regex.is_match(&path_str) {
            results.push(path_str.to_string());

            if results.len() >= limit {
                break;
            }
        }
    }

    if results.is_empty() {
        "No files found matching pattern".to_string()
    } else {
        // Sort results for consistent output
        results.sort();
        results.join("\n")
    }
}

/// Convert a glob pattern to a regex pattern.
/// Supports: * (any characters), ? (single character)
fn glob_to_regex(pattern: &str) -> String {
    let mut regex = String::new();
    regex.push('^');

    let mut chars = pattern.chars().peekable();
    while let Some(c) = chars.next() {
        match c {
            '*' => {
                // * matches any characters (including path separators)
                regex.push_str(".*");
            }
            '?' => {
                // ? matches any single character
                regex.push('.');
            }
            '.' => regex.push_str("\\."),
            '[' => {
                // Character class
                regex.push('[');
                // Copy until closing bracket
                while let Some(&c) = chars.peek() {
                    regex.push(c);
                    chars.next();
                    if c == ']' {
                        break;
                    }
                }
            }
            c if c.is_alphanumeric() || c == '_' || c == '-' || c == '/' => {
                regex.push(c);
            }
            c => regex.push_str(&regex::escape(&c.to_string())),
        }
    }

    regex.push('$');
    regex
}

/// Simple string-based find for when regex fails
fn run_find_simple(pattern: &str, path: &Path, limit: usize) -> String {
    let walker = WalkBuilder::new(path)
        .hidden(true)
        .git_global(false)
        .git_ignore(false)
        .git_exclude(false)
        .parents(false)
        .max_depth(Some(100))
        .build();

    let mut results: Vec<String> = Vec::new();

    for entry in walker.flatten() {
        if !entry.file_type().is_some_and(|ft| ft.is_file()) {
            continue;
        }

        let file_path = entry.path();
        let relative_path = match file_path.strip_prefix(path) {
            Ok(rel) => rel,
            Err(_) => file_path,
        };

        let file_name = match relative_path.file_name() {
            Some(name) => name.to_string_lossy(),
            None => continue,
        };

        // Simple exact match on filename
        if file_name == pattern {
            results.push(relative_path.to_string_lossy().to_string());

            if results.len() >= limit {
                break;
            }
        }
    }

    if results.is_empty() {
        "No files found matching pattern".to_string()
    } else {
        results.sort();
        results.join("\n")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn glob_to_regex_simple() {
        assert_eq!(glob_to_regex("*.txt"), "^.*\\.txt$");
        assert_eq!(glob_to_regex("test?"), "^test.$");
        assert_eq!(glob_to_regex("foo[0-9]bar"), "^foo[0-9]bar$");
    }

    #[test]
    fn glob_to_regex_complex() {
        assert_eq!(glob_to_regex("*.txt"), "^.*\\.txt$");
        assert_eq!(glob_to_regex("test*.log"), "^test.*\\.log$");
        assert_eq!(glob_to_regex("src/*.rs"), "^src/.*\\.rs$");
    }

    #[tokio::test]
    async fn find_no_matches() {
        let temp_dir = tempfile::tempdir().unwrap();
        let file = temp_dir.path().join("test.txt");
        std::fs::write(&file, "content").unwrap();

        let result = run_find("nonexistent", temp_dir.path(), 100);
        assert_eq!(result, "No files found matching pattern");
    }

    #[tokio::test]
    async fn find_exact_match() {
        let temp_dir = tempfile::tempdir().unwrap();
        let file = temp_dir.path().join("test.txt");
        std::fs::write(&file, "content").unwrap();

        let result = run_find("test.txt", temp_dir.path(), 100);
        assert!(result.contains("test.txt"), "Got: {}", result);
    }

    #[tokio::test]
    async fn find_glob_pattern() {
        let temp_dir = tempfile::tempdir().unwrap();

        std::fs::write(temp_dir.path().join("test.txt"), "content").unwrap();
        std::fs::write(temp_dir.path().join("test.rs"), "content").unwrap();
        std::fs::write(temp_dir.path().join("other.txt"), "content").unwrap();

        let result = run_find("*.txt", temp_dir.path(), 100);
        assert!(result.contains("test.txt"), "Got: {}", result);
        assert!(result.contains("other.txt"), "Got: {}", result);
        assert!(!result.contains("test.rs"), "Got: {}", result);
    }

    #[tokio::test]
    async fn find_star_pattern() {
        let temp_dir = tempfile::tempdir().unwrap();

        std::fs::write(temp_dir.path().join("test.txt"), "content").unwrap();
        std::fs::write(temp_dir.path().join("test123.txt"), "content").unwrap();

        let result = run_find("test*", temp_dir.path(), 100);
        assert!(result.contains("test.txt"), "Got: {}", result);
        assert!(result.contains("test123.txt"), "Got: {}", result);
    }

    #[tokio::test]
    async fn find_respects_limit() {
        let temp_dir = tempfile::tempdir().unwrap();

        for i in 0..10 {
            std::fs::write(temp_dir.path().join(format!("test{}.txt", i)), "content").unwrap();
        }

        let result = run_find("*.txt", temp_dir.path(), 3);
        let lines: Vec<&str> = result.lines().collect();
        assert_eq!(lines.len(), 3, "Got: {}", result);
    }

    #[tokio::test]
    async fn find_nested_directories() {
        let temp_dir = tempfile::tempdir().unwrap();

        let nested = temp_dir.path().join("src").join("lib");
        std::fs::create_dir_all(&nested).unwrap();
        std::fs::write(nested.join("test.rs"), "content").unwrap();

        let result = run_find("*.rs", temp_dir.path(), 100);
        assert!(result.contains("test.rs") || result.contains("src/lib/test.rs"),
            "Got: {}", result);
    }

    #[tokio::test]
    async fn find_question_mark() {
        let temp_dir = tempfile::tempdir().unwrap();

        std::fs::write(temp_dir.path().join("test1.txt"), "content").unwrap();
        std::fs::write(temp_dir.path().join("test2.txt"), "content").unwrap();
        std::fs::write(temp_dir.path().join("test.txt"), "content").unwrap();

        let result = run_find("test?.txt", temp_dir.path(), 100);
        // Should match test1.txt and test2.txt but not test.txt
        let lines: Vec<&str> = result.lines().collect();
        assert_eq!(lines.len(), 2, "Got: {}", result);
    }
}
