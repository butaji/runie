//! Shared headless runner for non-interactive binaries.
//!
//! `run_headless_turn` streams a single turn from a provider, optionally
//! executes any parsed tool calls, and continues the conversation for up to
//! `max_tool_rounds` rounds. The server mode sets `execute_tools: false` and
//! simply returns the streamed content.
//!
//! `run_headless_cli` is a higher-level helper that encapsulates the common
//! `spawn_headless_runtime → provider → PermissionGate → run_headless_turn`
//! pattern shared by `runie-print`, `runie-json`, and `runie-server`.

use crate::tool_runner::{execute_tool_call, tool_result_message};
use crate::PermissionGate;
use anyhow::Result;
use futures::StreamExt;
use runie_core::provider_event::ProviderEvent;
use runie_core::message::ChatMessage;
use runie_core::permissions::PermissionManager;
use runie_core::provider::Provider;
use runie_core::tool_parser::{
    assign_tool_call_ids, build_assistant_message, parse_tool_calls_fallible,
    tool_parse_error_message, ParsedToolCall, ToolParseError,
};
use runie_core::tool::{ToolContext, ToolOutput, ToolRegistry};
use runie_core::tool_stream::ToolStream;
use std::ops::ControlFlow;
use std::sync::Arc;

/// Run a headless turn with a fresh runtime, a PermissionGate, and an ApprovalSink.
///
/// This is the shared helper used by `runie-print`, `runie-json`, and `runie-server`.
pub async fn run_headless_cli(
    provider_name: Option<&str>,
    provider_model: Option<&str>,
    messages: Vec<ChatMessage>,
    sink: Arc<dyn runie_core::permissions::ApprovalSink>,
    options: HeadlessCliOptions,
) -> Result<HeadlessResult> {
    let runtime = runie_provider::spawn_headless_runtime().await;
    let provider = runtime.provider(provider_name, provider_model).await?;
    let opts = build_headless_options(sink, options);
    run_headless_turn(messages, &provider, opts).await
}

/// Options for `run_headless_cli` (subset of `HeadlessOptions` that varies per caller).
#[derive(Default)]
pub struct HeadlessCliOptions {
    pub execute_tools: bool,
    pub max_tool_rounds: usize,
    pub on_chunk: Option<Box<dyn FnMut(&str) + Send>>,
}

fn build_headless_options(
    sink: Arc<dyn runie_core::permissions::ApprovalSink>,
    opts: HeadlessCliOptions,
) -> HeadlessOptions {
    HeadlessOptions {
        execute_tools: opts.execute_tools,
        max_tool_rounds: opts.max_tool_rounds,
        on_chunk: opts.on_chunk,
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
            }
            ProviderEvent::ToolCallInputDelta { id, delta } => {
                self.tool_stream.append(&id, &delta);
            }
            ProviderEvent::ToolCallEnd { id } => self.on_tool_end(id),
            ProviderEvent::Finish { .. } => return ControlFlow::Break(()),
            ProviderEvent::Error(e) => {
                self.error = Some(format!("{:?}", e));
                return ControlFlow::Break(());
            }
            _ => {}
        }
        ControlFlow::Continue(())
    }

    fn on_text_delta(&mut self, delta: String) {
        self.text.push_str(&delta);
        self.content.push_str(&delta);
        if let Some(cb) = self.options.on_chunk.as_mut() {
            cb(&delta);
        }
    }

    fn on_tool_end(&mut self, id: String) {
        if let Some(call) = self.tool_stream.finish(&id) {
            self.tool_calls.push(call);
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
    let tools = build_tool_registry().to_openai_functions();
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

fn build_tool_registry() -> ToolRegistry {
    runie_engine::tool::builtin_registry()
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
        messages.push(tool_result_message(tool_call, &output));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tests::ensure_mock_provider;
    use runie_core::message::Role;
    use runie_core::permissions::{AutoAllowSink, PermissionManager};
    use runie_provider::MockProvider;
    use runie_testing::allow_all_gate;

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
        let _mock_guard = ensure_mock_provider().await;
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
        let _mock_guard = ensure_mock_provider().await;
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
        assert!(result.tool_outputs.len() >= 1);
    }

    #[tokio::test]
    async fn headless_runner_feeds_parse_errors_back_to_model() {
        let _mock_guard = ensure_mock_provider().await;
        let provider = MockProvider::default();
        let messages = vec![
            ChatMessage::system("You are helpful."),
            ChatMessage::user("malformed tool call"),
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

        assert!(
            result.tool_outputs.is_empty(),
            "malformed tool should not be executed"
        );
        let has_parse_error = result.messages.iter().any(|m| {
            m.role == Role::Tool && m.content().contains("Could not parse tool call")
        });
        assert!(has_parse_error, "parse error should be added to messages");
    }

    #[tokio::test]
    async fn headless_runner_executes_tool_call_markup() {
        let _mock_guard = ensure_mock_provider().await;
        let provider = MockProvider::default();
        let messages = vec![
            ChatMessage::system("You are helpful."),
            ChatMessage::user("use markup tool call"),
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

        assert_eq!(result.tool_outputs.len(), 1);
        assert_eq!(result.tool_outputs[0].tool_name, "list_dir");
        assert!(result.tool_outputs[0].tool_args.get("path").is_some());
        assert!(result.content.contains("[TOOL_CALL]"));
    }

    // Layer 1 — State/Logic: helper constructs a PermissionGate with the supplied sink.
    #[tokio::test]
    async fn headless_cli_helper_builds_gate() {
        let sink: Arc<dyn runie_core::permissions::ApprovalSink> =
            Arc::new(AutoAllowSink);
        let opts = HeadlessCliOptions {
            execute_tools: true,
            max_tool_rounds: 5,
            on_chunk: None,
        };
        let gate = PermissionGate::new(PermissionManager::default(), sink.clone());
        assert!(Arc::ptr_eq(gate.sink_ref(), &sink));
    }

    // Layer 4 — Smoke: run_headless_cli still works with a mock provider.
    #[tokio::test]
    async fn headless_cli_smoke_with_mock() {
        let _mock_guard = crate::tests::ensure_mock_provider().await;
        let sink: Arc<dyn runie_core::permissions::ApprovalSink> =
            Arc::new(AutoAllowSink);
        let messages = vec![
            ChatMessage::system("You are helpful."),
            ChatMessage::user("hello"),
        ];
        let opts = HeadlessCliOptions {
            execute_tools: false,
            max_tool_rounds: 5,
            on_chunk: None,
        };
        let result = run_headless_cli(Some("mock"), Some("echo"), messages, sink, opts)
            .await
            .unwrap();
        assert!(!result.content.is_empty());
    }
}
