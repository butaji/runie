use crate::parser::parse_tool_calls;
use crate::tools::Tool;
use crate::AgentCommand;
use anyhow::Result;
use futures::StreamExt;
use runie_core::event::Event;
use runie_core::provider::{Message, Provider};
use runie_provider::DynProvider;
use std::time::Instant;

pub async fn run_agent_turn<F>(
    provider: &DynProvider,
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
        if !run_agent_iteration(provider, command, &mut messages, &mut emit).await? {
            break;
        }
        has_intermediate_steps = true;
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

async fn run_agent_iteration<F>(
    provider: &DynProvider,
    command: &AgentCommand,
    messages: &mut Vec<Message>,
    emit: &mut F,
) -> Result<bool>
where
    F: FnMut(Event) + Send,
{
    emit(Event::AgentThinking {
        id: command.id.clone(),
    });

    let response_text = stream_response(provider, command, messages, emit).await?;
    emit(Event::AgentThoughtDone {
        id: command.id.clone(),
    });

    let tools = parse_tool_calls(&response_text);
    if tools.is_empty() {
        return Ok(false);
    }

    messages.push(Message::Assistant {
        content: response_text,
    });
    execute_tools(&command.id, &tools, emit, messages);
    Ok(true)
}

async fn stream_response<F>(
    provider: &DynProvider,
    command: &AgentCommand,
    messages: &[Message],
    emit: &mut F,
) -> Result<String>
where
    F: FnMut(Event) + Send,
{
    let mut response_text = String::new();
    let mut stream = provider.generate(messages.to_vec());
    while let Some(chunk_result) = stream.next().await {
        let chunk = chunk_result?;
        response_text.push_str(&chunk.content);
        emit(Event::AgentResponse {
            id: command.id.clone(),
            content: chunk.content,
        });
    }
    Ok(response_text)
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
