use anyhow::Result;
use runie_core::provider::{Message, Provider};
use crate::events::AgentEvent;
use crate::parser::parse_tool_calls;
use crate::tools::Tool;
use crate::AgentCommand;
use std::time::Instant;

pub async fn run_agent_turn<F>(
    command: &AgentCommand,
    mut emit: F,
    max_iterations: usize,
) -> Result<()>
where
    F: FnMut(AgentEvent) + Send,
{
    let mut messages = build_initial_messages(command);
    let turn_start = Instant::now();
    let mut has_intermediate_steps = false;

    for _ in 0..max_iterations {
        emit(AgentEvent::Thinking { id: command.id.clone() });

        let mut response_text = String::new();
        let provider = crate::build_provider(&command.provider, &command.model);
        provider
            .generate(messages.clone(), |chunk| {
                response_text.push_str(&chunk.content);
                emit(AgentEvent::Response {
                    id: command.id.clone(),
                    content: chunk.content,
                });
            })
            .await?;

        emit(AgentEvent::ThoughtDone { id: command.id.clone() });

        let tools = parse_tool_calls(&response_text);
        if tools.is_empty() {
            break;
        }

        has_intermediate_steps = true;
        messages.push(Message::Assistant {
            content: response_text.clone(),
        });
        execute_tools(&command.id, &tools, &mut emit, &mut messages);
    }

    if has_intermediate_steps {
        emit(AgentEvent::TurnComplete {
            id: command.id.clone(),
            duration_secs: turn_start.elapsed().as_secs_f64(),
        });
    }

    emit(AgentEvent::Done { id: command.id.clone() });
    Ok(())
}

fn build_initial_messages(command: &AgentCommand) -> Vec<Message> {
    vec![
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
    ]
}

fn execute_tools(
    cmd_id: &str,
    tools: &[Tool],
    emit: &mut dyn FnMut(AgentEvent),
    messages: &mut Vec<Message>,
) {
    for tool in tools {
        emit(AgentEvent::ToolStart {
            id: cmd_id.to_string(),
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
            content: format!("{} result:\n{}", result.tool.name(), result.output),
        });
    }
}
