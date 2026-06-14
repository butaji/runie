use crate::accumulator::{OutputAccumulator, TruncateStrategy};
use crate::truncate;
use std::path::{Path, PathBuf};
use std::time::Instant;

mod bash;
mod exec;
mod read_file;

#[derive(Debug, Clone, PartialEq)]
pub enum Tool {
    ReadFile {
        path: String,
        offset: Option<usize>,
        limit: Option<usize>,
    },
    ListDir {
        path: String,
    },
    WriteFile {
        path: String,
        content: String,
    },
    EditFile {
        path: String,
        search: String,
        replace: String,
    },
    Bash {
        command: String,
    },
    Grep {
        pattern: String,
        path: String,
        glob: Option<String>,
        ignore_case: bool,
        literal: bool,
        context: usize,
        limit: usize,
    },
    Find {
        pattern: String,
        path: String,
        limit: usize,
    },
    FetchDocs {
        library: String,
    },
}

#[derive(Debug, Clone)]
pub struct ToolResult {
    pub tool: Tool,
    pub output: String,
    pub success: bool,
}

/// Structured output from a shell execution.
///
/// Preserves stdout, stderr, exit code, and timeout information separately.
/// When output is truncated, the full content is saved to a temp file.
#[derive(Debug, Clone)]
pub struct ShellOutput {
    /// Standard output from the command.
    pub stdout: String,
    /// Standard error from the command.
    pub stderr: String,
    /// The rendered output shown to the user (already truncated with notice).
    pub rendered: String,
    /// Exit code. `None` if the command could not be executed.
    pub exit_code: Option<i32>,
    /// Whether the command timed out.
    pub timed_out: bool,
    /// Whether output was truncated.
    pub truncated: bool,
    /// Path to the full output file when truncated.
    pub full_output_path: Option<PathBuf>,
    /// Whether execution was blocked (safety check).
    pub blocked: Option<String>,
}

impl ShellOutput {
    /// Render the shell output as a user-facing string.
    ///
    /// Returns the pre-rendered output (already truncated with notice).
    pub fn render(&self) -> String {
        self.rendered.clone()
    }

    /// Returns true if the command succeeded (exit code 0).
    pub fn is_success(&self) -> bool {
        self.blocked.is_none()
            && !self.timed_out
            && self.exit_code == Some(0)
    }
}

/// Build the user-facing rendered string from shell output components.
///
/// `truncated_combined` is the output already cut by the accumulator (only
/// differs from `combined` when `truncated` is true).
pub(super) fn build_rendered(
    stdout: String,
    stderr: String,
    exit_code: Option<i32>,
    timed_out: bool,
    truncated: bool,
    full_output_path: Option<&PathBuf>,
    truncated_combined: &str,
) -> String {
    let result = combine_for_render(stdout, stderr, truncated, truncated_combined);
    append_notices(result, exit_code, timed_out, truncated, full_output_path)
}

fn combine_for_render(stdout: String, stderr: String, truncated: bool, truncated_combined: &str) -> String {
    if truncated {
        return truncated_combined.to_string();
    }
    if stdout.is_empty() && stderr.is_empty() {
        return String::new();
    }
    if stdout.is_empty() {
        return stderr.trim_end().to_string();
    }
    if stderr.is_empty() {
        return stdout.trim_end().to_string();
    }
    format!("{}\n{}", stdout.trim_end(), stderr.trim_end())
}

fn append_notices(
    mut result: String,
    exit_code: Option<i32>,
    timed_out: bool,
    truncated: bool,
    full_output_path: Option<&PathBuf>,
) -> String {
    if result.is_empty() {
        result = if exit_code == Some(0) || exit_code.is_none() {
            "(no output)".to_string()
        } else {
            "(command failed)".to_string()
        };
    }
    if timed_out {
        result.push_str("\n\n[Command timed out]");
    }
    if truncated {
        if let Some(path) = full_output_path {
            let notice = format!(
                "\n\n[Output truncated. Full output saved to {}. Read it with: head {} | tail -100]",
                path.display(),
                path.display()
            );
            result.push_str(&notice);
        }
    }
    result
}

impl Tool {
    pub fn name(&self) -> &'static str {
        match self {
            Tool::ReadFile { .. } => "read_file",
            Tool::ListDir { .. } => "list_dir",
            Tool::WriteFile { .. } => "write_file",
            Tool::EditFile { .. } => "edit_file",
            Tool::Bash { .. } => "bash",
            Tool::Grep { .. } => "grep",
            Tool::Find { .. } => "find",
            Tool::FetchDocs { .. } => "fetch_docs",
        }
    }

    pub fn execute(&self) -> ToolResult {
        self.execute_with_policy(&crate::truncate::TruncationPolicy::default())
    }

    /// Execute the tool with a specific truncation policy. Use this when the
    /// caller has a configured policy (e.g. from `config.toml`).
    pub fn execute_with_policy(&self, policy: &crate::truncate::TruncationPolicy) -> ToolResult {
        let start = Instant::now();
        let (output, success) = exec::run_inner(self, policy);
        let _elapsed = start.elapsed();
        ToolResult {
            tool: self.clone(),
            output,
            success,
        }
    }

    /// Returns true if this tool only reads data and does not modify the filesystem
    /// or execute arbitrary code.
    pub fn is_read_only(&self) -> bool {
        matches!(
            self,
            Tool::ReadFile { .. }
                | Tool::ListDir { .. }
                | Tool::Grep { .. }
                | Tool::Find { .. }
                | Tool::FetchDocs { .. }
        )
    }

    /// Execute the tool and return structured shell output if it is a bash command.
    /// Returns `None` for non-bash tools.
    pub fn execute_shell(&self, policy: &crate::truncate::TruncationPolicy) -> Option<ShellOutput> {
        if let Tool::Bash { command } = self {
            Some(bash::run_bash(command, policy))
        } else {
            None
        }
    }
}

fn apply_truncation(
    output: String,
    strategy: TruncateStrategy,
    policy: &truncate::TruncationPolicy,
) -> String {
    let mut acc = OutputAccumulator::new(policy, strategy);
    acc.append(output.as_bytes());
    let result = acc.snapshot();
    if result.was_truncated {
        format!(
            "[Output truncated: {} of {} lines, {} of {} bytes]\n{}",
            result.content.lines().count(),
            result.total_lines,
            result.content.len(),
            result.total_bytes,
            result.content
        )
    } else {
        result.content
    }
}

pub(crate) fn list_dir(path: &str, policy: &crate::truncate::TruncationPolicy) -> (String, bool) {
    let path = crate::path_utils::resolve_path(path);
    let p = Path::new(&path);
    match std::fs::read_dir(p) {
        Ok(entries) => {
            let mut lines = Vec::new();
            for entry in entries.flatten() {
                let name = entry.file_name().to_string_lossy().to_string();
                let typ = if entry.file_type().map(|t| t.is_dir()).unwrap_or(false) {
                    "dir"
                } else {
                    "file"
                };
                lines.push(format!("{} ({})", name, typ));
            }
            let output = if lines.is_empty() {
                "(empty directory)".to_string()
            } else {
                lines.join("\n")
            };
            (
                apply_truncation(output, TruncateStrategy::Head, policy),
                true,
            )
        }
        Err(e) => (format!("Error listing {}: {}", path.display(), e), false),
    }
}

pub(crate) fn edit_file(path: &str, search: &str, replace: &str) -> (String, bool) {
    let path = crate::path_utils::resolve_path(path);
    if search.is_empty() {
        return ("Error: search text cannot be empty".to_string(), false);
    }
    match std::fs::read_to_string(&path) {
        Ok(content) => {
            let count = content.matches(search).count();
            if count == 0 {
                return (
                    format!("Error: search text not found in {}", path.display()),
                    false,
                );
            }
            if count > 1 {
                return (
                    format!(
                        "Error: search text appears {} times in {}. Be more specific.",
                        count,
                        path.display()
                    ),
                    false,
                );
            }
            let new_content = content.replacen(search, replace, 1);
            match std::fs::write(&path, &new_content) {
                Ok(()) => {
                    // Generate diff output for display
                    let diff = crate::diff::generate_unified_diff(&content, &new_content);
                    let diff_output =
                        crate::diff::render_diff_to_string(&diff, &path.to_string_lossy());
                    (diff_output, true)
                }
                Err(e) => (format!("Error writing {}: {}", path.display(), e), false),
            }
        }
        Err(e) => (format!("Error reading {}: {}", path.display(), e), false),
    }
}

fn which_tool(name: &str) -> Option<String> {
    std::process::Command::new("which")
        .arg(name)
        .output()
        .ok()
        .filter(|o| o.status.success())
        .map(|o| String::from_utf8_lossy(&o.stdout).trim().to_string())
}

fn build_grep_args(
    pattern: &str,
    path: &str,
    glob: Option<&str>,
    ignore_case: bool,
    literal: bool,
    context: usize,
    limit: usize,
) -> Vec<String> {
    let mut args: Vec<String> = vec![
        "--line-number".into(),
        "--color=never".into(),
        "--hidden".into(),
    ];
    if ignore_case {
        args.push("--ignore-case".into());
    }
    if literal {
        args.push("--fixed-strings".into());
    }
    if let Some(g) = glob {
        args.push("--glob".into());
        args.push(g.into());
    }
    if context > 0 {
        args.push("--context".into());
        args.push(context.to_string());
    }
    args.push("--max-count".into());
    args.push(limit.to_string());
    args.push("--".into());
    args.push(pattern.into());
    args.push(path.into());
    args
}

fn parse_grep_output(
    output: std::process::Output,
    limit: usize,
    policy: &crate::truncate::TruncationPolicy,
) -> (String, bool) {
    let text = String::from_utf8_lossy(&output.stdout);
    let err = String::from_utf8_lossy(&output.stderr);
    if text.trim().is_empty() {
        if output.status.code() == Some(1) {
            return ("No matches found".to_string(), true);
        }
        return (format!("Error: {}", err.trim()), false);
    }
    let mut result = text.to_string();
    let lines: Vec<&str> = text.lines().collect();
    if lines.len() >= limit {
        result.push_str(&format!("\n\n[{} matches limit reached]", limit));
    }
    (
        apply_truncation(result, TruncateStrategy::Head, policy),
        true,
    )
}

#[allow(clippy::too_many_arguments)]
pub(crate) fn run_grep(
    pattern: &str,
    path: &str,
    glob: Option<&str>,
    ignore_case: bool,
    literal: bool,
    context: usize,
    limit: usize,
    policy: &crate::truncate::TruncationPolicy,
) -> (String, bool) {
    let path = crate::path_utils::resolve_path(path);
    let args = build_grep_args(
        pattern,
        &path.to_string_lossy(),
        glob,
        ignore_case,
        literal,
        context,
        limit,
    );
    let tool = if which_tool("rg").is_some() {
        "rg"
    } else {
        "grep"
    };
    match std::process::Command::new(tool).args(&args).output() {
        Ok(output) => parse_grep_output(output, limit, policy),
        Err(e) => (format!("Error running grep: {}", e), false),
    }
}

fn build_fd_args(pattern: &str, path: &str, limit: usize) -> Vec<String> {
    let mut args: Vec<String> = vec![
        "--glob".into(),
        "--color=never".into(),
        "--hidden".into(),
        "--no-require-git".into(),
    ];
    if pattern.contains("/") {
        args.push("--full-path".into());
    }
    args.push("--max-results".into());
    args.push(limit.to_string());
    args.push("--".into());
    args.push(pattern.into());
    args.push(path.into());
    args
}

fn build_find_args(pattern: &str, path: &str) -> Vec<String> {
    let mut args: Vec<String> = vec![path.into(), "-maxdepth".into(), "10".into()];
    if pattern.contains("*") || pattern.contains("?") {
        args.push("-name".into());
        args.push(pattern.into());
    } else {
        args.push("-path".into());
        args.push(format!("*/{}", pattern));
    }
    args
}

fn parse_find_output(
    output: std::process::Output,
    limit: usize,
    policy: &crate::truncate::TruncationPolicy,
) -> (String, bool) {
    let text = String::from_utf8_lossy(&output.stdout);
    if text.trim().is_empty() {
        return ("No files found matching pattern".to_string(), true);
    }
    let lines: Vec<&str> = text.lines().collect();
    let mut out = lines[..limit.min(lines.len())].join("\n");
    if lines.len() > limit {
        out.push_str(&format!("\n\n[{} results limit reached]", limit));
    }
    (apply_truncation(out, TruncateStrategy::Head, policy), true)
}

pub(crate) fn run_find(
    pattern: &str,
    path: &str,
    limit: usize,
    policy: &crate::truncate::TruncationPolicy,
) -> (String, bool) {
    let path = crate::path_utils::resolve_path(path);
    let tool = if which_tool("fd").is_some() {
        "fd"
    } else {
        "find"
    };
    let path_str = path.to_string_lossy();
    let result = if tool == "fd" {
        std::process::Command::new("fd")
            .args(build_fd_args(pattern, &path_str, limit))
            .output()
    } else {
        std::process::Command::new("find")
            .args(build_find_args(pattern, &path_str))
            .output()
    };

    match result {
        Ok(output) => parse_find_output(output, limit, policy),
        Err(e) => (format!("Error running find: {}", e), false),
    }
}

pub(crate) fn run_fetch_docs(library: &str) -> (String, bool) {
    let client = crate::context7::Context7Client::new();
    match client.fetch(library) {
        Ok(output) => (output, true),
        Err(e) => (
            format!("Error fetching docs for '{}': {}", library, e),
            false,
        ),
    }
}
