//! Runie Agent - Agentic loop with 4 basic tools + safety checks

use anyhow::Result;
use runie_core::provider::{Message, Provider};
use runie_provider::{AnyProvider, MockProvider};
use std::path::Path;
use std::time::Instant;

pub mod truncate;





#[derive(Debug, Clone)]
pub struct AgentCommand {
    pub content: String,
    pub id: String,
    pub provider: String,
    pub model: String,
}





/// Check if a bash command is dangerous.
/// Returns Some(reason) if blocked, None if safe.
pub fn check_bash_safety(command: &str) -> Option<&'static str> {
    let cmd = command.trim().to_lowercase();


    if cmd.contains("rm -rf /") || cmd.contains("rm -rf /*") || cmd.contains("rm -rf ~") {
        return Some("rm -rf on system directories or home is blocked");
    }


    if cmd.starts_with("dd ") && cmd.contains("of=/dev/") {
        return Some("dd writing to block devices is blocked");
    }
    if cmd.contains("> /dev/sda") || cmd.contains("> /dev/nvme") || cmd.contains("> /dev/hd") {
        return Some("writing directly to block devices is blocked");
    }


    if cmd.starts_with("mkfs") || cmd.starts_with("mkfs.") {
        return Some("mkfs is blocked");
    }


    if cmd.contains(":|:") && cmd.contains("};") {
        return Some("fork bombs are blocked");
    }


    if cmd.contains("chmod -r 777 /") || cmd.contains("chmod -r 000 /") {
        return Some("recursive chmod on root is blocked");
    }


    if cmd.starts_with("sudo ") && (cmd.contains(" rm ") || cmd.contains(" dd ")) {
        return Some("sudo with destructive commands is blocked");
    }

    None
}





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

    /// Execute the tool and return a ToolResult
    pub fn execute(&self) -> ToolResult {
        let start = Instant::now();
        let (output, success) = match self {
            Tool::ReadFile { path } => read_file(path),
            Tool::ListDir { path } => list_dir(path),
            Tool::WriteFile { path, content } => write_file(path, content),
            Tool::EditFile { path, search, replace } => edit_file(path, search, replace),
            Tool::Bash { command } => run_bash(command),
            Tool::Grep { pattern, path, glob, ignore_case, literal, context, limit } => {
                run_grep(pattern, path, glob.as_deref(), *ignore_case, *literal, *context, *limit)
            }
            Tool::Find { pattern, path, limit } => run_find(pattern, path, *limit),
        };
        let _elapsed = start.elapsed();
        ToolResult {
            tool: self.clone(),
            output,
            success,
        }
    }
}

/// Build a provider from provider/model strings.
pub fn build_provider(provider: &str, model: &str) -> AnyProvider {
    AnyProvider::new(provider, model)
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

    match std::process::Command::new("bash")
        .arg("-c")
        .arg(command)
        .output()
    {
        Ok(output) => {
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
            (apply_truncation(result, true), success)
        }
        Err(e) => (format!("Error executing '{}': {}", command, e), false),
    }
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

    let tool = if which_tool("rg").is_some() { "rg" } else { "grep" };
    match std::process::Command::new(tool).args(&args).output() {
        Ok(output) => {
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
        Err(e) => (format!("Error running grep: {}", e), false),
    }
}

fn run_find(pattern: &str, path: &str, limit: usize) -> (String, bool) {
    let tool = if which_tool("fd").is_some() { "fd" } else { "find" };
    let result = if tool == "fd" {
        let mut args: Vec<String> = vec![
            "--glob".into(), "--color=never".into(), "--hidden".into(), "--no-require-git".into(),
        ];
        if pattern.contains("/") {
            args.push("--full-path".into());
        }
        args.push("--max-results".into());
        args.push(limit.to_string());
        args.push("--".into());
        args.push(pattern.into());
        args.push(path.into());
        std::process::Command::new("fd").args(&args).output()
    } else {
        let mut args: Vec<String> = vec![path.into(), "-maxdepth".into(), "10".into()];
        if pattern.contains("*") || pattern.contains("?") {
            args.push("-name".into());
            args.push(pattern.into());
        } else {
            args.push("-path".into());
            args.push(format!("*/{}", pattern));
        }
        std::process::Command::new("find").args(&args).output()
    };

    match result {
        Ok(output) => {
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
        Err(e) => (format!("Error running find: {}", e), false),
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





#[derive(Debug, serde::Deserialize)]
struct ToolCall {
    name: String,
    arguments: serde_json::Map<String, serde_json::Value>,
}

fn arg_str(args: &serde_json::Map<String, serde_json::Value>, key: &str) -> String {
    args.get(key).and_then(|v| v.as_str()).unwrap_or("").to_string()
}

fn arg_opt_str(args: &serde_json::Map<String, serde_json::Value>, key: &str) -> Option<String> {
    args.get(key).and_then(|v| v.as_str()).filter(|s| !s.is_empty()).map(|s| s.to_string())
}

fn arg_bool(args: &serde_json::Map<String, serde_json::Value>, key: &str) -> bool {
    args.get(key).and_then(|v| v.as_bool()).unwrap_or(false)
}

fn arg_usize(args: &serde_json::Map<String, serde_json::Value>, key: &str) -> usize {
    args.get(key).and_then(|v| v.as_u64()).unwrap_or(0) as usize
}

fn parse_legacy_tool(payload: &str) -> Option<Tool> {
    let parts: Vec<&str> = payload.splitn(3, ':').collect();
    if parts.len() < 2 {
        return None;
    }
    let tool_name = parts[0];
    let arg1 = parts.get(1).unwrap_or(&"");
    let arg2 = parts.get(2).unwrap_or(&"");
    Some(match tool_name {
        "read_file" => Tool::ReadFile { path: arg1.to_string() },
        "list_dir" => Tool::ListDir { path: arg1.to_string() },
        "write_file" => Tool::WriteFile { path: arg1.to_string(), content: arg2.to_string() },
        "bash" => Tool::Bash { command: arg1.to_string() },
        _ => return None,
    })
}

fn parse_structured_tool(line: &str) -> Option<Tool> {
    let call: ToolCall = serde_json::from_str(line).ok()?;
    let args = &call.arguments;
    Some(match call.name.as_str() {
        "read_file" => Tool::ReadFile { path: arg_str(args, "path") },
        "list_dir" => Tool::ListDir { path: arg_str(args, "path") },
        "write_file" => Tool::WriteFile { path: arg_str(args, "path"), content: arg_str(args, "content") },
        "edit_file" => Tool::EditFile {
            path: arg_str(args, "path"),
            search: arg_str(args, "search"),
            replace: arg_str(args, "replace"),
        },
        "bash" => Tool::Bash { command: arg_str(args, "command") },
        "grep" => Tool::Grep {
            pattern: arg_str(args, "pattern"),
            path: arg_str(args, "path"),
            glob: arg_opt_str(args, "glob"),
            ignore_case: arg_bool(args, "ignore_case"),
            literal: arg_bool(args, "literal"),
            context: arg_usize(args, "context"),
            limit: arg_usize(args, "limit").max(1),
        },
        "find" => Tool::Find {
            pattern: arg_str(args, "pattern"),
            path: arg_str(args, "path"),
            limit: arg_usize(args, "limit").max(1),
        },
        _ => return None,
    })
}

/// Parse tool calls from assistant response text.
/// Supports both legacy `TOOL:name:args` and structured JSON formats.
pub fn parse_tool_calls(text: &str) -> Vec<Tool> {
    let mut tools = Vec::new();
    for line in text.lines() {
        let line = line.trim();
        if line.is_empty() {
            continue;
        }
        let tool = if line.starts_with("TOOL:") {
            parse_legacy_tool(line.strip_prefix("TOOL:").unwrap_or(""))
        } else if line.starts_with('{') {
            parse_structured_tool(line)
        } else {
            None
        };
        if let Some(t) = tool {
            tools.push(t);
        }
    }
    tools
}

/// Check if text contains any tool calls
pub fn has_tool_calls(text: &str) -> bool {
    !parse_tool_calls(text).is_empty()
}





/// Run a single agent turn with the provider, emitting events via the callback.
/// The loop continues until the assistant no longer requests tools or max_iterations is reached.
pub async fn run_agent_turn<F>(
    command: &AgentCommand,
    mut emit: F,
    max_iterations: usize,
) -> Result<()>
where
    F: FnMut(AgentEvent) + Send,
{
    let mut messages = vec![
        Message::System {
            content: "You are a helpful assistant with access to tools. \
                Use structured JSON format: {\"name\": \"tool_name\", \"arguments\": {...}}. \
                Available tools: read_file, list_dir, write_file, edit_file, bash, grep, find. \
                Use edit_file for safe changes: {\"name\": \"edit_file\", \"arguments\": {\"path\": \"...\", \"search\": \"...\", \"replace\": \"...\"}}. \
                Use grep to search file contents: {\"name\": \"grep\", \"arguments\": {\"pattern\": \"...\", \"path\": \"...\"}}. \
                Use find to list files by pattern: {\"name\": \"find\", \"arguments\": {\"pattern\": \"...\", \"path\": \"...\"}}.".to_string(),
        },
        Message::User {
            content: command.content.clone(),
        },
    ];

    let turn_start = Instant::now();
    let mut has_intermediate_steps = false;

    for _iteration in 0..max_iterations {

        emit(AgentEvent::Thinking {
            id: command.id.clone(),
        });

        let mut response_text = String::new();
        let provider = build_provider(&command.provider, &command.model);
        provider
            .generate(messages.clone(), |chunk| {
                response_text.push_str(&chunk.content);
                emit(AgentEvent::Response {
                    id: command.id.clone(),
                    content: chunk.content,
                });
            })
            .await?;

        emit(AgentEvent::ThoughtDone {
            id: command.id.clone(),
        });

        let tools = parse_tool_calls(&response_text);

        if tools.is_empty() {
            break;
        }


        has_intermediate_steps = true;
        messages.push(Message::Assistant {
            content: response_text.clone(),
        });
        for tool in tools {
            emit(AgentEvent::ToolStart {
                id: command.id.clone(),
                name: tool.name().to_string(),
            });

            let tool_start = Instant::now();
            let result = tool.execute();
            let tool_elapsed = tool_start.elapsed().as_secs_f64();

            emit(AgentEvent::ToolEnd {
                duration_secs: tool_elapsed,
                output: result.output.clone(),
            });

            messages.push(Message::ToolResult {
                content: format!(
                    "{} result:\n{}",
                    result.tool.name(),
                    result.output
                ),
            });
        }
    }

    if has_intermediate_steps {
        emit(AgentEvent::TurnComplete {
            id: command.id.clone(),
            duration_secs: turn_start.elapsed().as_secs_f64(),
        });
    }

    emit(AgentEvent::Done {
        id: command.id.clone(),
    });

    Ok(())
}





#[derive(Debug, Clone)]
pub enum AgentEvent {
    Thinking { id: String },
    ThoughtDone { id: String },
    ToolStart { id: String, name: String },
    ToolEnd { duration_secs: f64, output: String },
    Response { id: String, content: String },
    TurnComplete { id: String, duration_secs: f64 },
    Done { id: String },
    Error { id: String, message: String },
}

impl AgentEvent {
    pub fn to_core_event(&self) -> runie_core::Event {
        match self.clone() {
            AgentEvent::Thinking { id } => runie_core::Event::AgentThinking { id },
            AgentEvent::ThoughtDone { id } => runie_core::Event::AgentThoughtDone { id },
            AgentEvent::ToolStart { id, name } => {
                runie_core::Event::AgentToolStart { id, name }
            }
            AgentEvent::ToolEnd { duration_secs, output } => {
                runie_core::Event::AgentToolEnd { duration_secs, output }
            }
            AgentEvent::Response { id, content } => {
                runie_core::Event::AgentResponse { id, content }
            }
            AgentEvent::TurnComplete { id, duration_secs } => {
                runie_core::Event::AgentTurnComplete { id, duration_secs }
            }
            AgentEvent::Done { id } => runie_core::Event::AgentDone { id },
            AgentEvent::Error { id, message } => {
                runie_core::Event::AgentError { id, message }
            }
        }
    }
}

#[cfg(test)]
mod tests;
#[cfg(test)]
mod grep_find;
#[cfg(test)]
mod truncate_tests;
