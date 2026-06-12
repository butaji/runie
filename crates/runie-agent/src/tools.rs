use crate::accumulator::{OutputAccumulator, TruncateStrategy};
use crate::safety::check_bash_safety;
use crate::truncate;
use std::path::Path;
use std::process::Command;
use std::time::{Duration, Instant};

const DEFAULT_TIMEOUT_SECS: u64 = 60;

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
        let (output, success) = self.run_inner(policy);
        let _elapsed = start.elapsed();
        ToolResult {
            tool: self.clone(),
            output,
            success,
        }
    }

    fn run_inner(&self, policy: &crate::truncate::TruncationPolicy) -> (String, bool) {
        match self {
            Tool::ReadFile {
                path,
                offset,
                limit,
            } => read_file(path, *offset, *limit, policy),
            Tool::ListDir { path } => list_dir(path, policy),
            Tool::WriteFile { path, content } => write_file(path, content),
            Tool::EditFile {
                path,
                search,
                replace,
            } => edit_file(path, search, replace),
            Tool::Bash { command } => run_bash(command, policy),
            Tool::Grep {
                pattern,
                path,
                glob,
                ignore_case,
                literal,
                context,
                limit,
            } => run_grep(
                pattern,
                path,
                glob.as_deref(),
                *ignore_case,
                *literal,
                *context,
                *limit,
                policy,
            ),
            Tool::Find {
                pattern,
                path,
                limit,
            } => run_find(pattern, path, *limit, policy),
            Tool::FetchDocs { library } => run_fetch_docs(library),
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

fn read_file(
    path: &str,
    offset: Option<usize>,
    limit: Option<usize>,
    _policy: &crate::truncate::TruncationPolicy,
) -> (String, bool) {
    let path = crate::path_utils::resolve_path(path);
    match std::fs::read_to_string(&path) {
        Ok(content) => {
            let lines: Vec<&str> = content.lines().collect();
            let total_lines = lines.len();
            let start = offset.unwrap_or(0).min(total_lines);
            let end = limit
                .map(|l| (start + l).min(total_lines))
                .unwrap_or(total_lines);

            if start >= total_lines {
                return ("(end of file)".to_string(), true);
            }

            let selected: String = lines[start..end].join("\n");
            let _lines_read = end - start;
            let output = if offset.is_some() || limit.is_some() {
                format!(
                    "[Lines {}-{} of {}]\n{}",
                    start + 1,
                    end,
                    total_lines,
                    selected
                )
            } else {
                selected
            };

            if end < total_lines {
                (
                    format!("{}\n[{} more lines]", output, total_lines - end),
                    true,
                )
            } else {
                (output, true)
            }
        }
        Err(e) => (format!("Error reading {}: {}", path.display(), e), false),
    }
}

fn list_dir(path: &str, policy: &crate::truncate::TruncationPolicy) -> (String, bool) {
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

fn write_file(path: &str, content: &str) -> (String, bool) {
    let path = crate::path_utils::resolve_path(path);
    // Create parent directories if needed
    if let Some(parent) = path.parent() {
        if !parent.as_os_str().is_empty() {
            if let Err(e) = std::fs::create_dir_all(parent) {
                return (format!("Error creating parent directories: {}", e), false);
            }
        }
    }
    match std::fs::write(&path, content) {
        Ok(()) => (
            format!("Wrote {} bytes to {}", content.len(), path.display()),
            true,
        ),
        Err(e) => (format!("Error writing {}: {}", path.display(), e), false),
    }
}

fn edit_file(path: &str, search: &str, replace: &str) -> (String, bool) {
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

fn run_bash(command: &str, policy: &crate::truncate::TruncationPolicy) -> (String, bool) {
    if let Some(reason) = check_bash_safety(command) {
        return (format!("Blocked: {}", reason), false);
    }

    let output = match run_command_with_timeout(
        "bash".to_string(),
        vec!["-c".to_string(), command.to_string()],
        Duration::from_secs(DEFAULT_TIMEOUT_SECS),
    ) {
        Ok(output) => output,
        Err(e) => return (format!("Error executing '{}': {}", command, e), false),
    };

    let mut result = String::new();
    if !output.stdout.is_empty() {
        result.push_str(&String::from_utf8_lossy(&output.stdout));
    }
    if !output.stderr.is_empty() {
        if !result.is_empty() {
            result.push('\n');
        }
        result.push_str(&String::from_utf8_lossy(&output.stderr));
    }
    let success = output.status.success();
    if result.is_empty() {
        result = if success {
            "(no output)".to_string()
        } else {
            "(command failed)".to_string()
        };
    }
    (
        apply_truncation(result, TruncateStrategy::Tail, policy),
        success,
    )
}

fn run_command_with_timeout(
    program: String,
    args: Vec<String>,
    timeout: Duration,
) -> Result<std::process::Output, std::io::Error> {
    use std::sync::mpsc;
    use std::thread;

    let (tx, rx) = mpsc::channel();

    thread::spawn(move || match Command::new(&program).args(&args).output() {
        Ok(output) => {
            let _ = tx.send(Ok(output));
        }
        Err(e) => {
            let _ = tx.send(Err(e));
        }
    });

    match rx.recv_timeout(timeout) {
        Ok(result) => result,
        Err(mpsc::RecvTimeoutError::Timeout) => Err(std::io::Error::new(
            std::io::ErrorKind::TimedOut,
            format!("Command timed out after {:?}", timeout),
        )),
        Err(mpsc::RecvTimeoutError::Disconnected) => {
            Err(std::io::Error::other("Channel disconnected unexpectedly"))
        }
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
fn run_grep(
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

fn run_find(
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

fn run_fetch_docs(library: &str) -> (String, bool) {
    let client = crate::context7::Context7Client::new();
    match client.fetch(library) {
        Ok(output) => (output, true),
        Err(e) => (
            format!("Error fetching docs for '{}': {}", library, e),
            false,
        ),
    }
}
