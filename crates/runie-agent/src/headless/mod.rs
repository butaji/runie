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

use crate::streaming_parser::{SharedResponse, SharedStreamState, StreamingHandler};
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
use runie_core::provider_event::ProviderEvent;
use runie_core::tool::{
    assign_tool_call_ids, build_assistant_message, tool_parse_error_message, ParsedToolCall,
};
use runie_core::tool::{ToolContext, ToolOutput};
use runie_provider::BuiltProviderFactory;
use std::sync::Arc;

/// Run a headless turn with a fresh runtime, a PermissionGate, and an ApprovalSink.
///
/// This is the shared helper used by `runie-cli print`, `runie-cli json`, and `runie-cli server`.
///
/// The `factory` parameter allows injecting a custom provider factory (e.g., for replay).
/// When `None`, defaults to `BuiltProviderFactory::new()`.
pub async fn run_headless_cli(
    provider_name: Option<&str>,
    provider_model: Option<&str>,
    messages: Vec<ChatMessage>,
    sink: Arc<dyn runie_core::permissions::ApprovalSink>,
    options: HeadlessCliOptions,
    factory: Option<Arc<dyn runie_core::actors::provider::ProviderFactory>>,
) -> Result<HeadlessResult> {
    let factory = factory.unwrap_or_else(|| Arc::new(BuiltProviderFactory::new()));
    let runtime = HeadlessRuntime::spawn(EventBus::<Event>::new(10), factory).await?;
    let provider = runtime.provider(provider_name, provider_model).await?;
    let opts = build_headless_options(sink, options);
    let result = run_headless_turn(messages, &provider, opts).await;
    runtime.shutdown().await;
    result
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

        let SharedResponse {
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

        let any_blocked = execute_headless_tools(
            &tool_calls,
            &mut self.messages,
            &mut self.tool_outputs,
            &self.options.permission_gate,
            self.options.on_event.as_mut(),
        )
        .await?;

        // Stop the loop if any tools were blocked (denied by permission policy).
        // The agent should not re-issue the same tool call after a denial.
        if any_blocked {
            return Ok(false);
        }

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

/// `StreamingHandler` for the headless runner.
///
/// Handles the five core streaming events (text, tool lifecycle) via the shared
/// trait. `ThinkingDelta` and `Usage` are emitted directly in the stream loop
/// since they are not part of the shared trait.
struct HeadlessHandler<'a> {
    shared: SharedStreamState,
    content: &'a mut String,
    options: &'a mut HeadlessOptions,
}

impl<'a> HeadlessHandler<'a> {
    fn new(content: &'a mut String, options: &'a mut HeadlessOptions) -> Self {
        Self {
            shared: SharedStreamState::new(),
            content,
            options,
        }
    }

    fn emit(&mut self, event: HeadlessEvent) {
        if let Some(cb) = self.options.on_event.as_mut() {
            cb(event);
        }
    }
}

impl<'a> StreamingHandler for HeadlessHandler<'a> {
    fn on_text_delta(&mut self, delta: String) {
        self.shared.push_text(&delta);
        self.content.push_str(&delta);
        self.emit(HeadlessEvent::Text {
            data: delta.clone(),
        });
        if let Some(cb) = self.options.on_chunk.as_mut() {
            cb(&delta);
        }
    }

    fn on_tool_start(&mut self, id: String, name: String) {
        self.shared.start_tool(&id, &name);
        self.emit(HeadlessEvent::ToolCallStart { id, name });
    }

    fn on_tool_input(&mut self, id: String, delta: String) {
        self.shared.append_tool_input(&id, &delta);
        self.emit(HeadlessEvent::ToolCallInputDelta { id, delta });
    }

    fn on_tool_end(&mut self, id: String) {
        self.emit(HeadlessEvent::ToolCallEnd { id: id.clone() });
        self.shared.finish_tool(&id);
    }

    fn on_finish(&mut self) {
        // Finish reason is emitted separately by the stream loop.
    }

    fn on_error(&mut self, message: String) -> Result<()> {
        Err(anyhow::anyhow!("LLM error: {}", message))
    }

    fn is_cancelled(&self) -> bool {
        false // Headless does not support cancellation.
    }
}

async fn stream_headless_response(
    provider: &dyn Provider,
    messages: &[ChatMessage],
    content: &mut String,
    options: &mut HeadlessOptions,
) -> Result<SharedResponse> {
    let tools = crate::tool_registry::build_all_schemas();
    let mut handler = HeadlessHandler::new(content, options);
    let mut stream = provider.generate_with_tools(messages.to_vec(), tools);

    // Emit ThinkingDelta and Usage directly since they are not in StreamingHandler.
    while let Some(event_result) = stream.next().await {
        let event = match event_result? {
            // Delegate these five to the shared handler.
            ProviderEvent::TextDelta(delta) => {
                handler.on_text_delta(delta);
                continue;
            }
            ProviderEvent::ToolCallStart { id, name } => {
                handler.on_tool_start(id, name);
                continue;
            }
            ProviderEvent::ToolCallInputDelta { id, delta } => {
                handler.on_tool_input(id, delta);
                continue;
            }
            ProviderEvent::ToolCallEnd { id } => {
                handler.on_tool_end(id);
                continue;
            }
            ProviderEvent::Error(e) => {
                let msg = format!("{:?}", e);
                handler.emit(HeadlessEvent::Error {
                    message: msg.clone(),
                });
                handler.on_error(msg)?;
                return Ok(handler.shared.into_response());
            }
            // Pass through the rest.
            other => other,
        };

        match event {
            ProviderEvent::ThinkingDelta(data) => {
                handler.emit(HeadlessEvent::Thinking { data });
            }
            ProviderEvent::Usage {
                input_tokens,
                output_tokens,
            } => {
                handler.emit(HeadlessEvent::Usage {
                    input_tokens,
                    output_tokens,
                });
            }
            ProviderEvent::Finish { reason } => {
                handler.emit(HeadlessEvent::End {
                    stop_reason: format!("{:?}", reason),
                    session_id: None,
                    request_id: None,
                });
                break;
            }
            _ => {}
        }
    }

    Ok(handler.shared.into_response())
}

async fn execute_headless_tools(
    tools: &[ParsedToolCall],
    messages: &mut Vec<ChatMessage>,
    tool_outputs: &mut Vec<ToolOutput>,
    gate: &PermissionGate,
    mut on_event: Option<&mut Box<dyn FnMut(HeadlessEvent) + Send>>,
) -> Result<bool> {
    let ctx = ToolContext::default();
    let mut any_blocked = false;

    for tool_call in tools {
        let output = execute_tool_call(tool_call, &ctx, gate, None).await;

        // Track if any tool was blocked (denied by permission policy)
        if output.status == runie_core::tool::ToolStatus::Blocked {
            any_blocked = true;
        }

        tool_outputs.push(output.clone());
        messages.push(tool_result_message(tool_call, &output));
        if let Some(cb) = on_event.as_mut() {
            cb(HeadlessEvent::ToolResult {
                id: tool_call.id.clone().unwrap_or_default(),
                output: output.content.clone(),
            });
        }
    }
    Ok(any_blocked)
}

#[cfg(test)]
mod tests;
