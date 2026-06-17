//! Shared headless runner for non-interactive binaries.
//!
//! `run_headless_turn` streams a single turn from a provider, optionally
//! executes any parsed tool calls, and continues the conversation for up to
//! `max_tool_rounds` rounds. The server mode sets `execute_tools: false` and
//! simply returns the streamed content.

use crate::parser::{parse_tool_calls, ParsedToolCall};
use crate::PermissionGate;
use anyhow::Result;
use futures::StreamExt;
use runie_core::message::ChatMessage;
use runie_core::permissions::{PermissionAction, PermissionContext};
use runie_core::provider::Provider;
use runie_core::tool::{ToolContext, ToolOutput};

/// Result of a headless turn.
#[derive(Debug, Clone)]
pub struct HeadlessResult {
    /// All streamed assistant text accumulated across tool rounds.
    pub content: String,
    /// Tool calls that were executed (only populated when `execute_tools` is true).
    pub tool_outputs: Vec<ToolOutput>,
}

/// Options for headless turn execution.
#[allow(clippy::type_complexity)]
pub struct HeadlessOptions {
    /// Execute tools and collect results.
    pub execute_tools: bool,
    /// Maximum number of tool-call rounds.
    pub max_tool_rounds: usize,
    /// Callback for each text chunk received from the LLM.
    pub on_chunk: Option<Box<dyn FnMut(&str) + Send>>,
    /// Permission gate for tool execution.
    pub permission_gate: PermissionGate,
}

/// Run a headless turn with the given provider.
///
/// The caller must already include the system and user messages in `messages`.
pub async fn run_headless_turn(
    messages: Vec<ChatMessage>,
    provider: &dyn Provider,
    mut options: HeadlessOptions,
) -> Result<HeadlessResult> {
    let mut messages = messages;
    let mut content = String::new();
    let mut tool_outputs = Vec::new();

    for _ in 0..options.max_tool_rounds.max(1) {
        let response_text =
            stream_response_text(provider, &messages, &mut content, &mut options).await?;
        let tools = parse_tool_calls(&response_text);
        if tools.is_empty() || !options.execute_tools {
            break;
        }

        messages.push(ChatMessage::assistant(response_text.to_string()));
        execute_headless_tools(&tools, &mut messages, &mut tool_outputs, &options.permission_gate)
            .await?;
    }

    Ok(HeadlessResult {
        content,
        tool_outputs,
    })
}

async fn stream_response_text(
    provider: &dyn Provider,
    messages: &[ChatMessage],
    content: &mut String,
    options: &mut HeadlessOptions,
) -> Result<String> {
    let mut response_text = String::new();
    let mut stream = provider.generate(messages.to_vec());
    while let Some(event_result) = stream.next().await {
        let event = event_result?;
        match event {
            runie_core::llm_event::LLMEvent::TextDelta(text) => {
                response_text.push_str(&text);
                content.push_str(&text);
                if let Some(cb) = options.on_chunk.as_mut() {
                    cb(&text);
                }
            }
            runie_core::llm_event::LLMEvent::Finish { .. } => break,
            runie_core::llm_event::LLMEvent::Error(e) => {
                return Err(anyhow::anyhow!("LLM error: {:?}", e));
            }
            _ => {}
        }
    }
    Ok(response_text)
}

async fn execute_headless_tools(
    tools: &[ParsedToolCall],
    messages: &mut Vec<ChatMessage>,
    tool_outputs: &mut Vec<ToolOutput>,
    gate: &PermissionGate,
) -> Result<()> {
    let ctx = ToolContext::default();
    let registry = runie_engine::tool::builtin_registry();

    for tool_call in tools {
        let output = execute_tool_call(&registry, tool_call, &ctx, gate).await;
        tool_outputs.push(output.clone());
        messages.push(ChatMessage::tool_result(format!(
            "{} result:\n{}",
            tool_call.name, output.content
        )));
    }
    Ok(())
}

async fn execute_tool_call(
    registry: &runie_core::tool::ToolRegistry,
    tool_call: &ParsedToolCall,
    ctx: &ToolContext,
    gate: &PermissionGate,
) -> ToolOutput {
    let tool_name = &tool_call.name;
    let perm_ctx = build_permission_context(tool_name, &tool_call.args, &ctx.working_dir);
    match gate.evaluate(&perm_ctx).await {
        PermissionAction::Allow => match registry.get(tool_name) {
            Some(tool) => tool
                .call(tool_call.args.clone(), ctx)
                .await
                .unwrap_or_else(|e| ToolOutput {
                    tool_name: tool_name.clone(),
                    tool_args: tool_call.args.clone(),
                    content: format!("Tool execution failed: {}", e),
                    bytes_transferred: None,
                    duration: std::time::Duration::from_millis(0),
                    status: runie_core::tool::ToolStatus::Error,
                }),
            None => ToolOutput {
                tool_name: tool_name.clone(),
                tool_args: tool_call.args.clone(),
                content: format!("Error: unknown tool '{}'", tool_name),
                bytes_transferred: None,
                duration: std::time::Duration::from_millis(0),
                status: runie_core::tool::ToolStatus::Error,
            },
        },
        PermissionAction::Deny | PermissionAction::Ask => ToolOutput {
            tool_name: tool_name.clone(),
            tool_args: tool_call.args.clone(),
            content: format!("Permission denied for tool '{}'", tool_name),
            bytes_transferred: None,
            duration: std::time::Duration::from_millis(0),
            status: runie_core::tool::ToolStatus::Blocked,
        },
    }
}

fn build_permission_context<'a>(
    tool: &'a str,
    input: &'a serde_json::Value,
    cwd: &'a std::path::Path,
) -> PermissionContext<'a> {
    let path = input
        .get("path")
        .and_then(|v| v.as_str())
        .map(std::path::Path::new);
    PermissionContext {
        tool,
        path,
        input: Some(input),
        cwd: Some(cwd),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tests::ensure_mock_provider;
    use runie_core::permissions::{AutoAllowSink, PermissionManager};
    use runie_provider::MockProvider;
    use std::sync::Arc;

    fn allow_all_gate() -> PermissionGate {
        PermissionGate::new(PermissionManager::default(), Arc::new(AutoAllowSink))
    }

    #[tokio::test]
    async fn headless_runner_with_mock_returns_content() {
        let provider = MockProvider::default();
        let messages = vec![
            ChatMessage::system("You are helpful."),
            ChatMessage::user("hello world"),
        ];
        let options = HeadlessOptions {
            execute_tools: false,
            max_tool_rounds: 5,
            on_chunk: None,
            permission_gate: allow_all_gate(),
        };
        let result = run_headless_turn(messages, &provider, options)
            .await
            .unwrap();
        assert!(!result.content.is_empty());
        assert!(result.tool_outputs.is_empty());
    }

    #[tokio::test]
    async fn headless_runner_executes_tool_and_returns_output() {
        ensure_mock_provider();
        let provider = MockProvider::default();
        let messages = vec![
            ChatMessage::system("You are helpful."),
            ChatMessage::user("list files"),
        ];
        let options = HeadlessOptions {
            execute_tools: true,
            max_tool_rounds: 5,
            on_chunk: None,
            permission_gate: allow_all_gate(),
        };
        let result = run_headless_turn(messages, &provider, options)
            .await
            .unwrap();
        assert!(!result.content.is_empty());
        assert_eq!(result.tool_outputs.len(), 1);
        assert_eq!(result.tool_outputs[0].tool_name, "list_dir");
        assert!(result.tool_outputs[0].tool_args.get("path").is_some());
        assert!(!result.tool_outputs[0].content.is_empty());
    }

    #[tokio::test]
    async fn headless_runner_with_execute_tools_enabled() {
        ensure_mock_provider();
        let provider = MockProvider::default();
        let messages = vec![
            ChatMessage::system("You are helpful."),
            ChatMessage::user("list files"),
        ];
        let options = HeadlessOptions {
            execute_tools: true,
            max_tool_rounds: 5,
            on_chunk: None,
            permission_gate: allow_all_gate(),
        };
        let result = run_headless_turn(messages, &provider, options)
            .await
            .unwrap();
        // With execute_tools, mock returns tool call that gets executed
        assert!(result.tool_outputs.len() >= 1);
    }
}
