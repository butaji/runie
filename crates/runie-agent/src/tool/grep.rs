//! Grep tool — searches for patterns in files using native Rust.
//!
//! Replaces shell-outs to `rg`/`grep` with `ignore` for directory traversal
//! and `regex` for pattern matching.

use crate::tool::constants::GREP_DEFAULT_LIMIT;
use crate::tool::{ToolContext, ToolOutput, ToolStatus};
use ignore::WalkBuilder;
use regex::{Regex, RegexBuilder};
use runie_core::path::resolve_path_in;
use runie_core::tool::{tool_error, ToolDef};
use schemars::JsonSchema;
use serde::Deserialize;
use serde::Serialize;
use std::io::{BufRead, BufReader};
use std::path::Path;
use std::time::Instant;

/// Input parameters for grep tool.
#[derive(Debug, Deserialize, Serialize, JsonSchema)]
pub struct GrepInput {
    /// Search pattern
    pub pattern: String,
    /// Directory or file path to search
    pub path: String,
    /// File glob pattern (e.g., *.rs)
    #[serde(default)]
    pub glob: Option<String>,
    /// Case-insensitive search
    #[serde(default)]
    pub ignore_case: Option<bool>,
    /// Treat pattern as literal string
    #[serde(default)]
    pub literal: Option<bool>,
    /// Maximum number of matches (default: 100)
    #[serde(default)]
    pub limit: Option<usize>,
}

pub struct GrepTool;

impl ToolDef for GrepTool {
    type Input = GrepInput;

    const NAME: &'static str = "grep";
    const DESCRIPTION: &'static str =
        "Search for patterns in files using native Rust regex matching.";
    const READ_ONLY: bool = true;
    const REQUIRES_APPROVAL: bool = false;

    async fn execute(input: Self::Input, ctx: &ToolContext) -> ToolOutput {
        let start = Instant::now();
        let full_path = resolve_path_in(&input.path, &ctx.working_dir);
        run_grep_impl(
            &input.pattern,
            &full_path,
            input.glob.as_deref(),
            input.ignore_case.unwrap_or(false),
            input.literal.unwrap_or(false),
            input.limit.unwrap_or(GREP_DEFAULT_LIMIT),
            start,
        )
        .await
    }
}

async fn run_grep_impl(
    pattern: &str,
    path: &Path,
    glob: Option<&str>,
    ignore_case: bool,
    literal: bool,
    limit: usize,
    start: Instant,
) -> ToolOutput {
    // Build regex pattern
    let regex_pattern = if literal {
        // In literal mode, escape the pattern so it matches exactly.
        // Escape all regex metacharacters and anchor to match the entire line.
        format!("^{}$", regex::escape(pattern))
    } else {
        pattern.to_string()
    };

    let regex = match RegexBuilder::new(&regex_pattern)
        .case_insensitive(ignore_case)
        .build()
    {
        Ok(r) => r,
        Err(e) => {
            return tool_error(
                "grep",
                &format!("Invalid regex pattern '{}': {}", pattern, e),
                start,
                false,
            );
        }
    };

    // Build walker
    let mut walker = WalkBuilder::new(path);
    walker
        .hidden(true)
        .git_global(false)
        .git_ignore(false)
        .git_exclude(false)
        .parents(false)
        .max_depth(Some(100));

    if let Some(g) = glob {
        // Convert glob pattern to regex for filtering
        let glob_regex = glob_to_regex(g);
        walker.filter_entry(move |entry| {
            if let Some(name) = entry.file_name().to_str() {
                matches_glob(name, &glob_regex)
            } else {
                true
            }
        });
    }

    let mut results = Vec::new();
    let mut match_count = 0;

    for entry in walker.build().flatten() {
        if !entry.file_type().is_some_and(|ft| ft.is_file()) {
            continue;
        }

        let file_path = entry.path();
        if let Ok(file) = std::fs::File::open(file_path) {
            let reader = BufReader::new(file);
            for (line_num, line) in reader.lines().enumerate() {
                if let Ok(content) = line {
                    if regex.is_match(&content) {
                        match_count += 1;
                        if match_count <= limit {
                            results.push(format!(
                                "{}:{}:{}",
                                file_path.display(),
                                line_num + 1,
                                content
                            ));
                        }
                    }
                }
            }
        }

        if match_count > limit {
            break;
        }
    }

    let content = if results.is_empty() {
        "No matches found".to_string()
    } else {
        if match_count > limit {
            results.push(format!(
                "... ({} more matches, showing first {})",
                match_count - limit,
                limit
            ));
        }
        results.join("\n")
    };

    let bytes = content.len();
    // grep returns Success whether or not matches were found
    // (no matches means empty output, which is still success)
    let status = ToolStatus::Success;

    ToolOutput {
        tool_name: "grep".to_owned(),
        tool_args: serde_json::json!({ "path": path, "pattern": pattern }),
        content,
        bytes_transferred: Some(bytes as u64),
        duration: start.elapsed(),
        status,
    }
}

/// Convert a simple glob pattern to a regex pattern.
/// Supports: * (any characters), ? (single character)
fn glob_to_regex(glob: &str) -> String {
    let mut regex = String::from("^");
    for c in glob.chars() {
        match c {
            '*' => regex.push_str(".*"),
            '?' => regex.push('.'),
            '.' => regex.push_str("\\."),
            '[' => regex.push('['),
            ']' => regex.push(']'),
            c if c.is_alphanumeric() || c == '_' || c == '-' => regex.push(c),
            c => regex.push_str(&regex::escape(&c.to_string())),
        }
    }
    regex.push('$');
    regex
}

/// Check if a filename matches a glob pattern (converted to regex).
fn matches_glob(filename: &str, glob_regex: &str) -> bool {
    if let Ok(re) = Regex::new(glob_regex) {
        re.is_match(filename)
    } else {
        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn glob_to_regex_simple() {
        assert_eq!(glob_to_regex("*.rs"), "^.*\\.rs$");
        assert_eq!(glob_to_regex("test?"), "^test.$");
        assert_eq!(glob_to_regex("foo[0-9]bar"), "^foo[0-9]bar$");
    }

    #[test]
    fn glob_to_regex_complex() {
        assert_eq!(glob_to_regex("*.txt"), "^.*\\.txt$");
        assert_eq!(glob_to_regex("test*.log"), "^test.*\\.log$");
    }

    #[tokio::test]
    async fn grep_no_matches() {
        // Create a temp directory with no matching files
        let temp_dir = tempfile::tempdir().unwrap();
        let result = run_grep_impl(
            "nonexistent_pattern",
            temp_dir.path(),
            None,
            false,
            false,
            100,
            Instant::now(),
        )
        .await;
        assert_eq!(result.content, "No matches found");
        assert_eq!(result.status, ToolStatus::Success);
    }

    #[tokio::test]
    async fn grep_finds_matches() {
        let temp_dir = tempfile::tempdir().unwrap();

        // Create test files
        let file1 = temp_dir.path().join("test.txt");
        std::fs::write(&file1, "hello world\nfoo bar\n").unwrap();

        let file2 = temp_dir.path().join("test2.txt");
        std::fs::write(&file2, "hello again\n").unwrap();

        let result = run_grep_impl(
            "hello",
            temp_dir.path(),
            Some("*.txt"),
            false,
            false,
            100,
            Instant::now(),
        )
        .await;

        assert!(result.content.contains("test.txt:1:hello world"));
        assert!(result.content.contains("test2.txt:1:hello again"));
        assert_eq!(result.status, ToolStatus::Success);
    }

    #[tokio::test]
    async fn grep_case_insensitive() {
        let temp_dir = tempfile::tempdir().unwrap();
        let file = temp_dir.path().join("test.txt");
        std::fs::write(&file, "Hello WORLD\n").unwrap();

        let result = run_grep_impl(
            "hello",
            temp_dir.path(),
            None,
            true, // case insensitive
            false,
            100,
            Instant::now(),
        )
        .await;

        assert!(result.content.contains("Hello WORLD"));
        assert_eq!(result.status, ToolStatus::Success);
    }

    #[tokio::test]
    async fn grep_literal() {
        let temp_dir = tempfile::tempdir().unwrap();
        let file = temp_dir.path().join("test.txt");
        std::fs::write(&file, "hello.*world\n").unwrap();

        // Without literal, the pattern is treated as regex
        let result_regex = run_grep_impl(
            "hello.*",
            temp_dir.path(),
            None,
            false,
            false, // not literal
            100,
            Instant::now(),
        )
        .await;
        assert!(result_regex.content.contains("hello.*world"));

        // With literal, the pattern is treated as literal string
        let result_literal = run_grep_impl(
            "hello.*",
            temp_dir.path(),
            None,
            false,
            true, // literal
            100,
            Instant::now(),
        )
        .await;
        assert_eq!(result_literal.content, "No matches found");
    }

    #[tokio::test]
    async fn grep_invalid_regex() {
        let temp_dir = tempfile::tempdir().unwrap();
        let result = run_grep_impl(
            "[invalid", // Invalid regex
            temp_dir.path(),
            None,
            false,
            false,
            100,
            Instant::now(),
        )
        .await;

        assert!(result.content.contains("Invalid regex pattern"));
        assert_eq!(result.status, ToolStatus::Error);
    }

    #[tokio::test]
    async fn grep_respects_limit() {
        let temp_dir = tempfile::tempdir().unwrap();

        // Create many files with matches
        for i in 0..10 {
            let file = temp_dir.path().join(format!("test{}.txt", i));
            std::fs::write(&file, format!("match line\n")).unwrap();
        }

        let result = run_grep_impl(
            "match",
            temp_dir.path(),
            None,
            false,
            false,
            3, // limit to 3
            Instant::now(),
        )
        .await;

        // Should have exactly 3 matches (the 3 results)
        let match_count = result.content.matches("match line").count();
        assert_eq!(match_count, 3, "Result: {}", result.content);
        // The "more matches" message confirms results were limited
        // Count total lines: 3 result lines + 1 summary line = 4
        let lines: Vec<&str> = result.content.lines().collect();
        assert_eq!(lines.len(), 4, "Expected 4 lines (3 results + summary), got {}", result.content);
    }
}
