use std::path::Path;
use std::time::Instant;
use crate::safety::check_bash_safety;
use crate::truncate;

#[derive(Debug, Clone, PartialEq)]
pub enum Tool {
    ReadFile { path: String },
    ListDir { path: String },
    WriteFile { path: String, content: String },
    EditFile { path: String, search: String, replace: String },
    Bash { command: String },
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
        }
    }

    pub fn execute(&self) -> ToolResult {
        let start = Instant::now();
        let (output, success) = self.run_inner();
        let _elapsed = start.elapsed();
        ToolResult {
            tool: self.clone(),
            output,
            success,
        }
    }

    fn run_inner(&self) -> (String, bool) {
        match self {
            Tool::ReadFile { path } => read_file(path),
            Tool::ListDir { path } => list_dir(path),
            Tool::WriteFile { path, content } => write_file(path, content),
            Tool::EditFile { path, search, replace } => edit_file(path, search, replace),
            Tool::Bash { command } => run_bash(command),
            Tool::Grep { pattern, path, glob, ignore_case, literal, context, limit } => run_grep(pattern, path, glob.as_deref(), *ignore_case, *literal, *context, *limit),
            Tool::Find { pattern, path, limit } => run_find(pattern, path, *limit),
        }
    }
}

fn apply_truncation(output: String, use_tail: bool) -> String {
    let policy = truncate::TruncationPolicy::default();
    let result = if use_tail {
        truncate::truncate_tail(&output, &policy)
    } else {
        truncate::truncate_head(&output, &policy)
    };
    if result.was_truncated {
        format!(
            "[Output truncated: {} of {} lines, {} of {} bytes]\n{}",
            result.output_lines, result.total_lines,
            result.output_bytes, result.total_bytes,
            result.content
        )
    } else {
        result.content
    }
}

fn read_file(path: &str) -> (String, bool) {
    match std::fs::read_to_string(path) {
        Ok(content) => (apply_truncation(content, false), true),
        Err(e) => (format!("Error reading {}: {}", path, e), false),
    }
}

fn list_dir(path: &str) -> (String, bool) {
    let p = Path::new(path);
    match std::fs::read_dir(p) {
        Ok(entries) => {
            let mut lines = Vec::new();
            for entry in entries.flatten() {
                let name = entry.file_name().to_string_lossy().to_string();
                let typ = if entry.file_type().map(|t| t.is_dir()).unwrap_or(false) { "dir" } else { "file" };
                lines.push(format!("{} ({})", name, typ));
            }
            let output = if lines.is_empty() {
                "(empty directory)".to_string()
            } else {
                lines.join("\n")
            };
            (apply_truncation(output, false), true)
        }
        Err(e) => (format!("Error listing {}: {}", path, e), false),
    }
}

fn write_file(path: &str, content: &str) -> (String, bool) {
    match std::fs::write(path, content) {
        Ok(()) => (format!("Wrote {} bytes to {}", content.len(), path), true),
        Err(e) => (format!("Error writing {}: {}", path, e), false),
    }
}

fn edit_file(path: &str, search: &str, replace: &str) -> (String, bool) {
    if search.is_empty() {
        return ("Error: search text cannot be empty".to_string(), false);
    }
    match std::fs::read_to_string(path) {
        Ok(content) => {
            let count = content.matches(search).count();
            if count == 0 {
                return (format!("Error: search text not found in {}", path), false);
            }
            if count > 1 {
                return (format!("Error: search text appears {} times in {}. Be more specific.", count, path), false);
            }
            let new_content = content.replacen(search, replace, 1);
            match std::fs::write(path, new_content) {
                Ok(()) => (format!("Edited {}", path), true),
                Err(e) => (format!("Error writing {}: {}", path, e), false),
            }
        }
        Err(e) => (format!("Error reading {}: {}", path, e), false),
    }
}

fn run_bash(command: &str) -> (String, bool) {
    if let Some(reason) = check_bash_safety(command) {
        return (format!("Blocked: {}", reason), false);
    }
    match std::process::Command::new("bash").arg("-c").arg(command).output() {
        Ok(output) => {
            let mut result = String::new();
            if !output.stdout.is_empty() {
                result.push_str(&String::from_utf8_lossy(&output.stdout));
            }
            if !output.stderr.is_empty() {
                if !result.is_empty() { result.push('\n'); }
                result.push_str(&String::from_utf8_lossy(&output.stderr));
            }
            let success = output.status.success();
            if result.is_empty() {
                result = if success { "(no output)".to_string() } else { "(command failed)".to_string() };
            }
            (apply_truncation(result, true), success)
        }
        Err(e) => (format!("Error executing '{}': {}", command, e), false),
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
    if ignore_case { args.push("--ignore-case".into()); }
    if literal { args.push("--fixed-strings".into()); }
    if let Some(g) = glob { args.push("--glob".into()); args.push(g.into()); }
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

fn parse_grep_output(output: std::process::Output, limit: usize) -> (String, bool) {
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
    (apply_truncation(result, false), true)
}

fn run_grep(
    pattern: &str,
    path: &str,
    glob: Option<&str>,
    ignore_case: bool,
    literal: bool,
    context: usize,
    limit: usize,
) -> (String, bool) {
    let args = build_grep_args(pattern, path, glob, ignore_case, literal, context, limit);
    let tool = if which_tool("rg").is_some() { "rg" } else { "grep" };
    match std::process::Command::new(tool).args(&args).output() {
        Ok(output) => parse_grep_output(output, limit),
        Err(e) => (format!("Error running grep: {}", e), false),
    }
}

fn build_fd_args(pattern: &str, path: &str, limit: usize) -> Vec<String> {
    let mut args: Vec<String> = vec![
        "--glob".into(), "--color=never".into(), "--hidden".into(), "--no-require-git".into(),
    ];
    if pattern.contains("/") { args.push("--full-path".into()); }
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

fn parse_find_output(output: std::process::Output, limit: usize) -> (String, bool) {
    let text = String::from_utf8_lossy(&output.stdout);
    if text.trim().is_empty() {
        return ("No files found matching pattern".to_string(), true);
    }
    let lines: Vec<&str> = text.lines().collect();
    let mut out = lines[..limit.min(lines.len())].join("\n");
    if lines.len() > limit {
        out.push_str(&format!("\n\n[{} results limit reached]", limit));
    }
    (apply_truncation(out, false), true)
}

fn run_find(pattern: &str, path: &str, limit: usize) -> (String, bool) {
    let tool = if which_tool("fd").is_some() { "fd" } else { "find" };
    let result = if tool == "fd" {
        std::process::Command::new("fd").args(build_fd_args(pattern, path, limit)).output()
    } else {
        std::process::Command::new("find").args(build_find_args(pattern, path)).output()
    };

    match result {
        Ok(output) => parse_find_output(output, limit),
        Err(e) => (format!("Error running find: {}", e), false),
    }
}
