// `runie` — minimal TUI binary. Bootstraps the loop actor and runs the App.
//
// When `MINIMAX_API_KEY` is configured, the binary uses the live Minimax
// `StreamFn` adapter; without it, the deterministic placeholder remains
// available for local UI and replay-oriented development.

use std::collections::BTreeMap;
use std::io::{self, Stdout};
use std::sync::OnceLock;
use std::time::{Duration, Instant};

use anyhow::Result;
use base64::Engine;
use crossterm::event::{
    Event, EventStream, KeyCode, KeyEvent, KeyEventKind, KeyModifiers, MouseButton, MouseEventKind,
};
use crossterm::style::Print;
use crossterm::terminal::{
    disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen,
};
use crossterm::{
    event::{DisableMouseCapture, EnableMouseCapture},
    execute,
};
use ratatui::backend::CrosstermBackend;
use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::Terminal;
use ratatui::widgets::Widget;
use runie_core::commands::{classify_builtin_command, BuiltinCommandDisposition};
use runie_core::events::EventBus;
use runie_core::noninteractive::{encode_jsonl, JsonlEvent, NonInteractiveConfig, RunOutcome};
use runie_core::provider::codex::CodexWebSocketAdapter;
use runie_core::provider::stream_fn::{AssistantMessageEventStream, StreamError, StreamFn};
use runie_core::provider::ProviderActor;
use runie_core::queues::{FollowUpQueueActor, SteeringQueueActor};
use runie_core::r#loop::{LoopActor, LoopDeps};
use runie_core::state::AgentStateActor;
use runie_core::tools::executor::ToolExecHooks;
use runie_core::tools::ToolExecutorActor;
use runie_core::tools::ToolRegistry;
use runie_core::tools::{ApprovalMode, AskUserQuestionTool, BackgroundCancelTool, BackgroundJobsTool, BackgroundShellTool, BashTool, EditFileTool, GitCommitPrepareTool, GitCommitTool, GitDiffTool, GitPushTool, GitReviewTool, GitRevertTool, GitStatusTool, GitWorktreeTool, GlobTool, GrepTool, ReadFileTool, SubagentTool, TodoWriteTool, WebSearchHttpClient, WebSearchTool, WriteFileTool};
use runie_core::types::{
    AgentContext, AgentTool, AgentToolResult, AssistantMessage, AssistantMessageEvent, Model,
    QueueMode, SimpleStreamOptions, StopReason, ThemeKind, ToolCall, ToolExecutionMode,
    ToolResultContent, Usage,
};

use futures::StreamExt;
use runie_tui::app::{App, AppExit, PaletteAction, UiCommand};
use runie_tui::key::{is_quit_command, map_key, Action};
use runie_tui::widgets::{PromptOutcome, PromptWidget, Scrollback, Status, StatusBar};
use tokio::sync::{broadcast, mpsc, watch};
use tokio::task::JoinHandle;
use tokio::io::AsyncReadExt;

fn osc52_clipboard_sequence(text: &str) -> String {
    let encoded = base64::engine::general_purpose::STANDARD.encode(text);
    format!("\x1b]52;c;{encoded}\x07")
}

#[allow(clippy::too_many_lines)]
fn parse_theme_name(value: &str) -> Option<ThemeKind> {
    match value.trim().to_ascii_lowercase().replace([' ', '_', '-'], "").as_str() {
        "groknight" => Some(ThemeKind::GrokNight),
        "grokday" => Some(ThemeKind::GrokDay),
        "tokyonight" => Some(ThemeKind::TokyoNight),
        "rosepinemoon" => Some(ThemeKind::RosePineMoon),
        "oscuramidnight" => Some(ThemeKind::OscuraMidnight),
        "nightowl" => Some(ThemeKind::OscuraMidnight),
        "auto" => Some(ThemeKind::Auto),
        "terminalnative" => Some(ThemeKind::TerminalNative),
        "ayudark" => Some(ThemeKind::AyuDark),
        "ayulight" => Some(ThemeKind::AyuLight),
        "ayumirage" => Some(ThemeKind::AyuMirage),
        "catppuccinfrappe" => Some(ThemeKind::CatppuccinFrappe),
        "catppuccinlatte" => Some(ThemeKind::CatppuccinLatte),
        "catppuccinmacchiato" => Some(ThemeKind::CatppuccinMacchiato),
        "catppuccinmocha" => Some(ThemeKind::CatppuccinMocha),
        "dracula" => Some(ThemeKind::Dracula),
        "everforestdark" => Some(ThemeKind::EverforestDark),
        "everforestlight" => Some(ThemeKind::EverforestLight),
        "flexokidark" => Some(ThemeKind::FlexokiDark),
        "flexokilight" => Some(ThemeKind::FlexokiLight),
        "githubdarkdimmed" => Some(ThemeKind::GithubDarkDimmed),
        "githublight" => Some(ThemeKind::GithubLight),
        "gruvboxdark" => Some(ThemeKind::GruvboxDark),
        "gruvboxlight" => Some(ThemeKind::GruvboxLight),
        "kanagawadragon" => Some(ThemeKind::KanagawaDragon),
        "kanagawalotus" => Some(ThemeKind::KanagawaLotus),
        "kanagawawave" => Some(ThemeKind::KanagawaWave),
        "lightowl" => Some(ThemeKind::LightOwl),
        "monokaipro" => Some(ThemeKind::MonokaiPro),
        "nord" => Some(ThemeKind::Nord),
        "onedark" => Some(ThemeKind::OneDark),
        "onelight" => Some(ThemeKind::OneLight),
        "palenight" => Some(ThemeKind::Palenight),
        "rosepine" => Some(ThemeKind::RosePine),
        "rosepinedawn" => Some(ThemeKind::RosePineDawn),
        "silkcircuitdawn" => Some(ThemeKind::SilkCircuitDawn),
        "silkcircuitglow" => Some(ThemeKind::SilkCircuitGlow),
        "silkcircuitneon" => Some(ThemeKind::SilkCircuitNeon),
        "silkcircuitsoft" => Some(ThemeKind::SilkCircuitSoft),
        "silkcircuitvibrant" => Some(ThemeKind::SilkCircuitVibrant),
        "solarizeddark" => Some(ThemeKind::SolarizedDark),
        "solarizedlight" => Some(ThemeKind::SolarizedLight),
        "tokyonightmoon" => Some(ThemeKind::TokyoNightMoon),
        "tokyonightstorm" => Some(ThemeKind::TokyoNightStorm),
        _ => None,
    }
}

/// Safe, opt-in provider integration probe. It exercises the complete
/// Minimax tool-call round trip without granting the live TUI filesystem or
/// process access.
struct MinimaxEchoTool;

struct ListDirectoryTool;

#[async_trait::async_trait]
impl AgentTool for ListDirectoryTool {
    fn name(&self) -> &str { "list_dir" }
    fn label(&self) -> &str { "List directory" }
    fn description(&self) -> &str {
        "List files and directories. The path is relative to the TUI working directory."
    }
    fn parameters(&self) -> Option<serde_json::Value> {
        Some(serde_json::json!({
            "type": "object",
            "properties": { "path": { "type": "string", "description": "Directory path, default ." } }
        }))
    }
    async fn execute(
        &self,
        _tool_call_id: &str,
        args: serde_json::Value,
        _signal: Option<tokio_util::sync::CancellationToken>,
        _on_update: Option<Box<dyn Fn(serde_json::Value) + Send + Sync>>,
    ) -> Result<AgentToolResult, String> {
        let path = args.get("path").and_then(serde_json::Value::as_str).unwrap_or(".");
        let mut entries = tokio::fs::read_dir(path).await.map_err(|error| format!("list_dir {path:?}: {error}"))?;
        let mut names = Vec::new();
        while let Some(entry) = entries.next_entry().await.map_err(|error| format!("list_dir {path:?}: {error}"))? {
            let kind = if entry.file_type().await.map_err(|error| error.to_string())?.is_dir() { "/" } else { "" };
            names.push(format!("{}{kind}", entry.file_name().to_string_lossy()));
        }
        names.sort_unstable();
        Ok(AgentToolResult {
            content: vec![ToolResultContent::Text { text: names.join("\n") }],
            ..AgentToolResult::default()
        })
    }
}

#[async_trait::async_trait]
impl AgentTool for MinimaxEchoTool {
    fn name(&self) -> &str {
        "echo"
    }

    fn label(&self) -> &str {
        "Echo"
    }

    fn description(&self) -> &str {
        "Returns the supplied text unchanged. Safe integration probe."
    }

    fn parameters(&self) -> Option<serde_json::Value> {
        Some(serde_json::json!({
            "type": "object",
            "properties": {"text": {"type": "string"}},
            "required": ["text"]
        }))
    }

    async fn execute(
        &self,
        _tool_call_id: &str,
        args: serde_json::Value,
        _signal: Option<tokio_util::sync::CancellationToken>,
        _on_update: Option<Box<dyn Fn(serde_json::Value) + Send + Sync>>,
    ) -> Result<AgentToolResult, String> {
        let text = args
            .get("text")
            .and_then(serde_json::Value::as_str)
            .ok_or_else(|| "echo requires a string `text` argument".to_owned())?;
        Ok(AgentToolResult {
            content: vec![ToolResultContent::Text {
                text: text.to_owned(),
            }],
            ..AgentToolResult::default()
        })
    }
}

/// Minimal OpenAI-compatible Minimax adapter for the live binary. The core
/// provider actor still owns cancellation and lifecycle events; this adapter
/// only translates the remote response into assistant events.
struct MinimaxStream {
    client: reqwest::Client,
    base_url: String,
    model: String,
    registry: runie_core::provider_registry::ProviderRegistryActor,
}

impl MinimaxStream {
    fn from_environment(registry: runie_core::provider_registry::ProviderRegistryActor) -> Self {
        Self {
            client: reqwest::Client::new(),
            base_url: std::env::var("MINIMAX_BASE_URL")
                .unwrap_or_else(|_| "https://api.minimax.io/v1/text/chatcompletion_v2".into()),
            model: std::env::var("MINIMAX_MODEL").unwrap_or_else(|_| "MiniMax-M2.1".into()),
            registry,
        }
    }

    fn streaming_enabled() -> bool {
        std::env::var("MINIMAX_STREAM")
            .map(|value| !matches!(value.as_str(), "0" | "false" | "off"))
            .unwrap_or(true)
    }
}

fn minimax_messages(context: &AgentContext) -> Result<Vec<serde_json::Value>, StreamError> {
    let mut messages = Vec::with_capacity(context.messages.len() + 1);
    if !context.system_prompt.trim().is_empty() {
        messages.push(serde_json::json!({
            "role": "system",
            "content": context.system_prompt,
        }));
    }
    messages.extend(
        context
            .messages
            .iter()
            .map(|message| {
                serde_json::to_value(message)
                    .map(normalize_minimax_message)
                    .map_err(|error| StreamError::Invalid(error.to_string()))
            })
            .collect::<Result<Vec<_>, _>>()?,
    );
    Ok(messages)
}

fn normalize_minimax_message(mut message: serde_json::Value) -> serde_json::Value {
    let role = message
        .get("role")
        .and_then(serde_json::Value::as_str)
        .map(str::to_owned);
    if matches!(role.as_deref(), Some("toolResult" | "tool_result")) {
        normalize_minimax_tool_result(&mut message);
    } else if role.as_deref() == Some("assistant") {
        normalize_minimax_assistant(&mut message);
    }
    message
}

fn normalize_minimax_tool_result(message: &mut serde_json::Value) {
    message["role"] = serde_json::Value::String("tool".into());
    if let Some(id) = message.get("toolCallId").cloned() {
        message["tool_call_id"] = id;
        message
            .as_object_mut()
            .expect("tool result message object")
            .remove("toolCallId");
    }
}

fn normalize_minimax_assistant(message: &mut serde_json::Value) {
    let blocks = message
        .get("content")
        .and_then(serde_json::Value::as_array)
        .cloned()
        .unwrap_or_default();
    let tool_calls = blocks
        .iter()
        .filter(|block| {
            matches!(
                block.get("type").and_then(serde_json::Value::as_str),
                Some("toolCall" | "tool_call")
            )
        })
        .filter_map(minimax_tool_call_value)
        .collect::<Vec<_>>();
    if !tool_calls.is_empty() {
        message["tool_calls"] = serde_json::Value::Array(tool_calls);
        message["content"] = serde_json::Value::Null;
    }
}

fn minimax_tool_call_value(block: &serde_json::Value) -> Option<serde_json::Value> {
    let id = block.get("id")?.clone();
    let name = block.get("name")?.clone();
    let arguments = block.get("arguments")?.clone();
    Some(serde_json::json!({
        "id": id,
        "type": "function",
        "function": {
            "name": name,
            "arguments": serde_json::to_string(&arguments).ok()?
        }
    }))
}

fn minimax_tool_definitions(context: &AgentContext) -> Option<Vec<serde_json::Value>> {
    let tools = context.tools.as_ref()?;
    Some(
        tools
            .iter()
            .map(|tool| {
                serde_json::json!({
                    "type": "function",
                    "function": {
                        "name": tool.name(),
                        "description": tool.description(),
                        "parameters": tool.parameters().unwrap_or_else(|| serde_json::json!({
                            "type": "object",
                            "properties": {}
                        }))
                    }
                })
            })
            .collect(),
    )
}

fn minimax_error(payload: &serde_json::Value, status: reqwest::StatusCode) -> StreamError {
    let message = payload
        .pointer("/base_resp/status_msg")
        .and_then(serde_json::Value::as_str)
        .or_else(|| {
            payload
                .pointer("/error/message")
                .and_then(serde_json::Value::as_str)
        })
        .unwrap_or("Minimax request failed");
    StreamError::Provider {
        message: message.into(),
        status: Some(status.as_u16()),
        headers: std::collections::HashMap::new(),
    }
}

fn minimax_status_code(payload: &serde_json::Value) -> Option<i64> {
    let value = payload.pointer("/base_resp/status_code")?;
    value
        .as_i64()
        .or_else(|| value.as_str()?.parse::<i64>().ok())
}

fn minimax_usage(payload: &serde_json::Value) -> Usage {
    let usage = payload.pointer("/usage");
    let read = |names: &[&str]| {
        names
            .iter()
            .find_map(|name| usage?.get(*name)?.as_u64())
            .unwrap_or_default()
    };
    Usage {
        input: read(&["prompt_tokens", "input_tokens"]),
        output: read(&["completion_tokens", "output_tokens"]),
        total_tokens: read(&["total_tokens"]),
        ..Usage::default()
    }
}

fn minimax_events(payload: &serde_json::Value) -> Vec<AssistantMessageEvent> {
    let choice = payload.pointer("/choices/0/message").unwrap_or(payload);
    let tool_calls = minimax_tool_calls(choice);
    let text = choice
        .get("content")
        .and_then(serde_json::Value::as_str)
        .unwrap_or_default()
        .to_owned();
    let reasoning = choice
        .get("reasoning_content")
        .and_then(serde_json::Value::as_str)
        .filter(|text| !text.is_empty())
        .map(str::to_owned);
    let mut events = vec![AssistantMessageEvent::Start {
        partial: Default::default(),
    }];
    append_minimax_tool_events(&mut events, &tool_calls);
    append_minimax_content_events(&mut events, reasoning, text);
    events.push(AssistantMessageEvent::Done {
        stop_reason: if tool_calls.is_empty() {
            StopReason::Stop
        } else {
            StopReason::ToolUse
        },
        usage: minimax_usage(payload),
        message: None,
    });
    events
}

fn minimax_tool_calls(choice: &serde_json::Value) -> Vec<ToolCall> {
    choice
        .get("tool_calls")
        .and_then(serde_json::Value::as_array)
        .map(|calls| calls.iter().filter_map(minimax_tool_call).collect())
        .unwrap_or_default()
}

fn minimax_tool_call(call: &serde_json::Value) -> Option<ToolCall> {
    let function = call.get("function")?;
    let arguments = function
        .get("arguments")
        .and_then(serde_json::Value::as_str)
        .and_then(|value| serde_json::from_str(value).ok())
        .or_else(|| function.get("arguments").cloned())
        .unwrap_or_else(|| serde_json::json!({}));
    Some(ToolCall {
        id: call.get("id")?.as_str()?.to_owned(),
        name: function.get("name")?.as_str()?.to_owned(),
        arguments,
        thought_signature: None,
    })
}

fn append_minimax_tool_events(events: &mut Vec<AssistantMessageEvent>, tool_calls: &[ToolCall]) {
    for (index, tool_call) in tool_calls.iter().enumerate() {
        events.push(AssistantMessageEvent::ToolCallStart {
            index,
            partial: Default::default(),
        });
        events.push(AssistantMessageEvent::ToolCallEnd {
            index,
            tool_call: tool_call.clone(),
            partial: Default::default(),
        });
    }
}

fn append_minimax_content_events(
    events: &mut Vec<AssistantMessageEvent>,
    reasoning: Option<String>,
    text: String,
) {
    if let Some(reasoning) = reasoning {
        events.push(AssistantMessageEvent::ThinkingDelta {
            index: 0,
            delta: reasoning.clone(),
            partial: Default::default(),
        });
        events.push(AssistantMessageEvent::ThinkingEnd {
            index: 0,
            content: reasoning,
            elapsed_ms: None,
            partial: Default::default(),
        });
    }
    if !text.is_empty() {
        events.push(AssistantMessageEvent::TextDelta {
            index: 0,
            delta: text,
            partial: Default::default(),
        });
    }
}

fn minimax_stream_error(message: impl Into<String>) -> AssistantMessageEvent {
    AssistantMessageEvent::Error {
        reason: StopReason::Error,
        error: AssistantMessage {
            content: vec![runie_core::types::AssistantContent::Text {
                text: message.into(),
            }],
            ..AssistantMessage::default()
        },
    }
}

fn minimax_stream_delta(payload: &serde_json::Value, field: &str) -> Option<String> {
    payload
        .pointer("/choices/0/delta")
        .and_then(|value| value.get(field))
        .and_then(serde_json::Value::as_str)
        .filter(|value| !value.is_empty())
        .map(str::to_owned)
}

#[allow(
    clippy::too_many_lines,
    reason = "the SSE event reducer keeps provider chunk ordering and terminal handling together"
)]
fn minimax_sse_stream(response: reqwest::Response) -> AssistantMessageEventStream {
    let output = async_stream::stream! {
        let response = response;
        let mut bytes = response.bytes_stream();
        let mut buffer = String::new();
        let mut reasoning = String::new();
        let mut usage = Usage::default();
        let mut tool_calls: BTreeMap<usize, (String, String, String)> = BTreeMap::new();
        yield AssistantMessageEvent::Start { partial: Default::default() };
        while let Some(chunk) = bytes.next().await {
            let chunk = match minimax_chunk_text(chunk) {
                Ok(chunk) => chunk,
                Err(error) => { yield minimax_stream_error(error); return; }
            };
            buffer.push_str(&String::from_utf8_lossy(&chunk));
            while let Some(newline) = buffer.find('\n') {
                let line = buffer[..newline].trim_end_matches('\r').to_owned();
                buffer.drain(..=newline);
                let events = match minimax_line_events(&line, &mut reasoning, &mut tool_calls, &mut usage) {
                    Ok(events) => events,
                    Err(error) => {
                        yield minimax_stream_error(error);
                        return;
                    }
                };
                for event in events {
                    yield event;
                }
            }
        }
        if !buffer.trim().is_empty() { yield minimax_stream_error("Incomplete Minimax stream event"); return; }
        for event in minimax_stream_completion(reasoning, tool_calls, usage) {
            yield event;
        }
    };
    Box::pin(output)
}

fn minimax_chunk_text<T>(chunk: Result<T, reqwest::Error>) -> Result<T, String> {
    chunk.map_err(|error| format!("Minimax stream failed: {error}"))
}

fn minimax_line_events(
    line: &str,
    reasoning: &mut String,
    tool_calls: &mut BTreeMap<usize, (String, String, String)>,
    usage: &mut Usage,
) -> Result<Vec<AssistantMessageEvent>, String> {
    let Some(data) = line.strip_prefix("data:").map(str::trim) else {
        return Ok(Vec::new());
    };
    if data == "[DONE]" {
        return Ok(Vec::new());
    }
    let payload = serde_json::from_str::<serde_json::Value>(data)
        .map_err(|error| format!("Invalid Minimax stream event: {error}"))?;
    if let Some(error) = minimax_stream_failure(&payload) {
        return Err(error);
    }
    let mut events = minimax_content_events(&payload, reasoning);
    events.extend(minimax_tool_call_events(&payload, tool_calls));
    update_minimax_usage(usage, &payload);
    Ok(events)
}

fn minimax_stream_completion(
    reasoning: String,
    tool_calls: BTreeMap<usize, (String, String, String)>,
    usage: Usage,
) -> Vec<AssistantMessageEvent> {
    let mut events = Vec::new();
    if !reasoning.is_empty() {
        events.push(AssistantMessageEvent::ThinkingEnd {
            index: 0,
            content: reasoning,
            elapsed_ms: None,
            partial: Default::default(),
        });
    }
    let has_tool_calls = !tool_calls.is_empty();
    for (index, (id, name, arguments)) in tool_calls {
        let arguments = serde_json::from_str(&arguments)
            .unwrap_or_else(|_| serde_json::json!({"raw": arguments}));
        events.push(AssistantMessageEvent::ToolCallEnd {
            index,
            tool_call: ToolCall {
                id,
                name,
                arguments,
                thought_signature: None,
            },
            partial: Default::default(),
        });
    }
    events.push(AssistantMessageEvent::Done {
        stop_reason: if has_tool_calls {
            StopReason::ToolUse
        } else {
            StopReason::Stop
        },
        usage,
        message: None,
    });
    events
}

fn minimax_tool_call_events(
    payload: &serde_json::Value,
    tool_calls: &mut BTreeMap<usize, (String, String, String)>,
) -> Vec<AssistantMessageEvent> {
    let mut events = Vec::new();
    let Some(calls) = payload
        .pointer("/choices/0/delta/tool_calls")
        .and_then(serde_json::Value::as_array)
    else {
        return events;
    };
    for call in calls {
        let index = call
            .get("index")
            .and_then(serde_json::Value::as_u64)
            .unwrap_or_default() as usize;
        let entry = tool_calls.entry(index).or_default();
        append_tool_call_events(&mut events, call, entry, index);
    }
    events
}

fn minimax_stream_failure(payload: &serde_json::Value) -> Option<String> {
    if !minimax_status_code(payload).is_some_and(|code| code != 0) {
        return None;
    }
    Some(
        payload
            .pointer("/base_resp/status_msg")
            .and_then(serde_json::Value::as_str)
            .unwrap_or("Minimax stream request failed")
            .to_owned(),
    )
}

fn update_minimax_usage(usage: &mut Usage, payload: &serde_json::Value) {
    let parsed = minimax_usage(payload);
    if parsed.total_tokens != 0 {
        *usage = parsed;
    }
}

fn minimax_content_events(
    payload: &serde_json::Value,
    reasoning: &mut String,
) -> Vec<AssistantMessageEvent> {
    let mut events = Vec::new();
    if let Some(delta) = minimax_stream_delta(payload, "reasoning_content") {
        reasoning.push_str(&delta);
        events.push(AssistantMessageEvent::ThinkingDelta {
            index: 0,
            delta,
            partial: Default::default(),
        });
    }
    if let Some(delta) = minimax_stream_delta(payload, "content") {
        events.push(AssistantMessageEvent::TextDelta {
            index: 0,
            delta,
            partial: Default::default(),
        });
    }
    events
}

fn append_tool_call_events(
    events: &mut Vec<AssistantMessageEvent>,
    call: &serde_json::Value,
    entry: &mut (String, String, String),
    index: usize,
) {
    if let Some(id) = call.get("id").and_then(serde_json::Value::as_str) {
        entry.0 = id.to_owned();
        events.push(AssistantMessageEvent::ToolCallStart {
            index,
            partial: Default::default(),
        });
    }
    let Some(function) = call.get("function") else {
        return;
    };
    if let Some(name) = function.get("name").and_then(serde_json::Value::as_str) {
        entry.1.push_str(name);
    }
    if let Some(arguments) = function
        .get("arguments")
        .and_then(serde_json::Value::as_str)
    {
        entry.2.push_str(arguments);
        events.push(AssistantMessageEvent::ToolCallDelta {
            index,
            delta: arguments.to_owned(),
            partial: Default::default(),
        });
    }
}

#[async_trait::async_trait]
impl StreamFn for MinimaxStream {
    async fn list_models(&self) -> Result<Vec<Model>, StreamError> {
        let provider = self
            .registry
            .snapshot()
            .providers
            .into_iter()
            .find(|provider| provider.id == "minimax" && provider.connected);
        let Some(provider) = provider else {
            return Ok(Vec::new());
        };
        let models = runie_core::provider_registry::supported_providers()
            .iter()
            .find(|supported| supported.id == "minimax")
            .map(|supported| supported.models)
            .unwrap_or(&[]);
        Ok(models
            .iter()
            .map(|id| Model {
                id: (*id).to_owned(),
                name: (*id).to_owned(),
                api: "openai-completions".into(),
                provider: "minimax".into(),
                base_url: provider.base_url.clone(),
                ..Model::default()
            })
            .collect())
    }

    async fn stream(
        &self,
        _model: &Model,
        context: &AgentContext,
        options: Option<SimpleStreamOptions>,
    ) -> Result<AssistantMessageEventStream, StreamError> {
        let (response, status, is_sse) = fetch_minimax_response(self, context, options).await?;
        if Self::streaming_enabled() && status.is_success() && is_sse {
            return Ok(minimax_sse_stream(response));
        }
        let payload = response
            .json::<serde_json::Value>()
            .await
            .map_err(|error| StreamError::Api(error.to_string()))?;
        if !status.is_success() || minimax_status_code(&payload).is_some_and(|code| code != 0) {
            return Err(minimax_error(&payload, status));
        }
        let events = minimax_events(&payload);
        Ok(Box::pin(futures::stream::iter(events)))
    }
}

#[allow(clippy::too_many_lines)]
async fn fetch_minimax_response(
    stream: &MinimaxStream,
    context: &AgentContext,
    options: Option<SimpleStreamOptions>,
) -> Result<(reqwest::Response, reqwest::StatusCode, bool), StreamError> {
    let configured = stream
        .registry
        .snapshot()
        .providers
        .into_iter()
        .find(|provider| provider.id == "minimax" && provider.connected);
    let key = options
        .as_ref()
        .and_then(|options| options.api_key.clone())
        .or_else(|| configured.as_ref().and_then(|provider| provider.api_key.clone()))
        .or_else(|| std::env::var("MINIMAX_API_KEY").ok())
        .ok_or_else(|| StreamError::Api("MINIMAX_API_KEY is not configured".into()))?;
    let endpoint = configured
        .as_ref()
        .map(|provider| provider.base_url.as_str())
        .filter(|base_url| !base_url.is_empty())
        .unwrap_or(&stream.base_url);
    let model = configured
        .as_ref()
        .and_then(|provider| provider.selected_model.as_deref())
        .unwrap_or(&stream.model);
    let mut body = serde_json::json!({
        "model": model,
        "messages": minimax_messages(context)?,
            "stream": MinimaxStream::streaming_enabled(),
    });
    if let Some(tools) = minimax_tool_definitions(context) {
        body["tools"] = serde_json::Value::Array(tools);
    }
    let response = stream
        .client
        .post(endpoint)
        .bearer_auth(key)
        .json(&body)
        .send()
        .await
        .map_err(|error| StreamError::Network(error.to_string()))?;
    let status = response.status();
    let is_sse = response
        .headers()
        .get(reqwest::header::CONTENT_TYPE)
        .and_then(|value| value.to_str().ok())
        .is_some_and(|value| value.starts_with("text/event-stream"));
    Ok((response, status, is_sse))
}

enum InputEvent {
    Key(KeyEvent),
    Paste(String),
    Mouse(i32),
    MouseSelectionStart(u16, u16),
    MouseSelectionExtend(u16, u16),
    MouseSelectionCommit,
}

enum InputConfig {
    ScrollViewport(u16),
    SelectionOrigin { row: u16, column: u16 },
}

fn mouse_selection_input(
    kind: MouseEventKind,
    row: u16,
    column: u16,
    origin: (u16, u16),
) -> Option<InputEvent> {
    let position = || {
        (
            row.saturating_sub(origin.0),
            column.saturating_sub(origin.1),
        )
    };
    match kind {
        MouseEventKind::Down(MouseButton::Left) => {
            let (row, column) = position();
            Some(InputEvent::MouseSelectionStart(row, column))
        }
        MouseEventKind::Drag(MouseButton::Left) => {
            let (row, column) = position();
            Some(InputEvent::MouseSelectionExtend(row, column))
        }
        MouseEventKind::Up(MouseButton::Left) => Some(InputEvent::MouseSelectionCommit),
        _ => None,
    }
}

fn render_command_palette(
    area: Rect,
    buf: &mut Buffer,
    query: &str,
    selected: usize,
    skills: &[String],
    theme: runie_core::types::ThemeKind,
) {
    let frame = runie_tui_model::DialogFrame {
        spec: runie_tui_model::COMMAND_DIALOG,
        query: query.to_owned(),
        selected,
    };
    let rows = runie_tui_model::palette_display_rows(query, skills);
    ratatui::widgets::Widget::render(
        runie_tui::widgets::DialogWidget::new(frame, rows).with_theme(theme),
        area,
        buf,
    );
}

fn render_shared_dialog(
    area: Rect,
    buf: &mut Buffer,
    spec: runie_tui_model::DialogSpec,
    query: String,
    selected: usize,
    rows: Vec<String>,
    theme: runie_core::types::ThemeKind,
) {
    let frame = runie_tui_model::DialogFrame { spec, query, selected };
    ratatui::widgets::Widget::render(
        runie_tui::widgets::DialogWidget::new(frame, rows).with_theme(theme),
        area,
        buf,
    );
}

fn render_shortcuts_dialog(area: Rect, buf: &mut Buffer, theme: runie_core::types::ThemeKind) {
    render_shared_dialog(
        area,
        buf,
        runie_tui_model::SHORTCUTS_DIALOG,
        String::new(),
        0,
        [
            "Enter  send".into(),
            "Shift+Tab  cycle mode".into(),
            "Ctrl+C  clear / abort".into(),
            "Esc  clear prompt".into(),
            "Ctrl+L  file search".into(),
            "e  fold/unfold feed".into(),
        ]
        .into(),
        theme,
    );
}

fn render_model_selector(
    area: Rect,
    buf: &mut Buffer,
    ui: &runie_tui::app::UiState,
    theme: runie_core::types::ThemeKind,
) {
    render_shared_dialog(
        area,
        buf,
        runie_tui_model::MODEL_SELECTOR_DIALOG,
        ui.model_selector_query.clone(),
        ui.model_selector_index,
        ui.model_selector_rows.clone(),
        theme,
    );
}

fn render_session_dialog(
    area: Rect,
    buf: &mut Buffer,
    snapshot: &runie_core::session::SessionSnapshot,
    theme: runie_core::types::ThemeKind,
) {
    let stats = snapshot.stats();
    render_shared_dialog(
        area,
        buf,
        runie_tui_model::SESSION_DIALOG,
        String::new(),
        0,
        vec![
            format!("Messages: {}", stats.message_count),
            format!("Tokens: {}", stats.total_tokens),
            format!("Cost: ${:.4}", stats.cost_total),
            format!("Sequence: {}", snapshot.sequence),
        ],
        theme,
    );
}

fn render_changelog_dialog(area: Rect, buf: &mut Buffer, theme: runie_core::types::ThemeKind) {
    render_shared_dialog(
        area,
        buf,
        runie_tui_model::CHANGELOG_DIALOG,
        String::new(),
        0,
        vec!["No changelog entries found.".into()],
        theme,
    );
}

fn render_command_result_dialog(
    area: Rect,
    buf: &mut Buffer,
    result: &str,
    theme: runie_core::types::ThemeKind,
) {
    render_shared_dialog(
        area,
        buf,
        runie_tui_model::COMMAND_RESULT_DIALOG,
        String::new(),
        0,
        result.lines().map(str::to_owned).collect(),
        theme,
    );
}

fn render_user_question(
    area: Rect,
    buf: &mut Buffer,
    ui: &runie_tui::app::UiState,
    theme: runie_core::types::ThemeKind,
) {
    let Some(question) = ui.user_question.as_ref() else { return };
    let rows = std::iter::once(question.request.question.clone())
        .chain(question.request.options.iter().enumerate().map(|(index, option)| {
            let mark = if ui.user_question_selected.contains(&index) { '✓' } else { ' ' };
            format!("[{mark}] {}", option.label)
        }))
        .collect();
    let selected = ui.dialog_stack.top().map(|frame| frame.selected).unwrap_or_default() + 1;
    let frame = runie_tui_model::DialogFrame {
        spec: runie_tui_model::USER_QUESTION_DIALOG,
        query: String::new(),
        selected,
    };
    runie_tui::widgets::DialogWidget::new(frame, rows).with_theme(theme).render(area, buf);
}

fn render_palette_parameters(
    area: Rect,
    buf: &mut Buffer,
    ui: &runie_tui::app::UiState,
    theme: runie_core::types::ThemeKind,
) {
    let Some(frame) = ui.dialog_stack.top() else { return };
    let Some(action) = ui.palette_parameter_action.as_ref() else { return };
    let rows = if *action == PaletteAction::SelectTheme {
        theme_rows()
    } else if *action == PaletteAction::SetEffort {
        ui.palette_parameter_options.clone()
    } else {
        vec![format!("{} · {}", action.slash_command(), action.parameter_hint())]
    };
    render_shared_dialog(
        area,
        buf,
        if *action == PaletteAction::SelectTheme {
            runie_tui_model::THEME_SELECTOR_DIALOG
        } else {
            frame.spec.clone()
        },
        frame.query.clone(),
        frame.selected,
        rows,
        theme,
    );
}

fn theme_rows() -> Vec<String> {
    runie_tui_model::theme_labels()
    .into_iter()
    .enumerate()
    .map(|(index, name)| format!("{:>2}  {name}", index + 1))
    .collect()
}

fn render_file_selector(
    area: Rect,
    buf: &mut Buffer,
    prompt: &runie_tui_model::PromptSnapshot,
    theme: runie_core::types::ThemeKind,
) {
    let query = &prompt.text;
    let rows = prompt
        .file_candidates
        .iter()
        .filter(|path| query.is_empty() || path.contains(query))
        .take(12)
        .cloned()
        .collect();
    let frame = runie_tui_model::DialogFrame {
        spec: runie_tui_model::FILE_SELECTOR_DIALOG,
        query: query.clone(),
        selected: prompt.file_candidate_index,
    };
    ratatui::widgets::Widget::render(
        runie_tui::widgets::DialogWidget::new(frame, rows).with_theme(theme),
        area,
        buf,
    );
}

async fn discover_skill_names() -> Vec<String> {
    let mut roots = vec![std::path::PathBuf::from(".agents/skills")];
    if let Some(home) = std::env::var_os("HOME") {
        roots.push(std::path::PathBuf::from(home).join(".agents/skills"));
    }
    let mut names = Vec::new();
    for root in roots {
        let Ok(mut entries) = tokio::fs::read_dir(root).await else {
            continue;
        };
        while let Ok(Some(entry)) = entries.next_entry().await {
            let path = entry.path();
            if path.is_dir()
                && tokio::fs::try_exists(path.join("SKILL.md"))
                    .await
                    .unwrap_or(false)
            {
                if let Some(name) = path.file_name().and_then(|name| name.to_str()) {
                    names.push(name.to_owned());
                }
            }
        }
    }
    names.sort();
    names.dedup();
    names
}

fn render_compact_hint(area: Rect, buf: &mut Buffer, theme: runie_core::types::ThemeKind) {
    let line = ratatui::text::Line::from(vec![
        ratatui::text::Span::styled(
            "Tight on space? Try ",
            runie_tui::appearance::muted_style_for(theme),
        ),
        ratatui::text::Span::styled(
            "/compact-mode",
            runie_tui::appearance::base_style_for(theme)
                .add_modifier(ratatui::style::Modifier::BOLD),
        ),
        ratatui::text::Span::styled("", runie_tui::appearance::header_path_style_for(theme)),
    ])
    .style(runie_tui::appearance::base_style_for(theme));
    ratatui::widgets::Widget::render(
        ratatui::widgets::Paragraph::new(line),
        Rect {
            x: area.x,
            y: area.y.saturating_sub(1),
            width: area.width,
            height: 1,
        },
        buf,
    );
}

fn current_branch() -> &'static str {
    static BRANCH: OnceLock<String> = OnceLock::new();
    BRANCH
        .get_or_init(|| {
            std::process::Command::new("git")
                .args(["rev-parse", "--abbrev-ref", "HEAD"])
                .output()
                .ok()
                .and_then(|output| {
                    output
                        .status
                        .success()
                        .then(|| String::from_utf8_lossy(&output.stdout).trim().to_owned())
                })
                .filter(|branch| !branch.is_empty())
                .unwrap_or_else(|| "main".into())
        })
        .as_str()
}

fn repository_label() -> String {
    let path = std::env::current_dir().unwrap_or_else(|_| std::path::PathBuf::from("runie"));
    let home = std::env::var_os("HOME").map(std::path::PathBuf::from);
    runie_tui_model::repository_label(&path, home.as_deref())
}

fn render_header(area: Rect, buf: &mut Buffer, meter: &str, theme: runie_core::types::ThemeKind) {
    if area.width == 0 || area.height == 0 {
        return;
    }
    // Grok's working view keeps this chrome deliberately quiet: branch icon
    // at the transcript rail, repository path after it, no product/version
    // badge competing with the feed.
    let left = ratatui::widgets::Paragraph::new(ratatui::text::Line::from(vec![
        ratatui::text::Span::styled(
            format!(" {}", current_branch()),
            // Grok dims the branch identity while keeping the repository
            // path on its dedicated semantic token.
            runie_tui::appearance::base_style_for(theme)
                .add_modifier(ratatui::style::Modifier::DIM),
        ),
        ratatui::text::Span::raw(" "),
        ratatui::text::Span::styled(
            repository_label(),
            runie_tui::appearance::header_path_style_for(theme),
        ),
    ]));
    ratatui::widgets::Widget::render(left, area, buf);
    let x = area.right().saturating_sub(meter.len() as u16);
    buf.set_string(
        x,
        area.y,
        meter,
        // The context meter is primary header chrome in Grok, not muted
        // transcript text; the value itself remains actor-owned.
        runie_tui::appearance::header_meter_style_for(theme),
    );
}

fn render_live_ready_footer(area: Rect, buf: &mut Buffer, theme: runie_core::types::ThemeKind) {
    use ratatui::widgets::{Paragraph, Widget};

    let muted_style = runie_tui::appearance::muted_style_for(theme);
    let spans = runie_tui::appearance::footer_hotkey_actions(
        theme,
        [("Shift+Tab", "mode"), ("Ctrl+x", "shortcuts")],
    );
    Paragraph::new(ratatui::text::Line::from(spans))
        .style(muted_style)
        .render(area, buf);
}

#[derive(Debug, Default)]
struct ThemePreview {
    original: Option<runie_core::types::ThemeKind>,
}

impl ThemePreview {
    fn begin(&mut self, current: runie_core::types::ThemeKind) {
        self.original.get_or_insert(current);
    }

    fn cancel(&mut self) -> Option<runie_core::types::ThemeKind> {
        self.original.take()
    }

    fn commit(&mut self) {
        self.original = None;
    }
}

#[tokio::main(flavor = "multi_thread", worker_threads = 2)]
async fn main() -> Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
        .with_target(false)
        .init();

    let args = std::env::args().skip(1).collect::<Vec<_>>();
    if args.iter().any(|arg| arg == "--jsonl") {
        let config = NonInteractiveConfig::parse(&args).map_err(anyhow::Error::msg)?;
        let outcome = run_noninteractive(config).await;
        std::process::exit(i32::from(outcome.exit_code()));
    }

    // Resolve repository metadata before entering the redraw loop; render
    // paths only read the cached projection.
    let _ = current_branch();
    let terminal_native = std::env::args().any(|arg| arg == "--terminal-native");
    let mut terminal = setup_terminal()?;
    let res = run_app(&mut terminal, terminal_native).await;
    restore_terminal(&mut terminal)?;
    if let Err(error) = res {
        eprintln!("runie TUI error: {error:#}");
    }
    Ok(())
}

async fn run_noninteractive(config: NonInteractiveConfig) -> RunOutcome {
    let auto_approve = config.auto_approve;
    let prompt = match noninteractive_prompt(config.prompt).await {
        Ok(prompt) => prompt,
        Err(message) => {
            emit_jsonl(vec![
                JsonlEvent::Error { message },
                JsonlEvent::Finished {
                    outcome: RunOutcome::Failed,
                },
            ]);
            return RunOutcome::Failed;
        }
    };

    let app = build_live_app().await;
    if auto_approve {
        app.approval_mode.set(runie_core::tools::ApprovalMode::Auto);
    }
    let run_id = format!("run-{}", std::process::id());
    let mut events = vec![JsonlEvent::Started { run_id }];
    let messages = vec![runie_core::types::AgentMessage::User(
        runie_core::types::UserMessage {
            content: vec![runie_core::types::UserContent::Text { text: prompt }],
            timestamp: 0,
        },
    )];
    let outcome = match app
        .loop_actor
        .prompt_with_events(messages, AgentContext::default())
        .await
    {
        Ok((messages, provider_events)) => {
            events.extend(jsonl_events_for_messages(&messages));
            match runie_core::noninteractive::forward_provider_events(&provider_events) {
                Ok(provider_events) => events.extend(provider_events),
                Err(message) => events.push(JsonlEvent::Error { message }),
            }
            RunOutcome::Completed
        }
        Err(error) => {
            events.push(JsonlEvent::Error {
                message: error.to_string(),
            });
            RunOutcome::Failed
        }
    };
    events.push(JsonlEvent::Finished { outcome });
    emit_jsonl(events);
    outcome
}

async fn noninteractive_prompt(prompt: Option<String>) -> Result<String, String> {
    if let Some(prompt) = prompt {
        if prompt.trim().is_empty() {
            return Err("--prompt must not be empty in --jsonl mode".into());
        }
        return Ok(prompt);
    }
    let mut input = String::new();
    tokio::io::stdin()
        .read_to_string(&mut input)
        .await
        .map_err(|error| format!("could not read stdin: {error}"))?;
    (!input.trim().is_empty())
        .then_some(input)
        .ok_or_else(|| "--prompt or non-empty stdin is required in --jsonl mode".into())
}

fn jsonl_events_for_messages(messages: &[runie_core::types::AgentMessage]) -> Vec<JsonlEvent> {
    messages
        .iter()
        .flat_map(|message| match message {
            runie_core::types::AgentMessage::Assistant(assistant) => assistant
                .content
                .iter()
                .filter_map(|content| match content {
                    runie_core::types::AssistantContent::Text { text }
                    | runie_core::types::AssistantContent::Thinking { text } => {
                        Some(JsonlEvent::Text { text: text.clone() })
                    }
                    runie_core::types::AssistantContent::ToolCall(_) => None,
                })
                .collect::<Vec<_>>(),
            runie_core::types::AgentMessage::ToolResult(result) => vec![JsonlEvent::Tool {
                name: result.tool_name.clone(),
                result: serde_json::json!({
                    "details": result.details,
                    "is_error": result.is_error,
                    "content": result.content,
                }),
            }],
            _ => Vec::new(),
        })
        .collect()
}

fn emit_jsonl(events: Vec<JsonlEvent>) {
    match encode_jsonl(&events) {
        Ok(output) => print!("{output}"),
        Err(error) => eprintln!("could not encode JSONL output: {error}"),
    }
}

fn setup_terminal() -> Result<Terminal<CrosstermBackend<Stdout>>> {
    enable_raw_mode()?;
    execute!(io::stdout(), EnterAlternateScreen, EnableMouseCapture)?;
    let backend = CrosstermBackend::new(io::stdout());
    let mut terminal = Terminal::new(backend)?;
    // Alternate-screen terminals can retain the previous buffer until the
    // first differential draw. Clear it explicitly so startup is visible
    // even when the terminal reports an unchanged frame.
    terminal.clear()?;
    terminal.hide_cursor()?;
    Ok(terminal)
}

fn restore_terminal(terminal: &mut Terminal<CrosstermBackend<Stdout>>) -> Result<()> {
    disable_raw_mode()?;
    execute!(
        terminal.backend_mut(),
        DisableMouseCapture,
        LeaveAlternateScreen
    )?;
    terminal.show_cursor()?;
    Ok(())
}

async fn build_live_app() -> App {
    let bus = EventBus::new();
    let question_broker = runie_core::tools::UserQuestionBroker::default();
    let approval_mode = runie_core::tools::ApprovalModeStore::default();
    let subagent_loop = std::sync::Arc::new(tokio::sync::OnceCell::<LoopActor>::new());
    let background_actor = runie_core::background::BackgroundProcessActor::new();
    let todo_actor = runie_core::tools::TodoActor::default();
    let state = AgentStateActor::new();
    let steering = SteeringQueueActor::new();
    let follow_up = FollowUpQueueActor::new();
    let mut tools = ToolRegistry::new();
    runie_core::register_tools!(
        tools;
        ReadFileTool,
        WriteFileTool,
        EditFileTool,
        GrepTool,
        GlobTool,
        BashTool,
        AskUserQuestionTool
        ,TodoWriteTool
        ,SubagentTool
        ,WebSearchTool
        ,BackgroundShellTool
        ,BackgroundJobsTool
        ,BackgroundCancelTool
        ,GitStatusTool
        ,GitDiffTool
        ,GitReviewTool
        ,GitWorktreeTool
        ,GitCommitPrepareTool
        ,GitCommitTool
        ,GitPushTool
        ,GitRevertTool
    );
    tools.register(std::sync::Arc::new(ListDirectoryTool));
    if std::env::var("MINIMAX_ENABLE_ECHO_TOOL").is_ok_and(|value| value == "1") {
        tools.register(std::sync::Arc::new(MinimaxEchoTool));
    }
    let tool_executor = ToolExecutorActor::new_live(std::sync::Arc::new(tools));
    let provider_registry = runie_core::provider_registry::ProviderRegistryActor::new(
        runie_core::provider_registry::ProviderRegistryState::default(),
    );
    let fallback: std::sync::Arc<dyn StreamFn> =
        std::sync::Arc::new(MinimaxStream::from_environment(provider_registry.clone()));
    let websocket = std::sync::Arc::new(CodexWebSocketAdapter::production(Some(fallback.clone())));
    let provider = ProviderActor::new_with_websocket(fallback, Some(websocket));
    let deps = LoopDeps {
        state,
        steering,
        follow_up,
        tool_executor,
        provider,
        bus: bus.clone(),
        subscribers: runie_core::events::SubscriberRegistry::new(),
        hooks: ToolExecHooks {
            approval_mode: ApprovalMode::Ask,
            before_tool_call: Some({
                let broker = question_broker.clone();
                let approval_mode = approval_mode.clone();
                std::sync::Arc::new(move |input| {
                    let broker = broker.clone();
                    let approval_mode = approval_mode.clone();
                    Box::pin(async move {
                        if matches!(
                            runie_core::tools::approval_decision(
                                approval_mode.current(),
                                &input.tool_call.name,
                            ),
                            runie_core::tools::ApprovalDecision::Allow
                        ) {
                            return runie_core::types::BeforeToolCallResult::default();
                        }
                        if matches!(runie_core::tools::approval_decision(approval_mode.current(), &input.tool_call.name), runie_core::tools::ApprovalDecision::Deny { .. }) {
                            return runie_core::types::BeforeToolCallResult { block: true, reason: Some("Tool execution denied by approval mode".into()) };
                        }
                        let request = runie_core::tools::UserQuestionRequest {
                            question: format!("Allow tool '{}' to run?", input.tool_call.name),
                            options: vec![
                                runie_core::tools::UserQuestionOption {
                                    label: "Allow".into(),
                                    description: "Run this tool once.".into(),
                                },
                                runie_core::tools::UserQuestionOption {
                                    label: "Reject".into(),
                                    description: "Block this tool call.".into(),
                                },
                            ],
                            allow_multiple: false,
                        };
                        let Ok(answer) = broker.ask(request).await else {
                            return runie_core::types::BeforeToolCallResult {
                                block: true,
                                reason: Some("Approval UI was closed".into()),
                            };
                        };
                        let allowed = answer["answers"]
                            .as_array()
                            .and_then(|answers| answers.first())
                            .and_then(serde_json::Value::as_str)
                            == Some("Allow");
                        runie_core::types::BeforeToolCallResult {
                            block: !allowed,
                            reason: (!allowed).then(|| "Tool call rejected by user".into()),
                        }
                    })
                })
            }),
            ask_user_question: Some({
                let broker = question_broker.clone();
                std::sync::Arc::new(move |request| {
                    let broker = broker.clone();
                    Box::pin(async move { broker.ask(request).await })
                })
            }),
            subagent: Some({
                let loop_cell = subagent_loop.clone();
                std::sync::Arc::new(move |request| {
                    let loop_cell = loop_cell.clone();
                    Box::pin(async move {
                        let loop_actor = loop_cell
                            .get()
                            .ok_or_else(|| "subagent loop is not initialized".to_owned())?;
                        let prompt = format!("[subagent:{}]\n{}\n{}", serde_json::to_string(&request.role).unwrap_or_default(), request.task, request.context);
                        let messages = loop_actor
                            .prompt(
                                vec![runie_core::types::AgentMessage::User(runie_core::types::UserMessage {
                                    content: vec![runie_core::types::UserContent::Text { text: prompt }],
                                    timestamp: 0,
                                })],
                                AgentContext { system_prompt: request.role.clone().system_prompt().into(), ..AgentContext::default() },
                            )
                            .await
                            .map_err(|error| error.to_string())?;
                        Ok(serde_json::json!({"messages": messages}))
                    })
                })
            }),
            background_shell: Some({
                let actor = background_actor.clone();
                std::sync::Arc::new(move |request| {
                    let actor = actor.clone();
                    Box::pin(async move {
                        let id = actor.start(request.command).await?;
                        Ok(serde_json::json!({"job_id": id, "status": "running"}))
                    })
                })
            }),
            background_jobs: Some({
                let actor = background_actor.clone();
                std::sync::Arc::new(move || {
                    let actor = actor.clone();
                    Box::pin(async move {
                        serde_json::to_value(actor.snapshot()).map_err(|error| error.to_string())
                    })
                })
            }),
            background_cancel: Some({
                let actor = background_actor.clone();
                std::sync::Arc::new(move |request| {
                    let actor = actor.clone();
                    Box::pin(async move {
                        Ok(serde_json::json!({"job_id": request.job_id, "cancelled": actor.cancel(request.job_id).await}))
                    })
                })
            }),
            todo_write: Some({
                let actor = todo_actor.clone();
                std::sync::Arc::new(move |snapshot| {
                    let actor = actor.clone();
                    Box::pin(async move {
                        let snapshot = actor.replace(snapshot).await?;
                        serde_json::to_value(snapshot).map_err(|error| error.to_string())
                    })
                })
            }),
            web_search: std::env::var("RUNIE_WEB_SEARCH_ENDPOINT").ok().and_then(|endpoint| {
                WebSearchHttpClient::new(endpoint, std::env::var("RUNIE_WEB_SEARCH_TOKEN").ok(), std::time::Duration::from_secs(20)).ok().map(|client| client.hook())
            }),
            ..ToolExecHooks::default()
        },
        turn_hooks: runie_core::hooks::TurnHooks::default(),
        transform_context: None,
        api_key_resolver: None,
        context_recovery: None,
        convert_to_llm: None,
        stream_options: Default::default(),
        abort: None,
        tool_execution_mode: ToolExecutionMode::Parallel,
        steering_mode: QueueMode::OneAtATime,
        follow_up_mode: QueueMode::OneAtATime,
    };
    let loop_actor = LoopActor::new(deps);
    let _ = subagent_loop.set(loop_actor.clone());
    let mut app = App::new_with_broker_and_approval(loop_actor, bus, question_broker, approval_mode);
    app.attach_background_actor(background_actor);
    app.attach_todo_actor(todo_actor);
    app.provider_registry = provider_registry;
    app
}

#[allow(
    clippy::cognitive_complexity,
    clippy::too_many_lines,
    reason = "terminal event-loop ownership and redraw are intentionally co-located"
)]
async fn run_app(
    terminal: &mut Terminal<CrosstermBackend<Stdout>>,
    terminal_native: bool,
) -> Result<AppExit> {
    let app = build_live_app().await;
    app.load_provider_config().await;
    app.ui
        .send(runie_tui_model::UiMsg::SetSkillRows(
            discover_skill_names().await,
        ))
        .await;
    if terminal_native {
        app.set_theme(ThemeKind::TerminalNative).await;
    }
    let mut ui_commands = app.subscribe_ui_commands();
    let mut placeholder_hidden = false;
    let mut theme_preview = ThemePreview::default();
    let live_model = app
        .provider_registry
        .snapshot()
        .providers
        .iter()
        .find(|provider| provider.id == "minimax" && provider.connected)
        .and_then(|provider| provider.selected_model.clone())
        .unwrap_or_else(|| std::env::var("MINIMAX_MODEL").unwrap_or_else(|_| "MiniMax-M2.1".into()));
    app.prompt
        .set_model_caption(format!("{live_model} · always-approve"))
        .await;
    let color_level = runie_tui::terminal_color::ColorLevel::from_environment();

    // Paint a first frame synchronously before entering the event loop. This
    // guarantees the alternate screen is initialized even when the input
    // stream has not produced its first readiness notification yet.
    terminal.draw(|frame| {
        use ratatui::widgets::Widget;
        let frame_area = frame.area();
        let layout = runie_tui::layout::chat_layout(frame_area);
        let model = app.model_snapshot();
        let document = App::view_document_from_model(&model);
        let view = &document.root;
        let status = StatusBar::from_model_snapshot(document.props.status.clone());
        frame.buffer_mut().set_style(
            frame_area,
            runie_tui::appearance::background_style_for(status.theme()),
        );
        let dialog_visible = view.slots().any(|slot| {
            matches!(
                slot,
                runie_tui::view::Slot::ShortcutsOverlay
                    | runie_tui::view::Slot::CommandPaletteOverlay
                    | runie_tui::view::Slot::ModelSelectorOverlay
            )
        });
        if !dialog_visible && matches!(status.current(), Status::Ready) {
            render_live_ready_footer(layout.status, frame.buffer_mut(), status.theme());
        } else if !dialog_visible {
            status.render(layout.status, frame.buffer_mut());
        }
        let mut scrollback = Scrollback::from_model_snapshot(document.props.feed.clone());
        scrollback.set_live_grok_layout(true);
        scrollback.remove_kind(runie_tui::widgets::LineKind::SessionStart);
        scrollback.normalize_live_completed_assistants();
        scrollback.add_live_assistant_timestamp(layout.scrollback.width as usize);
        scrollback.render_with_terminal_height(
            layout.scrollback,
            frame_area.height,
            frame.buffer_mut(),
        );
        if view
            .slots()
            .any(|slot| slot == runie_tui::view::Slot::CompactModeHint)
            && runie_tui::layout::grok_small_screen_tip_visible(frame_area.height)
        {
            render_compact_hint(layout.prompt, frame.buffer_mut(), status.theme());
        }
        Widget::render(
            PromptWidget::from_model_snapshot(document.props.prompt.clone()),
            layout.prompt,
            frame.buffer_mut(),
        );
        // The stack is navigation state; this is the only dialog render
        // entry point, so no overlay can replace or overlap another one.
        match document.props.ui.dialog_stack.top_id() {
            Some("shortcuts") => render_shortcuts_dialog(frame.area(), frame.buffer_mut(), status.theme()),
            Some("commands") => render_command_palette(
                frame.area(),
                frame.buffer_mut(),
                &document.props.ui.command_palette_query,
                document.props.ui.command_palette_index,
                &document.props.ui.skill_rows,
                status.theme(),
            ),
            Some("palette-parameters") => render_palette_parameters(
                frame.area(),
                frame.buffer_mut(),
                &document.props.ui,
                status.theme(),
            ),
            Some("command-result") => render_command_result_dialog(
                frame.area(),
                frame.buffer_mut(),
                document.props.ui.command_result.as_deref().unwrap_or(""),
                status.theme(),
            ),
            Some("user-question") => render_user_question(frame.area(), frame.buffer_mut(), &document.props.ui, status.theme()),
            Some("model") => render_model_selector(
                frame.area(),
                frame.buffer_mut(),
                &document.props.ui,
                status.theme(),
            ),
            Some("session") => render_session_dialog(
                frame.area(),
                frame.buffer_mut(),
                &app.session_snapshot(),
                status.theme(),
            ),
            Some("changelog") => {
                render_changelog_dialog(frame.area(), frame.buffer_mut(), status.theme())
            }
            Some("files") => render_file_selector(
                frame.area(),
                frame.buffer_mut(),
                &document.props.prompt,
                status.theme(),
            ),
            None => {}
            Some(_) => {}
        }
        let header = &document.props.header;
        render_header(
            layout.header,
            frame.buffer_mut(),
            &header.meter,
            header.theme,
        );
        runie_tui::terminal_color::quantize_buffer(frame.buffer_mut(), color_level);
        frame.set_cursor_position(
            PromptWidget::from_model_snapshot(document.props.prompt.clone())
                .cursor_position(layout.prompt),
        );
    })?;

    let (mut renderer_handle, renderer_shutdown) = app.spawn_renderer();

    // OWNER: interactive input actor; the mailbox and worker live until the
    // terminal loop exits, and the owned task is dropped on shutdown.
    //
    // Input ingestion must never wait for rendering or actor work. The main
    // loop still dispatches events in order, but the terminal reader remains
    // free to receive every keystroke immediately.
    let (input_tx, mut input_rx) = mpsc::unbounded_channel::<InputEvent>();
    let (input_config_tx, mut input_config_rx) = mpsc::channel::<InputConfig>(4);
    let _input_owner = runie_core::spawn_owned_worker!(async move {
        let mut input = EventStream::new();
        let terminal_brand = std::env::var("TERM_PROGRAM")
            .or_else(|_| std::env::var("TERM"))
            .unwrap_or_else(|_| "unknown".into());
        let remuxed = ["TMUX", "STY", "ZELLIJ"]
            .into_iter()
            .any(|name| std::env::var_os(name).is_some());
        let scroll_speed = std::env::var("RUNIE_SCROLL_SPEED")
            .ok()
            .and_then(|value| value.parse::<u8>().ok())
            .unwrap_or(50);
        let inverted = std::env::var("RUNIE_INVERT_SCROLL")
            .ok()
            .is_some_and(|value| matches!(value.as_str(), "1" | "true" | "yes"));
        let scroll_mode = match std::env::var("RUNIE_SCROLL_MODE").ok().as_deref() {
            Some("wheel") => runie_tui_model::ScrollMode::Wheel,
            Some("trackpad") => runie_tui_model::ScrollMode::Trackpad,
            _ => runie_tui_model::ScrollMode::Auto,
        };
        let scroll_normalizer =
            runie_tui_model::ScrollNormalizer::for_terminal_context(&terminal_brand, remuxed)
                .with_speed(scroll_speed)
                .with_inversion(inverted)
                .with_mode(scroll_mode);
        const INPUT_SCROLL_VIEWPORT_ROWS: u16 = 24;
        let mut scroll_flush =
            runie_tui_model::ScrollFlushState::new(scroll_normalizer, INPUT_SCROLL_VIEWPORT_ROWS);
        let mut selection_origin = (0_u16, 0_u16);
        let scroll_epoch = Instant::now();
        let mut cadence = tokio::time::interval(Duration::from_millis(
            runie_tui_model::DEFAULT_SCROLL_FLUSH_CADENCE_MS,
        ));
        cadence.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);
        loop {
            tokio::select! {
                biased;
                // Branch priority: terminal events first, then config updates,
                // then the 16 ms scroll-flush cadence. Without `biased;`,
                // tokio picks ready branches pseudo-randomly, so a keystroke
                // queued behind a cadence tick could be delayed by one
                // 16 ms window of `input.next()` work — visible as input lag
                // during fast typing while the agent is streaming.
                result = input.next() => {
                    let Some(result) = result else { break };
                    let Ok(event) = result else { continue };
                    match event {
                        Event::Key(key) => {
                            if input_tx.send(InputEvent::Key(key)).is_err() { break; }
                        }
                        Event::Paste(text) => {
                            if input_tx.send(InputEvent::Paste(text)).is_err() { break; }
                        }
                        Event::Mouse(mouse) => {
                            if let Some(selection) = mouse_selection_input(
                                mouse.kind,
                                mouse.row,
                                mouse.column,
                                selection_origin,
                            ) {
                                if input_tx.send(selection).is_err() {
                                    break;
                                }
                                continue;
                            }
                            let direction = match mouse.kind {
                                MouseEventKind::ScrollUp => runie_tui_model::ScrollDirection::Up,
                                MouseEventKind::ScrollDown => runie_tui_model::ScrollDirection::Down,
                                _ => continue,
                            };
                            let at_ms = scroll_epoch.elapsed().as_millis() as u64;
                            let (next, _) = scroll_flush.input_at(at_ms, direction);
                            scroll_flush = next;
                        }
                        _ => {}
                    }
                }
                config = input_config_rx.recv() => {
                    let Some(config) = config else { break; };
                    match config {
                        InputConfig::ScrollViewport(rows) => {
                            scroll_flush = scroll_flush.with_viewport_rows(rows);
                        }
                        InputConfig::SelectionOrigin { row, column } => {
                            selection_origin = (row, column);
                        }
                    }
                }
                _ = cadence.tick() => {
                    let at_ms = scroll_epoch.elapsed().as_millis() as u64;
                    if scroll_flush.flush_due(at_ms) {
                        let (next, flush) = scroll_flush.flush_at(at_ms);
                        scroll_flush = next;
                        if flush.lines != 0 && input_tx.send(InputEvent::Mouse(flush.lines)).is_err() {
                            break;
                        }
                    }
                }
            }
        }
        let (_, _) = scroll_flush.finalize();
    });

    const RENDER_FRAME_INTERVAL: Duration = Duration::from_millis(16);
    let mut tick = tokio::time::interval(RENDER_FRAME_INTERVAL);

    // Routing for a single key event. The helper is invoked synchronously
    // from the input arm so every key crosses the prompt/UI actor mailbox
    // and the render frame observes the reduced state without waiting for
    // the next animation tick. The broadcast receiver is moved in and
    // returned back so the caller retains ownership across the await.
    #[allow(
        clippy::cognitive_complexity,
        clippy::too_many_lines,
        reason = "key routing table is intentionally co-located"
    )]
    async fn dispatch_key(
        app: &App,
        mut ui_commands: broadcast::Receiver<UiCommand>,
        renderer_shutdown: &watch::Sender<bool>,
        renderer_handle: &mut JoinHandle<()>,
        theme_preview: &mut ThemePreview,
        key: KeyEvent,
    ) -> Result<broadcast::Receiver<UiCommand>, AppExit> {
        let prompt_mode = app.model_snapshot().prompt.mode;
        if matches!(
            prompt_mode,
            runie_tui_model::InputMode::FileSearch | runie_tui_model::InputMode::FileViewer
        ) {
            let outcome = app.prompt.handle_key(key).await;
            let mode_after = app.model_snapshot().prompt.mode;
            if matches!(outcome, PromptOutcome::Edited | PromptOutcome::Ignored)
                && mode_after == runie_tui_model::InputMode::Normal
            {
                app.ui.send(runie_tui_model::UiMsg::DialogEscape).await;
            }
            return Ok(ui_commands);
        }
        let ui = app.ui.snapshot();
        let repeated_palette_trigger = ui.command_palette_open
            && ((key.code == KeyCode::Char('p')
                && key.modifiers.contains(KeyModifiers::CONTROL))
                || (key.code == KeyCode::Char('/') && key.modifiers == KeyModifiers::NONE));
        if repeated_palette_trigger {
            if let Some(theme) = theme_preview.cancel() {
                app.set_theme(theme).await;
            }
            app.ui.send(runie_tui_model::UiMsg::CloseDialogs).await;
            return Ok(ui_commands);
        }
        if app.ui.snapshot().dialog_stack.top_id() == Some("user-question") {
            let question = app.ui.snapshot().user_question.clone();
            match key.code {
                KeyCode::Up => app.ui.send(runie_tui_model::UiMsg::UserQuestionMove(-1)).await,
                KeyCode::Down => app.ui.send(runie_tui_model::UiMsg::UserQuestionMove(1)).await,
                KeyCode::Char(' ') => app.ui.send(runie_tui_model::UiMsg::ToggleUserQuestionSelection).await,
                KeyCode::Enter => {
                    app.ui.send(runie_tui_model::UiMsg::SubmitUserQuestion).await;
                    if let Ok(UiCommand::AnswerUserQuestion { id, answer }) = ui_commands.recv().await {
                        let _ = app.question_broker.answer(&id, answer);
                        persist_question_trace(app, &id).await;
                    }
                    app.ui.send(runie_tui_model::UiMsg::CloseDialogs).await;
                }
                KeyCode::Esc => {
                    if let Some(question) = question {
                        let _ = app.question_broker.cancel(&question.id);
                        persist_question_trace(app, &question.id).await;
                    }
                    app.ui.send(runie_tui_model::UiMsg::CloseDialogs).await;
                }
                _ => {}
            }
            return Ok(ui_commands);
        }
        if key.code == KeyCode::Esc && !app.ui.snapshot().dialog_stack.is_empty() {
            if let Some(theme) = theme_preview.cancel() {
                app.set_theme(theme).await;
            }
            app.ui.send(runie_tui_model::UiMsg::DialogEscape).await;
            return Ok(ui_commands);
        }
        if app.ui.snapshot().palette_parameter_action.is_some() {
            match key.code {
                KeyCode::Backspace => app.ui.send(runie_tui_model::UiMsg::PaletteParameterBackspace).await,
                KeyCode::Up => app.ui.send(runie_tui_model::UiMsg::PaletteParameterMove(-1)).await,
                KeyCode::Down => app.ui.send(runie_tui_model::UiMsg::PaletteParameterMove(1)).await,
                KeyCode::Char(' ') if app.ui.snapshot().palette_parameter_action == Some(PaletteAction::SelectTheme) => {
                    theme_preview.begin(app.status_snapshot().theme());
                    app.ui.send(runie_tui_model::UiMsg::PaletteParameterPreview).await;
                    if let Ok(UiCommand::SelectTheme(value)) = ui_commands.try_recv() {
                        if let Some(theme) = parse_theme_name(&value) {
                            app.set_theme(theme).await;
                        }
                    }
                }
                KeyCode::Enter => {
                    theme_preview.commit();
                    app.ui.send(runie_tui_model::UiMsg::PaletteParameterSubmit).await;
                    match ui_commands.try_recv() {
                        Ok(UiCommand::ExecuteMappable(command)) => {
                            let _ = app.route_mappable_command(command).await;
                        }
                        Ok(UiCommand::SelectTheme(value)) => {
                            if let Some(theme) = parse_theme_name(&value) {
                                app.set_theme(theme).await;
                            }
                        }
                        Ok(UiCommand::ProviderAction {
                            action: PaletteAction::ManageProviders,
                            value,
                        }) =>
                        {
                            let parts = value.split('|').map(str::trim).collect::<Vec<_>>();
                            match parts.first().copied() {
                                Some("connect") if parts.len() >= 5 => {
                                    use runie_core::provider_registry::{ProviderConfig, ProviderEvent};
                                    let _ = app.apply_provider_event(ProviderEvent::Connected(
                                        ProviderConfig {
                                            id: parts[1].to_owned(),
                                            label: parts[1].to_owned(),
                                            base_url: parts[2].to_owned(),
                                            api_key: Some(parts[3].to_owned()),
                                            selected_model: Some(parts[4].to_owned()),
                                            connected: true,
                                        },
                                    )).await;
                                }
                                Some("disconnect") if parts.len() >= 2 => {
                                    use runie_core::provider_registry::ProviderEvent;
                                    let _ = app.apply_provider_event(ProviderEvent::Disconnected {
                                        provider_id: parts[1].to_owned(),
                                    }).await;
                                }
                                Some("update") if parts.len() >= 4 => {
                                    use runie_core::provider_registry::{ProviderConfig, ProviderEvent};
                                    let existing = app.provider_registry.snapshot().providers
                                        .into_iter()
                                        .find(|provider| provider.id == parts[1]);
                                    let _ = app.apply_provider_event(ProviderEvent::Updated(
                                        ProviderConfig {
                                            id: parts[1].to_owned(),
                                            label: existing.as_ref().map(|p| p.label.clone()).unwrap_or_else(|| parts[1].to_owned()),
                                            base_url: parts[2].to_owned(),
                                            api_key: Some(parts[3].to_owned()),
                                            selected_model: parts.get(4).map(|v| (*v).to_owned()).or_else(|| existing.as_ref().and_then(|p| p.selected_model.clone())),
                                            connected: existing.as_ref().map(|p| p.connected).unwrap_or(true),
                                        },
                                    )).await;
                                }
                                Some("model") if parts.len() >= 3 => {
                                    let _ = app.select_provider_model(parts[1].to_owned(), parts[2].to_owned()).await;
                                }
                                Some("list") | None => app.show_provider_summary().await,
                                _ => {}
                            }
                        }
                        _ => {}
                    }
                }
                KeyCode::Char(ch)
                    if !key.modifiers.intersects(KeyModifiers::CONTROL | KeyModifiers::ALT) =>
                {
                    app.ui.send(runie_tui_model::UiMsg::PaletteParameterChar(ch)).await;
                }
                _ => {}
            }
            return Ok(ui_commands);
        }
        if app.ui.snapshot().model_selector_open {
            match key.code {
                KeyCode::Esc => {
                    app.model_selector_key(runie_tui::app::UiMsg::ModelSelectorEscape)
                        .await;
                }
                KeyCode::Backspace => {
                    app.model_selector_key(runie_tui::app::UiMsg::ModelSelectorBackspace)
                        .await;
                }
                KeyCode::Up => {
                    app.model_selector_key(runie_tui::app::UiMsg::ModelSelectorMove(-1))
                        .await;
                }
                KeyCode::Down => {
                    app.model_selector_key(runie_tui::app::UiMsg::ModelSelectorMove(1))
                        .await;
                }
                KeyCode::Tab => {
                    app.model_selector_key(runie_tui::app::UiMsg::ModelSelectorToggleScope)
                        .await;
                }
                KeyCode::Enter => {
                    let _ = app.activate_model_selector().await;
                    // Enter is a completed selector action from the user's
                    // perspective; do not leave the overlay stranded if the
                    // provider-side selection cannot be acknowledged.
                    if app.ui.snapshot().model_selector_open {
                        app.ui
                            .send(runie_tui::app::UiMsg::ActivateModelSelector)
                            .await;
                    }
                }
                KeyCode::Char(ch)
                    if !key
                        .modifiers
                        .intersects(KeyModifiers::CONTROL | KeyModifiers::ALT) =>
                {
                    app.model_selector_key(runie_tui::app::UiMsg::ModelSelectorChar(ch))
                        .await;
                }
                _ => {}
            }
            return Ok(ui_commands);
        }
        if app.ui.snapshot().command_palette_open && key.code == KeyCode::Esc {
            app.command_palette_key(runie_tui::app::UiMsg::CommandPaletteEscape)
                .await;
            return Ok(ui_commands);
        }
        if app.ui.snapshot().command_palette_open {
            match key.code {
                KeyCode::Char(ch)
                    if !key
                        .modifiers
                        .intersects(KeyModifiers::CONTROL | KeyModifiers::ALT) =>
                {
                    app.command_palette_key(runie_tui::app::UiMsg::CommandPaletteChar(ch))
                        .await;
                }
                KeyCode::Backspace | KeyCode::Char('\u{8}' | '\u{7f}') => {
                    app.command_palette_key(runie_tui::app::UiMsg::CommandPaletteBackspace)
                        .await;
                }
                KeyCode::Up => {
                    app.command_palette_key(runie_tui::app::UiMsg::CommandPaletteMove(-1))
                        .await;
                }
                KeyCode::Down => {
                    app.command_palette_key(runie_tui::app::UiMsg::CommandPaletteMove(1))
                        .await;
                }
                KeyCode::Enter => {
                    let _ = app.activate_command_palette().await;
                    match ui_commands.recv().await {
                        Ok(UiCommand::ActivatePaletteEntry(PaletteAction::NewSession)) => {
                            let _ = app.reset_session().await;
                        }
                        Ok(UiCommand::ActivatePaletteEntry(PaletteAction::KeyboardShortcuts)) => {
                            app.toggle_shortcuts().await;
                        }
                        Ok(UiCommand::ActivatePaletteEntry(PaletteAction::Quit)) => {
                            let _ = renderer_shutdown.send(true);
                            let _ = renderer_handle.await;
                            return Err(AppExit::Quit);
                        }
                        Ok(UiCommand::ActivatePaletteEntry(action)) => {
                            if action == PaletteAction::SelectModel {
                                app.toggle_model_selector().await;
                                return Ok(ui_commands);
                            }
                            if action == PaletteAction::ManageProviders {
                                app.show_provider_summary().await;
                                return Ok(ui_commands);
                            }
                            let command = match action {
                                PaletteAction::Changelog => Some(runie_core::commands::MappableBuiltinCommand::Changelog),
                                PaletteAction::CopyLastResponse => Some(runie_core::commands::MappableBuiltinCommand::Copy),
                                PaletteAction::SessionInfo => Some(runie_core::commands::MappableBuiltinCommand::SessionInfo),
                                PaletteAction::ScopedModels => Some(runie_core::commands::MappableBuiltinCommand::ScopedModels),
                                _ => None,
                            };
                            if let Some(command) = command {
                                let _ = app.route_mappable_command(command).await;
                            } else if let Some(name) = extended_palette_command_name(&action) {
                                let _ = app
                                    .route_mappable_command(
                                        runie_core::commands::MappableBuiltinCommand::Extended {
                                            name: name.to_owned(),
                                            args: String::new(),
                                        },
                                    )
                                    .await;
                            }
                        }
                        Ok(UiCommand::OpenPaletteParameters(action)) => {
                            app.ui.send(runie_tui::app::UiMsg::OpenPaletteParameters(action)).await;
                        }
                        Ok(UiCommand::ExecuteMappable(command)) => {
                            let _ = app.route_mappable_command(command).await;
                        }
                        Ok(UiCommand::SelectTheme(value)) => {
                            if let Some(theme) = parse_theme_name(&value) {
                                app.set_theme(theme).await;
                            }
                        }
                        Ok(UiCommand::ProviderAction {
                            action: PaletteAction::ManageProviders,
                            value,
                        }) => {
                            use runie_core::provider_registry::{ProviderConfig, ProviderEvent};
                            let parts = value.split('|').map(str::trim).collect::<Vec<_>>();
                            match parts.first().copied() {
                                Some("connect") if parts.len() >= 5 => {
                                    let _ = app.apply_provider_event(ProviderEvent::Connected(
                                        ProviderConfig {
                                            id: parts[1].to_owned(),
                                            label: parts[1].to_owned(),
                                            base_url: parts[2].to_owned(),
                                            api_key: Some(parts[3].to_owned()),
                                            selected_model: Some(parts[4].to_owned()),
                                            connected: true,
                                        },
                                    )).await;
                                }
                                Some("disconnect") if parts.len() >= 2 => {
                                    let _ = app.apply_provider_event(ProviderEvent::Disconnected {
                                        provider_id: parts[1].to_owned(),
                                    }).await;
                                }
                                Some("update") if parts.len() >= 4 => {
                                    let existing = app.provider_registry.snapshot().providers
                                        .into_iter()
                                        .find(|provider| provider.id == parts[1]);
                                    let label = existing.as_ref().map(|p| p.label.clone()).unwrap_or_else(|| parts[1].to_owned());
                                    let selected_model = existing.as_ref().and_then(|p| p.selected_model.clone());
                                    let connected = existing.as_ref().map(|p| p.connected).unwrap_or(true);
                                    let _ = app.apply_provider_event(ProviderEvent::Updated(
                                        ProviderConfig {
                                            id: parts[1].to_owned(),
                                            label,
                                            base_url: parts[2].to_owned(),
                                            api_key: Some(parts[3].to_owned()),
                                            selected_model: parts.get(4).map(|v| (*v).to_owned()).or(selected_model),
                                            connected,
                                        },
                                    )).await;
                                }
                                Some("model") if parts.len() >= 3 => {
                                    if let Err(error) = app.select_provider_model(parts[1].to_owned(), parts[2].to_owned()).await {
                                        app.prompt.set_model_caption(format!("provider error: {error}")).await;
                                    }
                                }
                                Some("list") | None => app.show_provider_summary().await,
                                _ => {}
                            }
                        }
                        Ok(UiCommand::ActivateSkill(skill)) => {
                            let _ = app
                                .handle_prompt_outcome(PromptOutcome::Submitted(format!(
                                    "/skills:{skill}"
                                )))
                                .await;
                        }
                        _ => {}
                    }
                }
                _ => {}
            }
            return Ok(ui_commands);
        }
        let status = app.status_snapshot().current().clone();
        let streaming = matches!(status, Status::Thinking | Status::Streaming);
        let prompt_model = app.model_snapshot().prompt;
        match map_key(key, !prompt_model.is_empty(), streaming) {
            Action::ClearPrompt => {
                app.prompt.clear().await;
                return Ok(ui_commands);
            }
            Action::Abort => {
                app.loop_actor.abort().await;
                return Ok(ui_commands);
            }
            Action::ModeCycle => {
                app.prompt.cycle_mode().await;
                return Ok(ui_commands);
            }
            Action::OpenShortcuts => {
                app.toggle_shortcuts().await;
                return Ok(ui_commands);
            }
            Action::OpenCommandPalette => {
                app.toggle_command_palette().await;
                return Ok(ui_commands);
            }
            Action::OpenModelSelector => {
                app.toggle_model_selector().await;
                return Ok(ui_commands);
            }
            Action::OpenFileSearch => {
                app.prompt.open_file_search().await;
                app.ui.send(runie_tui_model::UiMsg::OpenFileDialog).await;
                return Ok(ui_commands);
            }
            Action::ToggleFold => {
                app.toggle_selected_tool_fold().await;
                return Ok(ui_commands);
            }
            Action::SelectNextTool => {
                app.select_next_tool().await;
                return Ok(ui_commands);
            }
            Action::SelectPreviousTool => {
                app.select_previous_tool().await;
                return Ok(ui_commands);
            }
            Action::SelectNextEntry => {
                app.select_next_entry().await;
                return Ok(ui_commands);
            }
            Action::SelectPreviousEntry => {
                app.select_previous_entry().await;
                return Ok(ui_commands);
            }
            Action::ExtendSelectionNext => {
                app.extend_selection(1).await;
                return Ok(ui_commands);
            }
            Action::ExtendSelectionPrevious => {
                app.extend_selection(-1).await;
                return Ok(ui_commands);
            }
            Action::ScrollUp => {
                app.scroll_scrollback_by(-1).await;
                return Ok(ui_commands);
            }
            Action::ScrollDown => {
                app.scroll_scrollback_by(1).await;
                return Ok(ui_commands);
            }
            Action::Quit => {
                let _ = renderer_shutdown.send(true);
                let _ = renderer_handle.await;
                return Err(AppExit::Quit);
            }
            _ => {}
        }
        if key.modifiers.is_empty() {
            match key.code {
                KeyCode::Char(_) => {
                    app.prompt.handle_key(key).await;
                }
                KeyCode::Backspace => {
                    app.prompt.handle_key(key).await;
                }
                KeyCode::Enter => {
                    let outcome = app.prompt.handle_key(key).await;
                    if let PromptOutcome::Submitted(text) = outcome {
                        let disposition = classify_builtin_command(&text);
                        if !matches!(disposition, BuiltinCommandDisposition::NotBuiltin) {
                            let copy_command = matches!(
                                disposition,
                                BuiltinCommandDisposition::Mappable(
                                    runie_core::commands::MappableBuiltinCommand::Copy
                                )
                            );
                            if matches!(
                                disposition,
                                BuiltinCommandDisposition::Mappable(
                                    runie_core::commands::MappableBuiltinCommand::Quit
                                )
                            ) {
                                return Err(AppExit::Quit);
                            }
                            let _ = app.route_builtin_command(disposition).await;
                            if copy_command {
                                if let Ok(UiCommand::CopyText(text)) = ui_commands.recv().await {
                                    let _ = execute!(
                                        io::stdout(),
                                        Print(osc52_clipboard_sequence(&text))
                                    );
                                }
                            }
                            return Ok(ui_commands);
                        }
                        if is_quit_command(&text) {
                            return Err(AppExit::Quit);
                        }
                        let _ = app
                            .handle_prompt_outcome(PromptOutcome::Submitted(text))
                            .await;
                    }
                }
                _ => {}
            }
        } else if key.modifiers.contains(KeyModifiers::SHIFT)
            && matches!(key.code, KeyCode::Char(_))
        {
            // Crossterm reports an uppercase character
            // as Shift+Char. It is still prompt input;
            // dropping it makes `Hey` become `y`.
            app.prompt.handle_key(key).await;
        } else if matches!(key.code, KeyCode::Enter) {
            app.prompt.handle_key(key).await;
        }
        Ok(ui_commands)
}

async fn persist_question_trace(app: &App, id: &str) {
    if let Some(trace) = app
        .question_broker
        .traces()
        .into_iter()
        .rev()
        .find(|trace| trace.id == id)
    {
        let _ = app.session_actor.record_user_question_trace(trace).await;
    }
}

    // One terminal frame. Called from the input arm after a key is
    // dispatched and from the tick arm for animation/agent activity
    // refreshes, so a fast key burst is visible key-by-key instead of
    // after a tick-batched draw.
    #[allow(
        clippy::cognitive_complexity,
        clippy::too_many_lines,
        reason = "terminal draw is intentionally co-located with state update"
    )]
    async fn render_frame(
        app: &App,
        terminal: &mut Terminal<CrosstermBackend<Stdout>>,
        input_config_tx: &mpsc::Sender<InputConfig>,
        color_level: runie_tui::terminal_color::ColorLevel,
        placeholder_hidden: &mut bool,
    ) -> Result<(), String> {
        if let Some(question) = app.question_broker.try_next() {
            app.ui.send(runie_tui_model::UiMsg::OpenUserQuestion(question)).await;
        }
        let model = app.model_snapshot();
        let settled = matches!(model.status.state, Status::Ready) && !model.feed.is_empty();
        if settled != *placeholder_hidden {
            app.prompt.set_placeholder_visible(!settled).await;
            *placeholder_hidden = settled;
        }
        if let Err(e) = terminal.draw(|f| {
            use ratatui::layout::Rect;
            use ratatui::widgets::Widget;
            use runie_tui::layout::chat_layout;
            let layout = chat_layout(f.area());
            let _ = input_config_tx.try_send(InputConfig::ScrollViewport(layout.scrollback.height));
            let _ = input_config_tx.try_send(InputConfig::SelectionOrigin {
                row: layout.scrollback.y,
                column: layout.scrollback.x,
            });
            // Wrap each render in catch_unwind so a widget bug
            // doesn't kill the binary — log + continue.
            let res = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                let frame_area = f.area();
                let buf = f.buffer_mut();
                let document = App::view_document_from_model(&model);
                let view = &document.root;
                let status = StatusBar::from_model_snapshot(document.props.status.clone());
                buf.set_style(
                    frame_area,
                    runie_tui::appearance::background_style_for(status.theme()),
                );
                let turn_status = status.turn_status();
                let dialog_visible = view.slots().any(|slot| {
                    matches!(
                        slot,
                        runie_tui::view::Slot::ShortcutsOverlay
                            | runie_tui::view::Slot::CommandPaletteOverlay
                            | runie_tui::view::Slot::ModelSelectorOverlay
                    )
                });
                if !dialog_visible && matches!(status.current(), Status::Ready) {
                    render_live_ready_footer(layout.status, buf, status.theme());
                } else if !dialog_visible {
                    status.render(layout.status, buf);
                }
                let mut scrollback = Scrollback::from_model_snapshot(document.props.feed.clone());
                scrollback.set_live_grok_layout(true);
                scrollback.remove_kind(runie_tui::widgets::LineKind::SessionStart);
                scrollback.normalize_live_completed_assistants();
                scrollback.add_live_assistant_timestamp(layout.scrollback.width as usize);
                scrollback.render_with_terminal_height(layout.scrollback, frame_area.height, buf);
                if view
                    .slots()
                    .any(|slot| slot == runie_tui::view::Slot::CompactModeHint)
                    && runie_tui::layout::grok_small_screen_tip_visible(frame_area.height)
                {
                    render_compact_hint(layout.prompt, buf, status.theme());
                }
                if let Some(turn_status) = turn_status {
                    turn_status.render(
                        ratatui::layout::Rect {
                            x: layout.scrollback.x,
                            y: layout.prompt.y.saturating_sub(2),
                            width: layout.scrollback.width,
                            height: 1,
                        },
                        buf,
                    );
                }
                if document.props.prompt.mode == runie_tui_model::InputMode::FileSearch {
                    render_file_selector(frame_area, buf, &document.props.prompt, status.theme());
                }
                let prompt = PromptWidget::from_model_snapshot(document.props.prompt.clone());
                let cursor_position = prompt.cursor_position(layout.prompt);
                Widget::render(prompt, layout.prompt, buf);
                // Dialog navigation is stacked state, but rendering is one
                // container: only the top frame is ever painted.
                match document.props.ui.dialog_stack.top_id() {
                    Some("shortcuts") => render_shortcuts_dialog(frame_area, buf, status.theme()),
                    Some("commands") => render_command_palette(
                        frame_area,
                        buf,
                        &document.props.ui.command_palette_query,
                        document.props.ui.command_palette_index,
                        &document.props.ui.skill_rows,
                        status.theme(),
                    ),
                    Some("palette-parameters") => {
                        render_palette_parameters(frame_area, buf, &document.props.ui, status.theme())
                    }
                    Some("command-result") => render_command_result_dialog(
                        frame_area,
                        buf,
                        document.props.ui.command_result.as_deref().unwrap_or(""),
                        status.theme(),
                    ),
                    Some("user-question") => render_user_question(frame_area, buf, &document.props.ui, status.theme()),
                    Some("model") => render_model_selector(frame_area, buf, &document.props.ui, status.theme()),
                    Some("session") => {
                        let session = app.session_snapshot();
                        render_session_dialog(frame_area, buf, &session, status.theme());
                    }
                    Some("changelog") => render_changelog_dialog(frame_area, buf, status.theme()),
                    Some("files") => {
                        render_file_selector(frame_area, buf, &document.props.prompt, status.theme())
                    }
                    None => {}
                    Some(_) => {}
                }
                let header = &document.props.header;
                render_header(layout.header, buf, &header.meter, header.theme);
                runie_tui::terminal_color::quantize_buffer(buf, color_level);
                f.set_cursor_position(cursor_position);
                let _ = Rect::default();
            }));
            if let Err(e) = res {
                eprintln!("render panic: {:?}", e);
                std::panic::resume_unwind(e);
            }
        }) {
            return Err(format!("draw: {e}"));
        }
        Ok(())
    }

    loop {
        tokio::select! {
            biased;
            input = input_rx.recv() => {
                let Some(input) = input else { return Ok(AppExit::Quit) };
                match input {
                    InputEvent::Key(key) => {
                        if key.kind == KeyEventKind::Press {
                            ui_commands = match dispatch_key(
                                &app,
                                ui_commands,
                                &renderer_shutdown,
                                &mut renderer_handle,
                                &mut theme_preview,
                                key,
                            )
                            .await
                            {
                                Ok(rx) => rx,
                                Err(exit) => return Ok(exit),
                            };
                            if let Err(err) = render_frame(
                                &app,
                                terminal,
                                &input_config_tx,
                                color_level,
                                &mut placeholder_hidden,
                            )
                            .await
                            {
                                return Ok(AppExit::Error(err));
                            }
                        }
                    }
                    InputEvent::Paste(text) => {
                        let disposition = classify_builtin_command(&text);
                        if !text.starts_with('/') || matches!(disposition, BuiltinCommandDisposition::NotBuiltin) {
                            for ch in text.chars() {
                                app.prompt.handle_key(KeyEvent::new(
                                    KeyCode::Char(ch),
                                    KeyModifiers::NONE,
                                )).await;
                            }
                        } else if matches!(disposition, BuiltinCommandDisposition::Mappable(runie_core::commands::MappableBuiltinCommand::Quit)) {
                            let _ = renderer_shutdown.send(true);
                            let _ = renderer_handle.await;
                            return Ok(AppExit::Quit);
                        } else {
                            let _ = app.route_builtin_command(disposition).await;
                        }
                        if let Err(err) = render_frame(
                            &app,
                            terminal,
                            &input_config_tx,
                            color_level,
                            &mut placeholder_hidden,
                        ).await {
                            return Ok(AppExit::Error(err));
                        }
                    }
                    InputEvent::Mouse(delta) => app.scroll_scrollback_by(delta).await,
                    InputEvent::MouseSelectionStart(row, column) => {
                        app.mouse_selection_start(runie_tui::widgets::CellPosition { row, column }).await;
                    }
                    InputEvent::MouseSelectionExtend(row, column) => {
                        app.mouse_selection_extend(runie_tui::widgets::CellPosition { row, column }).await;
                    }
                    InputEvent::MouseSelectionCommit => {
                        app.mouse_selection_commit().await;
                        if let Some(text) = app.copy_selection_text().await {
                            let _ = execute!(
                                io::stdout(),
                                Print(osc52_clipboard_sequence(&text))
                            );
                        }
                    }
                }
            }
            _ = tick.tick() => {
                // Animation and agent activity refreshes run on the tick
                // cadence; key events render in the input arm so a fast
                // burst is visible key-by-key instead of waiting for the
                // tick to drain a queued batch.
                if let Err(err) =
                    render_frame(
                        &app,
                        terminal,
                        &input_config_tx,
                        color_level,
                        &mut placeholder_hidden,
                    )
                    .await
                {
                    return Ok(AppExit::Error(err));
                }
            }
        }
    }
}

fn extended_palette_command_name(action: &PaletteAction) -> Option<&'static str> {
    Some(match action {
        PaletteAction::Help => "help",
        PaletteAction::ContextInfo => "context",
        PaletteAction::Doctor => "doctor",
        PaletteAction::Recap => "recap",
        PaletteAction::ViewPlan => "view-plan",
        PaletteAction::ShareSession => "share",
        PaletteAction::Reload => "reload",
        PaletteAction::TrustProject => "trust",
        PaletteAction::Skills => "skills",
        PaletteAction::Hooks => "hooks",
        PaletteAction::Plugins => "plugins",
        PaletteAction::Mcps => "mcps",
        PaletteAction::Questions => "questions",
        PaletteAction::Memory => "memory",
        PaletteAction::Workflows => "workflows",
        _ => return None,
    })
}

#[allow(dead_code)]
fn _key_marker(_k: KeyEvent) {}

#[cfg(test)]
mod tests {
    use super::{
        current_branch, mouse_selection_input, render_header, render_live_ready_footer,
        repository_label, InputEvent, ThemePreview,
    };
    use crossterm::event::{MouseButton, MouseEventKind};
    use ratatui::buffer::Buffer;
    use ratatui::layout::Rect;
    use ratatui::style::Modifier;

    #[test]
    fn theme_preview_restores_origin_on_cancel_and_forgets_it_on_commit() {
        use runie_core::types::ThemeKind;

        let mut preview = ThemePreview::default();
        preview.begin(ThemeKind::GrokNight);
        preview.begin(ThemeKind::GrokDay);
        assert_eq!(preview.cancel(), Some(ThemeKind::GrokNight));
        assert_eq!(preview.cancel(), None);

        preview.begin(ThemeKind::GrokDay);
        preview.commit();
        assert_eq!(preview.cancel(), None);
    }

    #[test]
    fn minimax_usage_maps_openai_compatible_token_fields() {
        let payload = serde_json::json!({
            "usage": {
                "prompt_tokens": 12,
                "completion_tokens": 8,
                "total_tokens": 20
            }
        });
        let usage = super::minimax_usage(&payload);
        assert_eq!(usage.input, 12);
        assert_eq!(usage.output, 8);
        assert_eq!(usage.total_tokens, 20);
    }

    #[test]
    fn minimax_status_code_accepts_numeric_and_string_provider_codes() {
        const MINIMAX_AUTH_FAILURE_CODE: i64 = 1004;
        assert_eq!(
            super::minimax_status_code(&serde_json::json!({
                "base_resp": {"status_code": MINIMAX_AUTH_FAILURE_CODE}
            })),
            Some(MINIMAX_AUTH_FAILURE_CODE)
        );
        assert_eq!(
            super::minimax_status_code(&serde_json::json!({
                "base_resp": {"status_code": "0"}
            })),
            Some(0)
        );
    }

    #[test]
    fn minimax_stream_ignores_final_non_delta_message_payloads() {
        let final_message = serde_json::json!({
            "choices": [{"message": {"content": "already emitted"}}]
        });
        assert_eq!(super::minimax_stream_delta(&final_message, "content"), None);
    }

    #[test]
    fn minimax_messages_preserves_the_context_system_prompt() {
        let context = runie_core::types::AgentContext {
            system_prompt: "Be concise".into(),
            messages: Vec::new(),
            tools: None,
        };
        let messages = super::minimax_messages(&context).expect("messages serialize");
        assert_eq!(messages[0]["role"], "system");
        assert_eq!(messages[0]["content"], "Be concise");
    }

    #[test]
    fn minimax_events_translates_complete_tool_calls() {
        let events = super::minimax_events(&serde_json::json!({
            "choices": [{
                "message": {
                    "tool_calls": [{
                        "id": "call-1",
                        "function": {
                            "name": "read_file",
                            "arguments": "{\"path\":\"README.md\"}"
                        }
                    }]
                }
            }]
        }));
        assert!(matches!(
            events[1],
            runie_core::types::AssistantMessageEvent::ToolCallStart { index: 0, .. }
        ));
        let runie_core::types::AssistantMessageEvent::ToolCallEnd { tool_call, .. } = &events[2]
        else {
            panic!("expected tool call end");
        };
        assert_eq!(tool_call.name, "read_file");
        assert_eq!(tool_call.arguments["path"], "README.md");
        assert!(matches!(
            events.last(),
            Some(runie_core::types::AssistantMessageEvent::Done {
                stop_reason: runie_core::types::StopReason::ToolUse,
                ..
            })
        ));
    }

    #[test]
    fn minimax_normalizes_tool_result_role_and_call_id() {
        let normalized = super::normalize_minimax_message(serde_json::json!({
            "role": "toolResult",
            "toolCallId": "call-1",
            "content": "ok"
        }));
        assert_eq!(normalized["role"], "tool");
        assert_eq!(normalized["tool_call_id"], "call-1");
        assert!(normalized.get("toolCallId").is_none());
    }

    #[test]
    fn minimax_normalizes_assistant_tool_call_blocks() {
        let normalized = super::normalize_minimax_message(serde_json::json!({
            "role": "assistant",
            "content": [{
                "type": "toolCall",
                "id": "call-1",
                "name": "echo",
                "arguments": {"text": "ok"}
            }]
        }));
        assert_eq!(normalized["tool_calls"][0]["id"], "call-1");
        assert_eq!(normalized["tool_calls"][0]["function"]["name"], "echo");
        assert_eq!(normalized["content"], serde_json::Value::Null);
    }

    #[test]
    fn osc52_clipboard_sequence_base64_encodes_payload() {
        assert_eq!(
            super::osc52_clipboard_sequence("latest answer"),
            "\x1b]52;c;bGF0ZXN0IGFuc3dlcg==\x07"
        );
    }

    #[test]
    fn header_uses_cached_repository_branch() {
        let area = Rect::new(0, 0, 80, 1);
        let mut buffer = Buffer::empty(area);
        render_header(
            area,
            &mut buffer,
            "0 / 500K",
            runie_core::types::ThemeKind::GrokNight,
        );
        let row = (0..area.width)
            .filter_map(|x| buffer.cell((x, 0)))
            .map(|cell| cell.symbol())
            .collect::<String>();
        assert!(row.contains(current_branch()));
        assert!(row.contains(&repository_label()));
    }

    #[test]
    fn live_footer_advances_by_terminal_cells_for_unicode_separators() {
        let area = Rect::new(0, 0, 40, 1);
        let mut buffer = Buffer::empty(area);
        render_live_ready_footer(area, &mut buffer, runie_core::types::ThemeKind::GrokNight);

        // "Shift+Tab" is nine cells and ":mode  │  " is ten cells, so the
        // second shortcut starts at cell nineteen, not at the UTF-8 byte
        // offset produced by `str::len()`.
        let shortcut = buffer.cell((19, 0)).expect("second shortcut");
        assert_eq!(shortcut.symbol(), "C");
        assert!(shortcut.modifier.contains(Modifier::BOLD));
    }

    #[test]
    fn mouse_selection_input_is_pure_and_origin_relative() {
        assert!(matches!(
            mouse_selection_input(MouseEventKind::Down(MouseButton::Left), 12, 18, (5, 7)),
            Some(InputEvent::MouseSelectionStart(7, 11))
        ));
        assert!(matches!(
            mouse_selection_input(MouseEventKind::Drag(MouseButton::Left), 8, 10, (5, 7)),
            Some(InputEvent::MouseSelectionExtend(3, 3))
        ));
        assert!(matches!(
            mouse_selection_input(MouseEventKind::Up(MouseButton::Left), 8, 10, (5, 7)),
            Some(InputEvent::MouseSelectionCommit)
        ));
        assert!(mouse_selection_input(MouseEventKind::Moved, 8, 10, (5, 7)).is_none());
    }

    /// Pin down the contract that the live main loop's `dispatch_key`
    /// helper relies on: every Press key in a burst crosses the
    /// `PromptActor` mailbox in order and updates the prompt snapshot
    /// synchronously, so a render after each key is sufficient to make
    /// the typed text visible. A regression that batches keys through a
    /// single awaited reducer would surface here as a stale snapshot
    /// after `N` awaits.
    #[tokio::test]
    #[allow(
        clippy::too_many_lines,
        reason = "self-contained App wiring mirrors the runie.rs bin for fidelity"
    )]
    async fn prompt_actor_reduces_each_press_key_independently() {
        use crossterm::event::{KeyCode, KeyEvent, KeyEventKind, KeyModifiers};
        let app = prompt_test_app().await;

        let mut expected = String::new();
        for ch in "hello".chars() {
            let key = KeyEvent {
                code: KeyCode::Char(ch),
                modifiers: KeyModifiers::NONE,
                kind: KeyEventKind::Press,
                state: crossterm::event::KeyEventState::NONE,
            };
            app.prompt.handle_key(key).await;
            expected.push(ch);
            // The typed text must appear in the prompt snapshot after a
            // single `handle_key` await, not deferred to a later tick. The
            // `dispatch_key` helper relies on this so a render right after
            // a key produces the visible character.
            let text = app.prompt.snapshot().text();
            assert!(
                text.contains(&expected),
                "after typing {expected:?} the prompt text {text:?} must include it"
            );
        }
    }

    struct NoopStream;

    #[async_trait::async_trait]
    impl runie_core::provider::stream_fn::StreamFn for NoopStream {
        async fn stream(
            &self,
            _model: &runie_core::types::Model,
            _context: &runie_core::types::AgentContext,
            _options: Option<runie_core::types::SimpleStreamOptions>,
        ) -> Result<
            runie_core::provider::stream_fn::AssistantMessageEventStream,
            runie_core::provider::stream_fn::StreamError,
        > {
            Ok(Box::pin(futures::stream::empty()))
        }
    }

    async fn prompt_test_app() -> runie_tui::app::App {
        use runie_core::events::EventBus;
        use runie_core::provider::ProviderActor;
        use runie_core::queues::{FollowUpQueueActor, SteeringQueueActor};
        use runie_core::r#loop::{LoopActor, LoopDeps};
        use runie_core::state::AgentStateActor;
        use runie_core::tools::executor::ToolExecHooks;
        use runie_core::tools::{ToolExecutorActor, ToolRegistry};
        use runie_core::types::ToolExecutionMode;
        let bus = EventBus::new();
        let deps = LoopDeps {
            state: AgentStateActor::new(),
            steering: SteeringQueueActor::new(),
            follow_up: FollowUpQueueActor::new(),
            tool_executor: ToolExecutorActor::new_live(std::sync::Arc::new(ToolRegistry::new())),
            provider: ProviderActor::new(std::sync::Arc::new(NoopStream)),
            bus: bus.clone(),
            subscribers: runie_core::events::SubscriberRegistry::new(),
            hooks: ToolExecHooks::default(),
            turn_hooks: runie_core::hooks::TurnHooks::default(),
            transform_context: None,
            api_key_resolver: None,
            context_recovery: None,
            convert_to_llm: None,
            stream_options: Default::default(),
            abort: None,
            tool_execution_mode: ToolExecutionMode::Parallel,
            steering_mode: runie_core::types::QueueMode::OneAtATime,
            follow_up_mode: runie_core::types::QueueMode::OneAtATime,
        };
        runie_tui::app::App::new(LoopActor::new(deps), bus)
    }

    /// Pin down that the prompt snapshot reflects every key in a long
    /// burst after a single `handle_key` await, and that the final
    /// snapshot equals the concatenated input verbatim. Mirrors the
    /// wiring in `prompt_actor_reduces_each_press_key_independently`
    /// so the live main loop's `dispatch_key` path stays consistent
    /// across a 128-key press. A regression that batches keys through a
    /// single awaited reducer would surface here as a stale snapshot
    /// mid-burst, while a regression that drops characters would fail
    /// the verbatim equality check at the end.
    #[tokio::test]
    #[allow(
        clippy::too_many_lines,
        reason = "self-contained App wiring mirrors the runie.rs bin for fidelity"
    )]
    async fn prompt_actor_reduces_one_hundred_twenty_eight_keys_into_snapshot() {
        use crossterm::event::{KeyCode, KeyEvent, KeyEventKind, KeyModifiers};
        let app = prompt_test_app().await;

        let pool = prompt_key_pool();

        let mut expected = String::new();
        for (i, &ch) in pool.iter().enumerate() {
            let key = KeyEvent {
                code: KeyCode::Char(ch),
                modifiers: KeyModifiers::NONE,
                kind: KeyEventKind::Press,
                state: crossterm::event::KeyEventState::NONE,
            };
            app.prompt.handle_key(key).await;
            expected.push(ch);
            // After every awaited dispatch the prompt snapshot must end
            // with the cumulative prefix; a regression that defers the
            // snapshot until a later tick would surface here as a stale
            // text after `i + 1` keys.
            let text = app.prompt.snapshot().text();
            assert!(
                text.ends_with(&expected),
                "after key {i} ({ch:?}) the prompt text {text:?} must end with {expected:?}"
            );
        }
        // The buffer starts empty and we only ever insert chars, so the
        // final snapshot must equal the concatenated input verbatim.
        assert_eq!(
            app.prompt.snapshot().text(),
            expected,
            "final snapshot must equal the concatenated input verbatim"
        );
    }

    fn prompt_key_pool() -> Vec<char> {
        let mut pool = Vec::with_capacity(128);
        pool.extend('a'..='z');
        pool.extend('A'..='Z');
        pool.extend('0'..='9');
        pool.extend("!\"#$%&'()*+,-./:;<=>?@[\\]^_`{|}~".chars());
        pool.push(' ');
        pool.extend("¡¢£¤¥¦§¨©ª«¬®¯°±²³´µ¶·¸¹º»¼½¾¿ÀÁÂ".chars());
        debug_assert_eq!(pool.len(), 128);
        pool
    }
}
