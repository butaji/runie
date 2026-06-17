use crate::accumulator::{OutputAccumulator, TruncateStrategy};
use crate::truncate;
use std::path::PathBuf;

mod bash;
mod exec;
mod fetch_docs;
mod find;
mod fs;
mod grep;
mod read_file;
mod runtime;

pub(crate) use fetch_docs::run_fetch_docs;
pub(crate) use find::run_find;
pub(crate) use fs::{edit_file, list_dir};
pub(crate) use grep::run_grep;

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

/// Tool execution result — wraps the canonical ToolOutput from runie-core.
#[derive(Debug, Clone)]
pub struct ToolResult {
    /// The tool that was executed.
    pub tool: Tool,
    /// Canonical output from runie-core.
    pub output: runie_core::tool::ToolOutput,
}

impl ToolResult {
    /// Returns the output content.
    pub fn content(&self) -> &str {
        &self.output.content
    }

    /// Returns whether the tool succeeded.
    pub fn is_success(&self) -> bool {
        self.output.status == runie_core::tool::ToolStatus::Success
    }
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

/// Bash execution status for ToolOutput conversion.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BashStatus {
    Success,
    Error,
    Timeout,
    Blocked,
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
        self.blocked.is_none() && !self.timed_out && self.exit_code == Some(0)
    }

    /// Returns bytes transferred (stdout + stderr length).
    pub fn bytes_transferred(&self) -> Option<u64> {
        let stdout_len = self.stdout.len() as u64;
        let stderr_len = self.stderr.len() as u64;
        Some(stdout_len + stderr_len)
    }

    /// Returns the bash execution status.
    pub fn status(&self) -> BashStatus {
        if self.blocked.is_some() {
            BashStatus::Blocked
        } else if self.timed_out {
            BashStatus::Timeout
        } else if self.exit_code == Some(0) {
            BashStatus::Success
        } else {
            BashStatus::Error
        }
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

fn combine_for_render(
    stdout: String,
    stderr: String,
    truncated: bool,
    truncated_combined: &str,
) -> String {
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

    /// Convert the tool to JSON arguments for ToolOutput.
    pub fn to_args(&self) -> serde_json::Value {
        match self {
            Tool::ReadFile {
                path,
                offset,
                limit,
            } => read_file_args(path, *offset, *limit),
            Tool::ListDir { path } => serde_json::json!({ "path": path }),
            Tool::WriteFile { path, content: _ } => {
                serde_json::json!({ "path": path, "content": "<redacted>" })
            }
            Tool::EditFile {
                path,
                search,
                replace,
            } => edit_file_args(path, search, replace),
            other => other.to_args_rest(),
        }
    }
}

impl Tool {
    fn to_args_rest(&self) -> serde_json::Value {
        match self {
            Tool::Bash { command } => serde_json::json!({ "command": command }),
            Tool::Grep {
                pattern,
                path,
                glob,
                ignore_case,
                literal,
                context,
                limit,
            } => grep_args(
                pattern,
                path,
                glob,
                *ignore_case,
                *literal,
                *context,
                *limit,
            ),
            Tool::Find {
                pattern,
                path,
                limit,
            } => find_args(pattern, path, *limit),
            Tool::FetchDocs { library } => serde_json::json!({ "library": library }),
            _ => unreachable!(),
        }
    }
}

fn read_file_args(path: &str, offset: Option<usize>, limit: Option<usize>) -> serde_json::Value {
    serde_json::json!({ "path": path, "offset": offset, "limit": limit })
}

fn edit_file_args(path: &str, search: &str, replace: &str) -> serde_json::Value {
    serde_json::json!({ "path": path, "search": search, "replace": replace })
}

fn find_args(pattern: &str, path: &str, limit: usize) -> serde_json::Value {
    serde_json::json!({ "pattern": pattern, "path": path, "limit": limit })
}

fn grep_args(
    pattern: &str,
    path: &str,
    glob: &Option<String>,
    ignore_case: bool,
    literal: bool,
    context: usize,
    limit: usize,
) -> serde_json::Value {
    serde_json::json!({
        "pattern": pattern,
        "path": path,
        "glob": glob,
        "ignore_case": ignore_case,
        "literal": literal,
        "context": context,
        "limit": limit
    })
}

impl Tool {
    pub fn execute(&self) -> ToolResult {
        self.execute_with_policy(&crate::truncate::TruncationPolicy::default())
    }

    /// Execute the tool with a specific truncation policy. Use this when the
    /// caller has a configured policy (e.g. from `config.toml`).
    pub fn execute_with_policy(&self, policy: &crate::truncate::TruncationPolicy) -> ToolResult {
        let output = exec::run_inner(self, policy);
        ToolResult {
            tool: self.clone(),
            output,
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
