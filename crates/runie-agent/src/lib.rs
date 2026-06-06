//! Runie Agent - Agentic loop with 4 basic tools + safety checks

use anyhow::Result;
use runie_core::provider::{Message, Provider};
use runie_provider::{AnyProvider, MockProvider, OpenAiProvider};
use std::path::Path;
use std::time::Instant;

// ============================================================================
// Agent Command
// ============================================================================

#[derive(Debug, Clone)]
pub struct AgentCommand {
    pub content: String,
    pub id: String,
    pub provider: String,
    pub model: String,
}

// ============================================================================
// Bash Safety
// ============================================================================

/// Check if a bash command is dangerous.
/// Returns Some(reason) if blocked, None if safe.
pub fn check_bash_safety(command: &str) -> Option<&'static str> {
    let cmd = command.trim().to_lowercase();

    // Destructive rm patterns
    if cmd.contains("rm -rf /") || cmd.contains("rm -rf /*") || cmd.contains("rm -rf ~") {
        return Some("rm -rf on system directories or home is blocked");
    }

    // Disk destruction
    if cmd.starts_with("dd ") && cmd.contains("of=/dev/") {
        return Some("dd writing to block devices is blocked");
    }
    if cmd.contains("> /dev/sda") || cmd.contains("> /dev/nvme") || cmd.contains("> /dev/hd") {
        return Some("writing directly to block devices is blocked");
    }

    // Filesystem format
    if cmd.starts_with("mkfs") || cmd.starts_with("mkfs.") {
        return Some("mkfs is blocked");
    }

    // Fork bomb
    if cmd.contains(":|:") && cmd.contains("};") {
        return Some("fork bombs are blocked");
    }

    // Dangerous chmod
    if cmd.contains("chmod -r 777 /") || cmd.contains("chmod -r 000 /") {
        return Some("recursive chmod on root is blocked");
    }

    // sudo with destructive commands
    if cmd.starts_with("sudo ") && (cmd.contains(" rm ") || cmd.contains(" dd ")) {
        return Some("sudo with destructive commands is blocked");
    }

    None
}

// ============================================================================
// Tools
// ============================================================================

#[derive(Debug, Clone, PartialEq)]
pub enum Tool {
    ReadFile { path: String },
    ListDir { path: String },
    WriteFile { path: String, content: String },
    Bash { command: String },
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
            Tool::Bash { .. } => "bash",
        }
    }

    /// Execute the tool and return a ToolResult
    pub fn execute(&self) -> ToolResult {
        let start = Instant::now();
        let (output, success) = match self {
            Tool::ReadFile { path } => read_file(path),
            Tool::ListDir { path } => list_dir(path),
            Tool::WriteFile { path, content } => write_file(path, content),
            Tool::Bash { command } => run_bash(command),
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
    match provider {
        "openai" => {
            let key = std::env::var("OPENAI_API_KEY").unwrap_or_default();
            if key.is_empty() {
                eprintln!("Warning: OPENAI_API_KEY not set, falling back to mock");
                AnyProvider::Mock(MockProvider)
            } else {
                AnyProvider::OpenAi(OpenAiProvider::new(key, model))
            }
        }
        _ => AnyProvider::Mock(MockProvider),
    }
}

fn read_file(path: &str) -> (String, bool) {
    match std::fs::read_to_string(path) {
        Ok(content) => (content, true),
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
            if lines.is_empty() {
                ("(empty directory)".to_string(), true)
            } else {
                (lines.join("\n"), true)
            }
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
            (result, success)
        }
        Err(e) => (format!("Error executing '{}': {}", command, e), false),
    }
}

// ============================================================================
// Tool Parser
// ============================================================================

/// Parse a tool call from assistant response text.
/// Format: `TOOL:tool_name:arg1:arg2:...`
pub fn parse_tool_calls(text: &str) -> Vec<Tool> {
    let mut tools = Vec::new();
    for line in text.lines() {
        let line = line.trim();
        if let Some(payload) = line.strip_prefix("TOOL:") {
            let parts: Vec<&str> = payload.splitn(3, ':').collect();
            if parts.len() < 2 {
                continue;
            }
            let tool_name = parts[0];
            let arg1 = parts.get(1).unwrap_or(&"");
            let arg2 = parts.get(2).unwrap_or(&"");

            let tool = match tool_name {
                "read_file" => Tool::ReadFile {
                    path: arg1.to_string(),
                },
                "list_dir" => Tool::ListDir {
                    path: arg1.to_string(),
                },
                "write_file" => Tool::WriteFile {
                    path: arg1.to_string(),
                    content: arg2.to_string(),
                },
                "bash" => Tool::Bash {
                    command: arg1.to_string(),
                },
                _ => continue,
            };
            tools.push(tool);
        }
    }
    tools
}

/// Check if text contains any tool calls
pub fn has_tool_calls(text: &str) -> bool {
    parse_tool_calls(text).len() > 0
}

// ============================================================================
// Agent Loop
// ============================================================================

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
                Use TOOL:tool_name:args format to call tools. \
                Available tools: read_file, list_dir, write_file, bash.".to_string(),
        },
        Message::User {
            content: command.content.clone(),
        },
    ];

    let turn_start = Instant::now();
    let mut has_intermediate_steps = false;

    for _iteration in 0..max_iterations {
        // Thinking phase
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

        // Execute tools
        has_intermediate_steps = true;
        for tool in tools {
            emit(AgentEvent::ToolStart {
                id: command.id.clone(),
                name: tool.name().to_string(),
            });

            let result = tool.execute();

            emit(AgentEvent::ToolEnd {
                duration_secs: 0.0, // simplified
            });

            messages.push(Message::Assistant {
                content: response_text.clone(),
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

// ============================================================================
// Agent Events
// ============================================================================

#[derive(Debug, Clone)]
pub enum AgentEvent {
    Thinking { id: String },
    ThoughtDone { id: String },
    ToolStart { id: String, name: String },
    ToolEnd { duration_secs: f64 },
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
            AgentEvent::ToolEnd { duration_secs } => {
                runie_core::Event::AgentToolEnd { duration_secs }
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
