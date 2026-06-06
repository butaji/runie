//! Runie Agent - Agentic loop with 4 basic tools

use serde::{Deserialize, Serialize};
use std::path::Path;
use std::time::Instant;

// ============================================================================
// Messages
// ============================================================================

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum Message {
    System { content: String },
    User { content: String },
    Assistant { content: String },
    ToolResult { content: String },
}

impl Message {
    pub fn role(&self) -> &'static str {
        match self {
            Message::System { .. } => "system",
            Message::User { .. } => "user",
            Message::Assistant { .. } => "assistant",
            Message::ToolResult { .. } => "tool",
        }
    }

    pub fn content(&self) -> &str {
        match self {
            Message::System { content }
            | Message::User { content }
            | Message::Assistant { content }
            | Message::ToolResult { content } => content,
        }
    }
}

// ============================================================================
// Response Chunks
// ============================================================================

#[derive(Debug, Clone)]
pub struct ResponseChunk {
    pub content: String,
}

// ============================================================================
// Agent Command
// ============================================================================

#[derive(Debug, Clone)]
pub struct AgentCommand {
    pub content: String,
    pub id: String,
}

// ============================================================================
// Provider Trait
// ============================================================================

pub trait Provider: Send {
    fn generate(&self, messages: Vec<Message>) -> Vec<ResponseChunk>;
}

// ============================================================================
// Mock Provider
// ============================================================================

#[derive(Default, Clone)]
pub struct MockProvider;

impl Provider for MockProvider {
    fn generate(&self, messages: Vec<Message>) -> Vec<ResponseChunk> {
        let last = messages.last();

        // If last message is a tool result, respond with a final answer
        if matches!(last, Some(Message::ToolResult { .. })) {
            return vec![ResponseChunk {
                content: "Done. I have the information you requested.".to_string(),
            }];
        }

        // If user asks for files, use the list_files tool
        let user_input = messages
            .iter()
            .rev()
            .find_map(|m| match m {
                Message::User { content } => Some(content.clone()),
                _ => None,
            })
            .unwrap_or_default();

        if user_input.to_lowercase().contains("list files")
            || user_input.to_lowercase().contains("files")
        {
            return vec![ResponseChunk {
                content: "TOOL:list_dir:.".to_string(),
            }];
        }

        if user_input.to_lowercase().contains("read") {
            return vec![ResponseChunk {
                content: "TOOL:read_file:README.md".to_string(),
            }];
        }

        if user_input.to_lowercase().contains("write") {
            return vec![ResponseChunk {
                content: "TOOL:write_file:hello.txt:Hello World".to_string(),
            }];
        }

        if user_input.to_lowercase().contains("run") || user_input.to_lowercase().contains("cmd")
        {
            return vec![ResponseChunk {
                content: "TOOL:bash:echo hello".to_string(),
            }];
        }

        // Default: echo back the input word by word
        user_input
            .split_whitespace()
            .map(|word| ResponseChunk {
                content: format!("{} ", word),
            })
            .collect()
    }
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
pub fn run_agent_turn<P, F>(
    provider: &P,
    command: &AgentCommand,
    mut emit: F,
    max_iterations: usize,
) where
    P: Provider,
    F: FnMut(AgentEvent),
{
    let mut messages = vec![
        Message::System {
            content: "You are a helpful assistant with access to tools. \
                Use TOOL:tool_name:args format to call tools.".to_string(),
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

        let chunks = provider.generate(messages.clone());
        let response_text: String = chunks.iter().map(|c| &c.content as &str).collect();

        emit(AgentEvent::ThoughtDone {
            id: command.id.clone(),
        });

        let tools = parse_tool_calls(&response_text);

        if tools.is_empty() {
            // No tools — stream the response
            for chunk in chunks {
                emit(AgentEvent::Response {
                    id: command.id.clone(),
                    content: chunk.content,
                });
            }
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
