use crate::parser::parse_tool_calls;
use crate::tools::Tool;
use crate::AgentCommand;
use anyhow::Result;
use futures::StreamExt;
use runie_core::event::Event;
use runie_core::provider::{Message, Provider};
use std::time::Instant;

pub async fn run_agent_turn<F>(
    command: &AgentCommand,
    mut emit: F,
    max_iterations: usize,
) -> Result<()>
where
    F: FnMut(Event) + Send,
{
    let mut messages = build_initial_messages(command);
    let turn_start = Instant::now();
    let mut has_intermediate_steps = false;

    for _ in 0..max_iterations {
        emit(Event::AgentThinking {
            id: command.id.clone(),
        });

        let mut response_text = String::new();
        let provider = crate::build_provider(&command.provider, &command.model);
        let mut stream = provider.generate(messages.clone());
        while let Some(chunk_result) = stream.next().await {
            let chunk = chunk_result?;
            response_text.push_str(&chunk.content);
            emit(Event::AgentResponse {
                id: command.id.clone(),
                content: chunk.content,
            });
        }

        emit(Event::AgentThoughtDone {
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
        execute_tools(&command.id, &tools, &mut emit, &mut messages);
    }

    if has_intermediate_steps {
        emit(Event::AgentTurnComplete {
            id: command.id.clone(),
            duration_secs: turn_start.elapsed().as_secs_f64(),
        });
    }

    emit(Event::AgentDone {
        id: command.id.clone(),
    });
    Ok(())
}

pub(crate) fn build_initial_messages(command: &AgentCommand) -> Vec<Message> {
    let tools_list = if command.read_only {
        "read_file, list_dir, grep, find, fetch_docs"
    } else {
        "read_file, list_dir, write_file, edit_file, bash, grep, find, fetch_docs"
    };
    let base = if command.system_prompt.is_empty() {
        runie_core::prompts::DEFAULT_PROMPT
    } else {
        &command.system_prompt
    };
    let mut system = runie_core::prompts::build_system_prompt(
        base,
        tools_list,
        command.read_only,
        command.thinking_level.prompt_suffix(),
    );
    if !command.skills_context.is_empty() {
        system.push_str(&command.skills_context);
    }
    vec![
        Message::System { content: system },
        Message::User {
            content: command.content.clone(),
        },
    ]
}

fn execute_tools(
    cmd_id: &str,
    tools: &[Tool],
    emit: &mut dyn FnMut(Event),
    messages: &mut Vec<Message>,
) {
    for tool in tools {
        emit(Event::AgentToolStart {
            id: cmd_id.to_string(),
            name: tool.name().to_string(),
        });

        let tool_start = Instant::now();
        let result = run_tool_with_preview(tool, emit);
        let tool_elapsed = tool_start.elapsed().as_secs_f64();

        emit(Event::AgentToolEnd {
            duration_secs: tool_elapsed,
            output: result.output.clone(),
        });

        messages.push(Message::ToolResult {
            content: format!("{} result:\n{}", result.tool.name(), result.output),
        });
    }
}

fn run_tool_with_preview(tool: &Tool, emit: &mut dyn FnMut(Event)) -> crate::tools::ToolResult {
    match tool {
        Tool::EditFile {
            path,
            search,
            replace,
        } => {
            let resolved = crate::path_utils::resolve_path(path);
            match crate::diff::preview_edit(&resolved, search, replace) {
                Ok(preview) => {
                    emit(Event::PendingEdit {
                        path: path.clone(),
                        original: preview.original,
                        proposed: preview.proposed,
                        diff: preview.diff.clone(),
                    });
                    crate::tools::ToolResult {
                        tool: tool.clone(),
                        output: preview.diff,
                        success: true,
                    }
                }
                Err(e) => crate::tools::ToolResult {
                    tool: tool.clone(),
                    output: format!("Error generating preview: {}", e),
                    success: false,
                },
            }
        }
        _ => tool.execute(),
    }
}
