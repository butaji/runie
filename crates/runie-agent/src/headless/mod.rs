//! Shared headless runner for non-interactive binaries.
//!
//! `run_headless_turn` streams a single turn from a provider, optionally
//! executes any parsed tool calls, and continues the conversation for up to
//! `max_tool_rounds` rounds. The server mode sets `execute_tools: false` and
//! simply returns the streamed content.
//!
//! `run_headless_cli` is a higher-level helper that encapsulates the common
//! `spawn_headless_runtime → provider → PermissionGate → run_headless_turn`
//! pattern shared by `runie-cli print`, `runie-cli json`, and `runie-cli server`.

use crate::tool_runner::{execute_tool_call, tool_result_message};
use crate::PermissionGate;
use anyhow::Result;
use futures::StreamExt;
use runie_core::bus::EventBus;
use runie_core::event::headless::HeadlessEvent;
use runie_core::event::Event;
use runie_core::headless_runtime::HeadlessRuntime;
use runie_core::message::ChatMessage;
use runie_core::permissions::PermissionManager;
use runie_core::provider::Provider;
use runie_provider::DynProviderFactory;
use runie_core::provider_event::ProviderEvent;
use runie_core::tool::{
    assign_tool_call_ids, build_assistant_message, parse_tool_calls_fallible,
    tool_parse_error_message, ParsedToolCall, ToolParseError,
};
use runie_core::tool::{ToolContext, ToolOutput};
use runie_core::tool_stream::ToolStream;
use std::ops::ControlFlow;
use std::sync::Arc;

/// Run a headless turn with a fresh runtime, a PermissionGate, and an ApprovalSink.
///
/// This is the shared helper used by `runie-cli print`, `runie-cli json`, and `runie-cli server`.
pub async fn run_headless_cli(
    provider_name: Option<&str>,
    provider_model: Option<&str>,
    messages: Vec<ChatMessage>,
    sink: Arc<dyn runie_core::permissions::ApprovalSink>,
    options: HeadlessCliOptions,
) -> Result<HeadlessResult> {
    let runtime = HeadlessRuntime::spawn(
        EventBus::<Event>::new(10),
        Arc::new(DynProviderFactory),
    )
    .await?;
    let provider = runtime.provider(provider_name, provider_model).await?;
    let opts = build_headless_options(sink, options);
    run_headless_turn(messages, &provider, opts).await
}

/// Options for `run_headless_cli` (subset of `HeadlessOptions` that varies per caller).
#[derive(Default)]
#[allow(clippy::type_complexity)]
pub struct HeadlessCliOptions {
    pub execute_tools: bool,
    pub max_tool_rounds: usize,
    pub on_chunk: Option<Box<dyn FnMut(&str) + Send>>,
    /// Callback for each headless event (text, tool, permission, usage, etc.).
    pub on_event: Option<Box<dyn FnMut(HeadlessEvent) + Send>>,
}

fn build_headless_options(
    sink: Arc<dyn runie_core::permissions::ApprovalSink>,
    opts: HeadlessCliOptions,
) -> HeadlessOptions {
    HeadlessOptions {
        execute_tools: opts.execute_tools,
        max_tool_rounds: opts.max_tool_rounds,
        on_chunk: opts.on_chunk,
        on_event: opts.on_event,
        permission_gate: PermissionGate::new(PermissionManager::default(), sink),
    }
}

/// Result of a headless turn.
#[derive(Debug, Clone)]
pub struct HeadlessResult {
    /// All streamed assistant text accumulated across tool rounds.
    pub content: String,
    /// Tool calls that were executed (only populated when `execute_tools` is true).
    pub tool_outputs: Vec<ToolOutput>,
    /// Final message history, including tool results.
    pub messages: Vec<ChatMessage>,
}

/// Options for headless turn execution.
// allow: fn_mut callback type is intentional for flexible on_chunk hook
#[allow(clippy::type_complexity)]
pub struct HeadlessOptions {
    /// Execute tools and collect results.
    pub execute_tools: bool,
    /// Maximum number of tool-call rounds.
    pub max_tool_rounds: usize,
    /// Callback for each text chunk received from the LLM.
    pub on_chunk: Option<Box<dyn FnMut(&str) + Send>>,
    /// Callback for each structured headless event.
    pub on_event: Option<Box<dyn FnMut(HeadlessEvent) + Send>>,
    /// Permission gate for tool execution.
    pub permission_gate: PermissionGate,
}

/// Run a headless turn with the given provider.
///
/// The caller must already include the system and user messages in `messages`.
pub async fn run_headless_turn(
    messages: Vec<ChatMessage>,
    provider: &dyn Provider,
    options: HeadlessOptions,
) -> Result<HeadlessResult> {
    let mut state = HeadlessTurnState::new(messages, options);

    for _ in 0..state.options.max_tool_rounds.max(1) {
        if !state.run_round(provider).await? {
            break;
        }
    }

    Ok(state.into_result())
}

struct HeadlessTurnState {
    messages: Vec<ChatMessage>,
    options: HeadlessOptions,
    content: String,
    tool_outputs: Vec<ToolOutput>,
}

impl HeadlessTurnState {
    fn new(messages: Vec<ChatMessage>, options: HeadlessOptions) -> Self {
        Self {
            messages,
            options,
            content: String::new(),
            tool_outputs: Vec::new(),
        }
    }

    async fn run_round(&mut self, provider: &dyn Provider) -> Result<bool> {
        let response = stream_headless_response(
            provider,
            &self.messages,
            &mut self.content,
            &mut self.options,
        )
        .await?;

        let HeadlessStreamedResponse {
            text,
            mut tool_calls,
            parse_errors,
        } = response;
        assign_tool_call_ids(&mut tool_calls);
        self.messages
            .push(build_assistant_message(&text, None, &tool_calls));
        for (i, err) in parse_errors.iter().enumerate() {
            self.messages
                .push(tool_parse_error_message(err, &format!("parse_{}", i)));
        }

        if tool_calls.is_empty() || !self.options.execute_tools {
            return Ok(false);
        }

        execute_headless_tools(
            &tool_calls,
            &mut self.messages,
            &mut self.tool_outputs,
            &self.options.permission_gate,
            self.options.on_event.as_mut(),
        )
        .await?;
        Ok(true)
    }

    fn into_result(self) -> HeadlessResult {
        HeadlessResult {
            content: self.content,
            tool_outputs: self.tool_outputs,
            messages: self.messages,
        }
    }
}

struct HeadlessStreamState<'a> {
    text: String,
    content: &'a mut String,
    options: &'a mut HeadlessOptions,
    tool_stream: ToolStream,
    tool_calls: Vec<ParsedToolCall>,
    error: Option<String>,
}

impl<'a> HeadlessStreamState<'a> {
    fn new(content: &'a mut String, options: &'a mut HeadlessOptions) -> Self {
        Self {
            text: String::new(),
            content,
            options,
            tool_stream: ToolStream::new(),
            tool_calls: Vec::new(),
            error: None,
        }
    }

    fn handle_event(&mut self, event: ProviderEvent) -> ControlFlow<()> {
        match event {
            ProviderEvent::TextDelta(delta) => self.on_text_delta(delta),
            ProviderEvent::ToolCallStart { id, name } => {
                self.tool_stream.start(&id, &name);
                self.emit(HeadlessEvent::ToolCallStart { id, name });
            }
            ProviderEvent::ToolCallInputDelta { id, delta } => {
                self.tool_stream.append(&id, &delta);
                self.emit(HeadlessEvent::ToolCallInputDelta { id, delta });
            }
            ProviderEvent::ToolCallEnd { id } => self.on_tool_end(id),
            ProviderEvent::ThinkingDelta(content) => {
                self.emit(HeadlessEvent::Thinking { data: content });
            }
            ProviderEvent::Usage {
                input_tokens,
                output_tokens,
            } => {
                self.emit(HeadlessEvent::Usage {
                    input_tokens,
                    output_tokens,
                });
            }
            ProviderEvent::Finish { reason } => {
                self.emit(HeadlessEvent::End {
                    stop_reason: format!("{:?}", reason),
                    session_id: None,
                    request_id: None,
                });
                return ControlFlow::Break(());
            }
            ProviderEvent::Error(e) => {
                self.error = Some(format!("{:?}", e));
                self.emit(HeadlessEvent::Error {
                    message: format!("{:?}", e),
                });
                return ControlFlow::Break(());
            }
            _ => {}
        }
        ControlFlow::Continue(())
    }

    fn on_text_delta(&mut self, delta: String) {
        self.text.push_str(&delta);
        self.content.push_str(&delta);
        self.emit(HeadlessEvent::Text {
            data: delta.clone(),
        });
        if let Some(cb) = self.options.on_chunk.as_mut() {
            cb(&delta);
        }
    }

    fn on_tool_end(&mut self, id: String) {
        self.emit(HeadlessEvent::ToolCallEnd { id: id.clone() });
        if let Some(call) = self.tool_stream.finish(&id) {
            self.tool_calls.push(call);
        }
    }

    fn emit(&mut self, event: HeadlessEvent) {
        if let Some(cb) = self.options.on_event.as_mut() {
            cb(event);
        }
    }

    fn into_response(mut self) -> HeadlessStreamedResponse {
        self.tool_calls.extend(self.tool_stream.finish_all());
        let mut parse_errors = Vec::new();
        if self.tool_calls.is_empty() && !self.text.is_empty() {
            for result in parse_tool_calls_fallible(&self.text) {
                match result {
                    Ok(call) => self.tool_calls.push(call),
                    Err(err) => parse_errors.push(err),
                }
            }
        }
        HeadlessStreamedResponse {
            text: self.text,
            tool_calls: self.tool_calls,
            parse_errors,
        }
    }
}

#[derive(Debug)]
struct HeadlessStreamedResponse {
    text: String,
    tool_calls: Vec<ParsedToolCall>,
    parse_errors: Vec<ToolParseError>,
}

async fn stream_headless_response(
    provider: &dyn Provider,
    messages: &[ChatMessage],
    content: &mut String,
    options: &mut HeadlessOptions,
) -> Result<HeadlessStreamedResponse> {
    let tools = build_tool_registry();
    let mut state = HeadlessStreamState::new(content, options);
    let mut stream = provider.generate_with_tools(messages.to_vec(), tools);

    while let Some(event_result) = stream.next().await {
        if let ControlFlow::Break(()) = state.handle_event(event_result?) {
            break;
        }
    }

    if let Some(err) = state.error {
        return Err(anyhow::anyhow!("LLM error: {err}"));
    }

    Ok(state.into_response())
}

fn build_tool_registry() -> Vec<serde_json::Value> {
    use crate::tool::{
        BashTool, EditFileTool, FetchDocsTool, FindDefinitionsTool, FindTool, GrepTool,
        ListDirTool, ReadFileTool, SearchTool, WriteFileTool,
    };
    use runie_core::tool::to_openai_function;
    vec![
        to_openai_function::<BashTool>(),
        to_openai_function::<ReadFileTool>(),
        to_openai_function::<WriteFileTool>(),
        to_openai_function::<EditFileTool>(),
        to_openai_function::<ListDirTool>(),
        to_openai_function::<GrepTool>(),
        to_openai_function::<FindTool>(),
        to_openai_function::<FetchDocsTool>(),
        to_openai_function::<SearchTool>(),
        to_openai_function::<FindDefinitionsTool>(),
    ]
}

async fn execute_headless_tools(
    tools: &[ParsedToolCall],
    messages: &mut Vec<ChatMessage>,
    tool_outputs: &mut Vec<ToolOutput>,
    gate: &PermissionGate,
    mut on_event: Option<&mut Box<dyn FnMut(HeadlessEvent) + Send>>,
) -> Result<()> {
    let ctx = ToolContext::default();

    for tool_call in tools {
        let output = execute_tool_call(tool_call, &ctx, gate).await;
        tool_outputs.push(output.clone());
        messages.push(tool_result_message(tool_call, &output));
        if let Some(cb) = on_event.as_mut() {
            cb(HeadlessEvent::ToolResult {
                id: tool_call.id.clone().unwrap_or_default(),
                output: output.content.clone(),
            });
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests;
