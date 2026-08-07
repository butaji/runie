//! YAML-driven e2e test runner for `runie-tui`.
//!
//! Each YAML fixture under `tests/yaml_fixtures/*.yaml` is loaded, parsed into
//! a `Scenario`, then executed against a real `LoopActor` + `EventRenderer`.
//! The runner applies the fixture's assertions against the recorded events
//! and the rendered scrollback.

use std::collections::BTreeMap;
use std::path::Path;
use std::sync::Arc;

use crate::event_renderer::EventRenderer;
use crate::widgets::{FeedSnapshot, Line, LineKind, Scrollback, ScrollbackMsg, ToolBlock};
use parking_lot::Mutex;
use ratatui::buffer::Buffer;
use runie_core::events::{EventBus, Subscriber};
use runie_core::provider::stream_fn::{
    AssistantMessageEventStream, StreamError, StreamFn, WebSocketAdapter,
};
use runie_core::provider::ProviderActor;
use runie_core::queues::{FollowUpQueueActor, SteeringQueueActor};
use runie_core::r#loop::{LoopActor, LoopDeps};
use runie_core::session::SessionActor;
use runie_core::state::AgentStateActor;
use runie_core::tools::executor::ToolExecHooks;
use runie_core::tools::{ToolExecutorActor, ToolRegistry};
use runie_core::types::{
    AgentContext, AgentEvent, AgentMessage, AgentTool, AgentToolResult, AssistantContent,
    AssistantMessage, AssistantMessageEvent, CacheRetention, DeferredHandle, Model,
    SimpleStreamOptions, StopReason, ThinkingLevel, ToolExecutionMode, ToolResultContent, Usage,
    UserContent, UserMessage, WaitingReason,
};
use serde::Deserialize;
use tokio::sync::broadcast;
use unicode_width::UnicodeWidthStr;

#[derive(Debug, Deserialize)]
pub struct Scenario {
    pub name: String,
    pub initial_prompt: Option<String>,
    #[serde(default)]
    pub follow_up: Vec<String>,
    #[serde(default)]
    pub steering_mode: Option<runie_core::types::QueueMode>,
    #[serde(default)]
    pub follow_up_mode: Option<runie_core::types::QueueMode>,
    #[serde(default)]
    pub tool_execution: Option<ToolExecutionMode>,
    /// Provider request options declared by the replay fixture. These stay
    /// YAML-editable so provider contract experiments do not require Rust
    /// recompilation.
    #[serde(default)]
    pub provider_options: ProviderOptionsSpec,
    #[serde(default)]
    pub tools: Vec<ToolSpec>,
    #[serde(default)]
    pub context: ContextSpec,
    /// Optional validated Pi JSONL seed restored through the SessionActor
    /// mailbox before this scenario's event sequence runs.
    #[serde(default)]
    pub session_restore: Option<String>,
    pub events: Vec<EventSpec>,
    /// Capture the frame after tool execution while the next model request is
    /// still pending. This models Grok's stable waiting/feed boundary.
    #[serde(default)]
    pub capture_while_waiting: bool,
    /// Deterministic user-row clock for full-frame replay assertions.
    pub prompt_timestamp: Option<String>,
    /// Unix timestamp carried by YAML-created initial user messages. Keeping
    /// this in the fixture makes timestamped replay frames event-controlled
    /// instead of relying on the live wall clock or a hard-coded sentinel.
    #[serde(default = "default_replay_prompt_timestamp")]
    pub initial_prompt_timestamp: i64,
    /// Deterministic Pi tool-result timestamp for replay.
    #[serde(default)]
    pub tool_result_timestamp: i64,
    #[serde(default)]
    pub assertions: Assertions,
}

fn default_replay_prompt_timestamp() -> i64 {
    1
}

impl Scenario {
    fn initial_prompt_timestamp(&self) -> i64 {
        self.initial_prompt_timestamp
    }
}

#[derive(Debug, Deserialize, Default, Clone)]
pub struct ProviderOptionsSpec {
    #[serde(default)]
    pub model_headers: Option<std::collections::HashMap<String, String>>,
    #[serde(default)]
    pub model_max_tokens: Option<u64>,
    #[serde(default)]
    pub session_id: Option<String>,
    #[serde(default)]
    pub api_key: Option<String>,
    #[serde(default)]
    pub headers: Option<std::collections::HashMap<String, String>>,
    #[serde(default)]
    pub env: Option<std::collections::HashMap<String, String>>,
    #[serde(default)]
    pub metadata: Option<std::collections::HashMap<String, serde_json::Value>>,
    #[serde(default)]
    pub transport: Option<String>,
    #[serde(default)]
    pub cache_retention: Option<String>,
    #[serde(default)]
    pub websocket_connect_timeout_ms: Option<u64>,
    #[serde(default)]
    pub thinking_budgets: Option<runie_core::types::ThinkingBudgets>,
    #[serde(default)]
    pub reasoning: Option<ThinkingLevel>,
    #[serde(default)]
    pub deferred: Option<runie_core::types::DeferredRequest>,
    #[serde(default)]
    pub temperature: Option<f64>,
    #[serde(default)]
    pub max_tokens: Option<u64>,
    #[serde(default)]
    pub timeout_ms: Option<u64>,
    #[serde(default)]
    pub max_retries: Option<u32>,
    #[serde(default)]
    pub max_retry_delay_ms: Option<u64>,
    #[serde(default)]
    pub sampling_params: Option<std::collections::HashMap<String, serde_json::Value>>,
}

impl ProviderOptionsSpec {
    fn stream_options(&self) -> runie_core::types::SimpleStreamOptions {
        runie_core::types::SimpleStreamOptions {
            session_id: self.session_id.clone(),
            api_key: self.api_key.clone(),
            headers: self.headers.clone(),
            env: self.env.clone(),
            metadata: self.metadata.clone(),
            transport: self.transport.as_deref().map(parse_provider_transport),
            cache_retention: self.cache_retention.as_deref().map(parse_cache_retention),
            websocket_connect_timeout_ms: self.websocket_connect_timeout_ms,
            thinking_budgets: self.thinking_budgets.clone(),
            reasoning: self.reasoning,
            deferred: self.deferred.clone(),
            temperature: self.temperature,
            max_tokens: self.max_tokens,
            timeout_ms: self.timeout_ms,
            max_retries: self.max_retries,
            max_retry_delay_ms: self.max_retry_delay_ms,
            sampling_params: self.sampling_params.clone(),
            ..Default::default()
        }
    }
}

fn parse_provider_transport(value: &str) -> runie_core::types::ProviderTransport {
    match value {
        "sse" => runie_core::types::ProviderTransport::Sse,
        "websocket" => runie_core::types::ProviderTransport::Websocket,
        "websocket-cached" => runie_core::types::ProviderTransport::WebsocketCached,
        "auto" => runie_core::types::ProviderTransport::Auto,
        other => panic!("unknown provider transport: {other}"),
    }
}

fn parse_cache_retention(value: &str) -> CacheRetention {
    match value {
        "none" => CacheRetention::None,
        "short" => CacheRetention::Short,
        "long" => CacheRetention::Long,
        other => panic!("unknown cache retention: {other}"),
    }
}

fn provider_transport_name(value: runie_core::types::ProviderTransport) -> &'static str {
    match value {
        runie_core::types::ProviderTransport::Sse => "sse",
        runie_core::types::ProviderTransport::Websocket => "websocket",
        runie_core::types::ProviderTransport::WebsocketCached => "websocket-cached",
        runie_core::types::ProviderTransport::Auto => "auto",
    }
}

fn cache_retention_name(value: CacheRetention) -> &'static str {
    match value {
        CacheRetention::None => "none",
        CacheRetention::Short => "short",
        CacheRetention::Long => "long",
    }
}

#[derive(Debug, Deserialize, Default, Clone)]
pub struct ContextSpec {
    #[serde(default)]
    pub system_prompt: String,
    #[serde(default)]
    pub messages: Vec<String>,
    /// Explicit Pi-style empty tool set; omission keeps registered defaults.
    #[serde(default)]
    pub disable_tools: bool,
}

impl Scenario {
    fn agent_context(&self) -> AgentContext {
        AgentContext {
            system_prompt: self.context.system_prompt.clone(),
            messages: self
                .context
                .messages
                .iter()
                .map(|text| {
                    AgentMessage::User(UserMessage {
                        content: vec![UserContent::Text { text: text.clone() }],
                        timestamp: 0,
                    })
                })
                .collect(),
            // Omitted in YAML: let the scenario's registered executor tools
            // supply the Pi-compatible default.
            tools: self.context.disable_tools.then_some(Vec::new()),
        }
    }
}

#[derive(Debug, Deserialize)]
pub struct ToolSpec {
    pub name: String,
    /// Optional Pi tool presentation metadata.
    #[serde(default)]
    pub label: Option<String>,
    #[serde(default)]
    pub description: Option<String>,
    #[serde(default = "default_tool_kind")]
    pub kind: String,
    /// Optional Pi-compatible JSON Schema used by deterministic replay tools.
    #[serde(default)]
    pub parameters: Option<serde_json::Value>,
    /// Optional Pi `prepareArguments` replacement for deterministic replay.
    #[serde(default)]
    pub prepared_arguments: Option<serde_json::Value>,
    /// Optional deterministic result body/details for YAML-only replay cases.
    #[serde(default)]
    pub output: Option<String>,
    #[serde(default)]
    pub details: Option<serde_json::Value>,
    /// Optional Pi-compatible token usage returned by the deterministic tool.
    #[serde(default)]
    pub usage: Option<Usage>,
    #[serde(default)]
    pub error: Option<String>,
    #[serde(default)]
    pub media: Option<String>,
    /// Pi-compatible terminal hint for this deterministic tool result.
    #[serde(default)]
    pub terminate: bool,
    #[serde(default)]
    pub added_tool_names: Vec<String>,
    /// Optional Pi per-tool execution-mode override.
    #[serde(default)]
    pub execution_mode: Option<ToolExecutionMode>,
}

fn default_tool_kind() -> String {
    "echo".into()
}

#[derive(Debug, Deserialize, Clone)]
#[serde(untagged)]
pub enum EventSpec {
    Bare(String),
    TextDelta {
        text_delta: String,
    },
    TextStart {
        text_start: TextStartSpec,
    },
    TextEnd {
        text_end: TextEndSpec,
    },
    ThinkingDelta {
        thinking_delta: String,
    },
    ThinkingStart {
        thinking_start: ThinkingStartSpec,
    },
    ThinkingEnd {
        thinking_end: ThinkingEndSpec,
    },
    ToolCall {
        tool_call: ToolCallSpec,
    },
    ToolCallStart {
        tool_call_start: ToolCallSectionSpec,
    },
    ToolCallDelta {
        tool_call_delta: ToolCallSectionSpec,
    },
    ToolCallEnd {
        tool_call_end: ToolCallSectionSpec,
    },
    ToolUpdate {
        tool_update: ToolUpdateSpec,
    },
    ToolSeed {
        tool_seed: ToolSeedSpec,
    },
    Done {
        done: DoneSpec,
    },
    Error {
        error: String,
    },
    Waiting {
        waiting: String,
    },
    Theme {
        theme: String,
    },
    ContextWindow {
        context_window: u64,
    },
    ThinkingLevel {
        thinking_level: String,
    },
    ActiveTools {
        active_tools: Vec<String>,
    },
    SessionLabel {
        session_label: SessionLabelSpec,
    },
    BranchSummary {
        branch_summary: BranchSummarySpec,
    },
    CustomEntry {
        custom_entry: CustomEntrySpec,
    },
    Compaction {
        compaction: CompactionSpec,
    },
    OperationRecord {
        operation_record: OperationRecordSpec,
    },
    ToolMode {
        tool_mode: ToolModeSpec,
    },
    ToolFold {
        tool_fold: String,
    },
    ToolSelect {
        tool_select: String,
    },
    SelectRange {
        select_range: SelectionRangeSpec,
    },
    MouseSelectionStart {
        mouse_selection_start: CellPositionSpec,
    },
    MouseSelectionExtend {
        mouse_selection_extend: CellPositionSpec,
    },
    Scroll {
        scroll: i32,
    },
    ScrollInput {
        scroll_input: ScrollInputSpec,
    },
    ScrollRawInput {
        scroll_raw_input: ScrollInputSpec,
    },
    ScrollFlush {
        scroll_flush: ScrollFlushSpec,
    },
    ScrollFinalize,
    AnimationTicks {
        animation_ticks: usize,
    },
    LayoutMeasured {
        layout_measured: LayoutMeasuredSpec,
    },
    RevealLatest {
        reveal_latest: bool,
    },
    FollowLatest {
        follow_latest: bool,
    },
    BackgroundStart {
        background_start: BackgroundStartSpec,
    },
    BackgroundProgress {
        background_progress: BackgroundProgressSpec,
    },
    BackgroundEnd {
        background_end: BackgroundEndSpec,
    },
    BackgroundCancel {
        background_cancel: BackgroundCancelSpec,
    },
    WorkflowStart {
        workflow_start: WorkflowStartSpec,
    },
    WorkflowProgress {
        workflow_progress: WorkflowProgressSpec,
    },
    WorkflowEnd {
        workflow_end: WorkflowEndSpec,
    },
}

#[derive(Debug, Deserialize, Clone)]
pub struct BackgroundStartSpec {
    pub work_id: String,
    pub description: String,
    #[serde(default)]
    pub background: bool,
}

#[derive(Debug, Deserialize, Clone)]
pub struct BackgroundProgressSpec {
    pub work_id: String,
    pub description: String,
    pub activity: String,
}

#[derive(Debug, Deserialize, Clone)]
pub struct BackgroundEndSpec {
    pub work_id: String,
    pub description: String,
    #[serde(default)]
    pub is_error: bool,
    #[serde(default)]
    pub elapsed_ms: Option<u64>,
    #[serde(default)]
    pub error: Option<String>,
}

#[derive(Debug, Deserialize, Clone)]
pub struct BackgroundCancelSpec {
    pub work_id: String,
    pub description: String,
    #[serde(default)]
    pub elapsed_ms: Option<u64>,
}

#[derive(Debug, Deserialize, Clone)]
pub struct WorkflowStartSpec {
    pub run_id: String,
    pub name: String,
    pub objective: String,
}

#[derive(Debug, Deserialize, Clone)]
pub struct WorkflowProgressSpec {
    pub run_id: String,
    pub phase: String,
    pub state: String,
    #[serde(default)]
    pub active_agents: u32,
}

#[derive(Debug, Deserialize, Clone)]
pub struct WorkflowEndSpec {
    pub run_id: String,
    pub status: String,
    #[serde(default)]
    pub elapsed_ms: Option<u64>,
}

#[derive(Debug, Deserialize, Clone)]
pub struct BranchSummarySpec {
    pub from_id: String,
    pub summary: String,
    #[serde(default)]
    pub details: Option<serde_json::Value>,
}

#[derive(Debug, Deserialize, Clone)]
pub struct SessionLabelSpec {
    pub target_id: String,
    #[serde(default)]
    pub label: Option<String>,
}

#[derive(Debug, Deserialize, Clone)]
pub struct CustomEntrySpec {
    pub custom_type: String,
    #[serde(default)]
    pub data: Option<serde_json::Value>,
}

#[derive(Debug, Deserialize, Clone)]
pub struct CompactionSpec {
    pub summary: String,
    #[serde(default)]
    pub retained_tail: Vec<AgentMessage>,
    pub tokens_before: u64,
    #[serde(default)]
    pub details: Option<serde_json::Value>,
    #[serde(default)]
    pub usage: Option<Usage>,
}

#[derive(Debug, Deserialize, Clone)]
pub struct OperationRecordSpec {
    pub record_type: String,
    #[serde(default)]
    pub data: serde_json::Value,
}

#[derive(Debug, Deserialize, Clone)]
pub struct ToolModeSpec {
    pub tool_call_id: String,
    pub mode: runie_core::types::ToolDisplayMode,
}

#[derive(Debug, Deserialize, Clone)]
pub struct ToolUpdateSpec {
    pub tool_call_id: String,
    pub tool_name: String,
    #[serde(default)]
    pub args: serde_json::Value,
    #[serde(default)]
    pub partial_result: serde_json::Value,
}

#[derive(Debug, Deserialize, Clone)]
pub struct ToolSeedSpec {
    pub tool_call_id: String,
    pub header: String,
    /// Test-only lifecycle fact for inspecting a live card before completion.
    #[serde(default)]
    pub running: bool,
}

#[derive(Debug, Deserialize, Clone)]
pub struct ThinkingStartSpec {
    #[serde(default)]
    pub index: usize,
}

#[derive(Debug, Deserialize, Clone)]
pub struct ThinkingEndSpec {
    #[serde(default)]
    pub index: usize,
    pub content: String,
    #[serde(default)]
    pub elapsed_ms: Option<u64>,
}

#[derive(Debug, Deserialize, Clone)]
pub struct TextStartSpec {
    #[serde(default)]
    pub index: usize,
}

#[derive(Debug, Deserialize, Clone)]
pub struct TextEndSpec {
    #[serde(default)]
    pub index: usize,
    pub content: String,
}

fn waiting_name(name: &str) -> WaitingReason {
    match name {
        "subagent" => WaitingReason::Subagent,
        "tasks_complete" => WaitingReason::TasksComplete,
        "sleep" => WaitingReason::Sleep,
        _ => WaitingReason::Model,
    }
}

#[derive(Debug, Deserialize, Clone)]
pub struct DoneSpec {
    #[serde(default)]
    pub stop_reason: StopReasonSpec,
    /// Provider usage is part of the terminal event and must be fixture-owned
    /// for deterministic footer parity; omitted usage keeps the zero default.
    #[serde(default)]
    pub usage: Usage,
    #[serde(default)]
    pub deferred: Option<DeferredHandle>,
}

#[derive(Debug, Deserialize, Clone)]
pub struct ToolCallSpec {
    pub name: String,
    #[serde(default)]
    pub args: serde_json::Value,
}

#[derive(Debug, Deserialize, Clone)]
pub struct ToolCallSectionSpec {
    #[serde(default)]
    pub index: usize,
    #[serde(default)]
    pub id: String,
    #[serde(default)]
    pub name: String,
    #[serde(default)]
    pub arguments: serde_json::Value,
    #[serde(default)]
    pub delta: String,
}

#[derive(Debug, Default, Deserialize, Clone)]
#[serde(rename_all = "snake_case")]
pub enum StopReasonSpec {
    #[default]
    Stop,
    ToolUse,
    MaxTokens,
    Aborted,
    Deferred,
}

impl From<&StopReasonSpec> for StopReason {
    fn from(s: &StopReasonSpec) -> Self {
        match s {
            StopReasonSpec::Stop => StopReason::Stop,
            StopReasonSpec::ToolUse => StopReason::ToolUse,
            StopReasonSpec::MaxTokens => StopReason::MaxTokens,
            StopReasonSpec::Aborted => StopReason::Aborted,
            StopReasonSpec::Deferred => StopReason::Deferred,
        }
    }
}

impl EventSpec {
    #[allow(
        clippy::too_many_lines,
        reason = "keeps the declarative assistant event mapping together"
    )]
    fn to_assistant_event(&self, index: usize) -> Option<AssistantMessageEvent> {
        match self {
            Self::Bare(s) if s == "start" => Some(AssistantMessageEvent::Start {
                partial: AssistantMessage::default(),
            }),
            Self::Bare(s) if s == "reset" => None,
            Self::TextDelta { text_delta } => Some(AssistantMessageEvent::TextDelta {
                index: 0,
                delta: text_delta.clone(),
                partial: AssistantMessage::default(),
            }),
            Self::TextStart { text_start } => Some(AssistantMessageEvent::TextStart {
                index: text_start.index,
                partial: AssistantMessage::default(),
            }),
            Self::TextEnd { text_end } => Some(AssistantMessageEvent::TextEnd {
                index: text_end.index,
                content: text_end.content.clone(),
                partial: AssistantMessage::default(),
            }),
            Self::ThinkingDelta { thinking_delta } => Some(AssistantMessageEvent::ThinkingDelta {
                index: 1,
                delta: thinking_delta.clone(),
                partial: AssistantMessage::default(),
            }),
            Self::ThinkingStart { thinking_start } => Some(AssistantMessageEvent::ThinkingStart {
                index: thinking_start.index,
                partial: AssistantMessage::default(),
            }),
            Self::ThinkingEnd { thinking_end } => Some(AssistantMessageEvent::ThinkingEnd {
                index: thinking_end.index,
                content: thinking_end.content.clone(),
                elapsed_ms: thinking_end.elapsed_ms,
                partial: AssistantMessage::default(),
            }),
            Self::ToolCall { tool_call } => Some(AssistantMessageEvent::ToolCallDelta {
                index,
                delta: serde_json::to_string(&tool_call.args).unwrap_or_default(),
                partial: AssistantMessage::with_tool_call(runie_core::types::ToolCall {
                    id: format!("call-{index}"),
                    name: tool_call.name.clone(),
                    arguments: tool_call.args.clone(),
                    thought_signature: None,
                }),
            }),
            Self::ToolCallStart { tool_call_start } => Some(AssistantMessageEvent::ToolCallStart {
                index: tool_call_start.index,
                partial: AssistantMessage::with_tool_call(runie_core::types::ToolCall {
                    id: tool_call_start.id.clone(),
                    name: tool_call_start.name.clone(),
                    arguments: tool_call_start.arguments.clone(),
                    thought_signature: None,
                }),
            }),
            Self::ToolCallDelta { tool_call_delta } => Some(AssistantMessageEvent::ToolCallDelta {
                index: tool_call_delta.index,
                delta: tool_call_delta.delta.clone(),
                partial: AssistantMessage::with_tool_call(runie_core::types::ToolCall {
                    id: tool_call_delta.id.clone(),
                    name: tool_call_delta.name.clone(),
                    arguments: tool_call_delta.arguments.clone(),
                    thought_signature: None,
                }),
            }),
            Self::ToolCallEnd { tool_call_end } => Some(AssistantMessageEvent::ToolCallEnd {
                index: tool_call_end.index,
                tool_call: runie_core::types::ToolCall {
                    id: tool_call_end.id.clone(),
                    name: tool_call_end.name.clone(),
                    arguments: tool_call_end.arguments.clone(),
                    thought_signature: None,
                },
                partial: AssistantMessage::default(),
            }),
            Self::ToolUpdate { .. } | Self::ToolSeed { .. } => None,
            Self::Done { done } => Some(AssistantMessageEvent::Done {
                stop_reason: StopReason::from(&done.stop_reason),
                usage: done.usage.clone(),
                message: done.deferred.clone().map(|deferred| AssistantMessage {
                    stop_reason: Some(StopReason::Deferred),
                    deferred: Some(deferred),
                    usage: done.usage.clone(),
                    ..AssistantMessage::default()
                }),
            }),
            Self::Error { error } => Some(AssistantMessageEvent::Error {
                reason: StopReason::Error,
                error: AssistantMessage::with_error(StopReason::Error, error.clone()),
            }),
            Self::Waiting { .. } => None,
            Self::Theme { .. } => None,
            Self::ContextWindow { .. } => None,
            Self::ThinkingLevel { .. } => None,
            Self::ActiveTools { .. } => None,
            Self::SessionLabel { .. } => None,
            Self::BranchSummary { .. } => None,
            Self::CustomEntry { .. } => None,
            Self::Compaction { .. } => None,
            Self::OperationRecord { .. } => None,
            Self::ToolMode { .. } => None,
            Self::ToolFold { .. } => None,
            Self::ToolSelect { .. } => None,
            Self::SelectRange { .. } => None,
            Self::MouseSelectionStart { .. } | Self::MouseSelectionExtend { .. } => None,
            Self::Scroll { .. } => None,
            Self::ScrollInput { .. } => None,
            Self::ScrollRawInput { .. } => None,
            Self::ScrollFlush { .. } | Self::ScrollFinalize => None,
            Self::AnimationTicks { .. } => None,
            Self::LayoutMeasured { .. } => None,
            Self::RevealLatest { .. } => None,
            Self::FollowLatest { .. } => None,
            Self::BackgroundStart { .. }
            | Self::BackgroundProgress { .. }
            | Self::BackgroundEnd { .. }
            | Self::BackgroundCancel { .. }
            | Self::WorkflowStart { .. }
            | Self::WorkflowProgress { .. }
            | Self::WorkflowEnd { .. } => None,
            Self::Bare(other)
                if matches!(
                    other.as_str(),
                    "scroll_finalize"
                        | "mouse_selection_commit"
                        | "clear_cell_selection"
                        | "copy_selection"
                        | "clear_copy_request"
                ) =>
            {
                None
            }
            Self::Bare(other) => panic!("unknown event kind: {other:?}"),
        }
    }

    #[allow(
        clippy::too_many_lines,
        reason = "the declarative control-event projection stays in one table"
    )]
    fn waiting_event(&self) -> Option<AgentEvent> {
        match self {
            Self::Bare(s) if s == "reset" => Some(AgentEvent::Reset),
            Self::Waiting { waiting } => Some(AgentEvent::Waiting {
                reason: waiting_name(waiting),
            }),
            Self::Theme { theme } => Some(AgentEvent::ThemeChanged {
                theme: parse_theme(theme),
            }),
            Self::ThinkingLevel { thinking_level } => Some(AgentEvent::ThinkingLevelChanged {
                level: parse_thinking_level(thinking_level),
            }),
            Self::ActiveTools { active_tools } => Some(AgentEvent::ActiveToolsChanged {
                tool_names: active_tools.clone(),
            }),
            Self::SessionLabel { session_label } => Some(AgentEvent::SessionLabelChanged {
                target_id: session_label.target_id.clone(),
                label: session_label.label.clone(),
            }),
            Self::BranchSummary { branch_summary } => Some(AgentEvent::BranchSummaryCreated {
                from_id: branch_summary.from_id.clone(),
                summary: branch_summary.summary.clone(),
                details: branch_summary.details.clone(),
            }),
            Self::CustomEntry { custom_entry } => Some(AgentEvent::CustomSessionEntryCreated {
                custom_type: custom_entry.custom_type.clone(),
                data: custom_entry.data.clone(),
            }),
            Self::Compaction { compaction } => Some(AgentEvent::CompactionCreated {
                summary: compaction.summary.clone(),
                retained_tail: compaction.retained_tail.clone(),
                tokens_before: compaction.tokens_before,
                details: compaction.details.clone(),
                usage: compaction.usage.clone(),
            }),
            Self::OperationRecord { operation_record } => {
                Some(AgentEvent::OperationRecordCreated {
                    record_type: operation_record.record_type.clone(),
                    data: operation_record.data.clone(),
                })
            }
            Self::ToolMode { tool_mode } => Some(AgentEvent::ToolDisplayModeChanged {
                tool_call_id: tool_mode.tool_call_id.clone(),
                mode: tool_mode.mode,
            }),
            Self::ToolUpdate { tool_update } => Some(AgentEvent::ToolExecutionUpdate {
                tool_call_id: tool_update.tool_call_id.clone(),
                tool_name: tool_update.tool_name.clone(),
                args: tool_update.args.clone(),
                partial_result: tool_update.partial_result.clone(),
            }),
            Self::ThinkingStart { .. } | Self::ThinkingEnd { .. } => None,
            Self::ToolFold { .. } => None,
            Self::ToolSelect { .. } => None,
            Self::LayoutMeasured { .. } => None,
            Self::BackgroundStart { background_start } => Some(AgentEvent::BackgroundWorkStarted {
                work_id: background_start.work_id.clone(),
                description: background_start.description.clone(),
                background: background_start.background,
            }),
            Self::BackgroundProgress {
                background_progress,
            } => Some(AgentEvent::BackgroundWorkProgress {
                work_id: background_progress.work_id.clone(),
                description: background_progress.description.clone(),
                activity: background_progress.activity.clone(),
            }),
            Self::BackgroundEnd { background_end } => Some(AgentEvent::BackgroundWorkFinished {
                work_id: background_end.work_id.clone(),
                description: background_end.description.clone(),
                is_error: background_end.is_error,
                elapsed_ms: background_end.elapsed_ms,
                error: background_end.error.clone(),
            }),
            Self::BackgroundCancel { background_cancel } => {
                Some(AgentEvent::BackgroundWorkCancelled {
                    work_id: background_cancel.work_id.clone(),
                    description: background_cancel.description.clone(),
                    elapsed_ms: background_cancel.elapsed_ms,
                })
            }
            Self::WorkflowStart { workflow_start } => Some(AgentEvent::WorkflowStarted {
                run_id: workflow_start.run_id.clone(),
                name: workflow_start.name.clone(),
                objective: workflow_start.objective.clone(),
            }),
            Self::WorkflowProgress { workflow_progress } => Some(AgentEvent::WorkflowProgress {
                run_id: workflow_progress.run_id.clone(),
                phase: workflow_progress.phase.clone(),
                state: workflow_progress.state.clone(),
                active_agents: workflow_progress.active_agents,
            }),
            Self::WorkflowEnd { workflow_end } => Some(AgentEvent::WorkflowFinished {
                run_id: workflow_end.run_id.clone(),
                status: workflow_end.status.clone(),
                elapsed_ms: workflow_end.elapsed_ms,
            }),
            _ => None,
        }
    }
}

fn parse_theme(theme: &str) -> runie_core::types::ThemeKind {
    match theme.to_ascii_lowercase().as_str() {
        "grok_day" | "grok-day" | "day" => runie_core::types::ThemeKind::GrokDay,
        "tokyo_night" | "tokyo-night" => runie_core::types::ThemeKind::TokyoNight,
        "rose_pine_moon" | "rose-pine-moon" => runie_core::types::ThemeKind::RosePineMoon,
        "oscura_midnight" | "oscura-midnight" => runie_core::types::ThemeKind::OscuraMidnight,
        "auto" => runie_core::types::ThemeKind::Auto,
        "terminal_native" | "terminal-native" | "native" => {
            runie_core::types::ThemeKind::TerminalNative
        }
        _ => runie_core::types::ThemeKind::GrokNight,
    }
}

fn parse_thinking_level(level: &str) -> ThinkingLevel {
    match level.to_ascii_lowercase().as_str() {
        "minimal" => ThinkingLevel::Minimal,
        "low" => ThinkingLevel::Low,
        "medium" => ThinkingLevel::Medium,
        "high" => ThinkingLevel::High,
        "xhigh" | "x-high" => ThinkingLevel::XHigh,
        "max" => ThinkingLevel::Max,
        _ => ThinkingLevel::Off,
    }
}

#[derive(Debug, Deserialize, Default)]
pub struct Assertions {
    #[serde(default)]
    pub transcript_contains: Vec<String>,
    #[serde(default)]
    pub events: Vec<String>,
    /// When present, require the complete event vector in order. This keeps
    /// event-sequence/state tests declarative and recompilation-free.
    #[serde(default)]
    pub exact_events: Option<Vec<String>>,
    /// Optional exact assertion over the closed Pi-core event boundary. This
    /// is intentionally separate from `exact_events`, which includes
    /// Runie/TUI compatibility events when a scenario declares them.
    #[serde(default)]
    pub pi_events: Option<Vec<String>>,
    #[serde(default)]
    pub turn_starts: Option<usize>,
    #[serde(default)]
    pub scrollback_lines: Vec<LineAssertion>,
    /// Optional in-process visual check: drive the TUI App via TestBackend
    /// at the given viewport size, then assert substrings appear in / are
    /// excluded from the rendered screen.
    #[serde(default)]
    pub visual: Option<VisualAssertions>,
    #[serde(default)]
    pub state: Option<StateAssertions>,
    #[serde(default)]
    pub provider_options: Option<ProviderOptionsAssertions>,
    /// Exact event sequence observed by the awaited actor listener path.
    #[serde(default)]
    pub listener_events: Option<Vec<String>>,
}

#[derive(Debug, Deserialize, Default, Clone)]
pub struct ProviderOptionsAssertions {
    pub session_id: Option<String>,
    pub api_key: Option<String>,
    pub headers: Option<std::collections::HashMap<String, String>>,
    pub env: Option<std::collections::HashMap<String, String>>,
    pub metadata: Option<std::collections::HashMap<String, serde_json::Value>>,
    pub transport: Option<String>,
    pub cache_retention: Option<String>,
    pub websocket_connect_timeout_ms: Option<u64>,
    pub thinking_budgets: Option<runie_core::types::ThinkingBudgets>,
    pub reasoning: Option<ThinkingLevel>,
    pub deferred: Option<runie_core::types::DeferredRequest>,
    pub temperature: Option<f64>,
    pub max_tokens: Option<u64>,
    pub timeout_ms: Option<u64>,
    pub max_retries: Option<u32>,
    pub max_retry_delay_ms: Option<u64>,
    pub sampling_params: Option<std::collections::HashMap<String, serde_json::Value>>,
}

/// Small assertion DSL used by the YAML projection oracle. It keeps the
/// fixture-facing field checks declarative while preserving one consistent
/// diagnostic shape; it has no state or rendering side effects.
macro_rules! assert_yaml_eq {
    ($expected:expr, $actual:expr, $field:literal) => {
        if let Some(expected) = $expected {
            let actual = $actual;
            if actual != expected {
                return Err(format!(
                    "state {} mismatch: expected {:?}, got {:?}",
                    $field, expected, actual
                ));
            }
        }
    };
}

#[derive(Debug, Deserialize, Default, Clone)]
pub struct StateAssertions {
    /// Renderer-independent status label (for example `thinking` or `ready`).
    pub status: Option<String>,
    /// Actor-owned theme after the declared theme event sequence.
    pub theme: Option<runie_core::types::ThemeKind>,
    pub is_streaming: Option<bool>,
    pub pending_tool_calls: Option<usize>,
    pub messages: Option<usize>,
    /// Stop reason on the latest actor-owned assistant message.
    pub assistant_stop_reason: Option<StopReasonSpec>,
    /// Deferred handle on the latest actor-owned assistant message.
    pub assistant_deferred: Option<DeferredHandle>,
    /// `addedToolNames` from the latest Pi tool result.
    pub tool_result_added_tool_names: Option<Vec<String>>,
    /// `details` from the latest Pi tool result.
    pub tool_result_details: Option<serde_json::Value>,
    /// `usage` from the latest Pi tool result.
    pub tool_result_usage: Option<Usage>,
    /// `isError` from the latest Pi tool result.
    pub tool_result_is_error: Option<bool>,
    /// Timestamp on the latest Pi tool result.
    pub tool_result_timestamp: Option<i64>,
    /// Arguments on the latest assistant tool call after preparation.
    pub tool_call_arguments: Option<serde_json::Value>,
    pub session_entries: Option<usize>,
    /// Complete Pi-compatible session statistics projection.
    pub session_stats: Option<serde_json::Value>,
    /// Ordered parent-linked Pi session nodes from the selected leaf.
    pub session_branch_entry_ids: Option<Vec<String>>,
    pub active_operations: Option<BTreeMap<String, String>>,
    /// Last Pi navigation intent reduced by the session actor.
    pub navigation: Option<NavigationAssertion>,
    /// Pure validation of the projected navigation IDs against the journal.
    pub navigation_validation: Option<NavigationValidationAssertion>,
    /// Terminal Pi operation outcomes keyed by operation ID.
    pub operation_outcomes: Option<BTreeMap<String, String>>,
    /// Pi operation intent kinds keyed by operation ID.
    pub operation_kinds: Option<BTreeMap<String, String>>,
    /// Pi failure metadata keyed by operation ID.
    pub operation_errors: Option<BTreeMap<String, OperationErrorAssertion>>,
    /// Ordered Pi session configuration-record kinds from the actor journal.
    pub session_config_records: Option<Vec<String>>,
    /// Ordered admitted Pi operation-lane record kinds.
    pub session_lane_records: Option<Vec<String>>,
    /// Ordered actor-owned assistant step operation IDs.
    pub session_step_run_ids: Option<Vec<String>>,
    /// Ordered actor-owned assistant step result entry IDs.
    pub session_step_result_entry_ids: Option<Vec<String>>,
    /// Complete actor-produced Pi `tool_started` payloads, in lane order.
    pub session_tool_started: Option<Vec<serde_json::Value>>,
    /// Message entry IDs selected after the newest compaction boundary.
    pub compaction_context_entry_ids: Option<Vec<String>>,
    /// Internal Pi context-message roles after compaction, before provider
    /// conversion applies wire-role rules.
    pub compaction_context_roles: Option<Vec<String>>,
    /// Runtime-declared token estimates for the pure compaction oracle.
    pub compaction_token_estimates: Option<Vec<u64>>,
    pub compaction_keep_recent_tokens: Option<u64>,
    pub compaction_first_kept_entry_index: Option<usize>,
    pub compaction_split_turn: Option<bool>,
    pub compaction_history_indices: Option<Vec<usize>>,
    pub compaction_turn_prefix_indices: Option<Vec<usize>>,
    pub compaction_retained_indices: Option<Vec<usize>>,
    pub compaction_tokens_before: Option<u64>,
    pub compaction_context_tokens: Option<u64>,
    pub compaction_reserve_tokens: Option<u64>,
    pub compaction_enabled: Option<bool>,
    pub compaction_should_run: Option<bool>,
    /// Runtime-only messages for Pi context-token estimation coverage.
    pub context_usage_messages: Option<Vec<String>>,
    pub context_usage_tokens: Option<u64>,
    pub context_usage_reported_tokens: Option<u64>,
    pub context_usage_trailing_tokens: Option<u64>,
    pub context_usage_last_index: Option<usize>,
    /// Termination metadata on the latest actor-owned session entry.
    pub session_last_terminate: Option<bool>,
    pub tool_count: Option<usize>,
    /// Ordered labels from the actor-owned registered tool projection.
    pub tool_labels: Option<Vec<String>>,
    /// Ordered per-tool execution-mode overrides from the actor-owned registry.
    pub tool_execution_modes: Option<Vec<Option<ToolExecutionMode>>>,
    pub streaming_contains: Option<String>,
    pub error_contains: Option<String>,
    pub context_window: Option<u64>,
    pub system_prompt_contains: Option<String>,
    pub tool_blocks: Option<usize>,
    pub tool_output_lines: Option<usize>,
    pub tool_modes: Option<Vec<runie_core::types::ToolDisplayMode>>,
    /// Ordered running-state projection for typed tool cards.
    pub tool_running: Option<Vec<bool>>,
    /// Ordered semantic headers for the projected Grok tool blocks.
    pub tool_headers: Option<Vec<String>>,
    /// Ordered reducer identities for semantic tool-header lines. `null`
    /// denotes a compatibility-seeded row; numeric values are opaque live
    /// reducer tokens.
    pub tool_header_row_ids: Option<Vec<Option<u64>>>,
    /// Ordered lifecycle eligibility for the same semantic header rows. This
    /// distinguishes a retained opaque identity from an active event target.
    pub tool_header_row_active: Option<Vec<bool>>,
    /// Ordered output rows for each projected tool block.
    pub tool_outputs: Option<Vec<Vec<String>>>,
    /// Ordered semantic card-row roles across the transcript.
    pub tool_row_kinds: Option<Vec<runie_tui_model::ToolCardRowKind>>,
    /// Ordered renderer-neutral paint intents for semantic card rows.
    pub tool_row_paint_intents: Option<Vec<runie_tui_model::ToolCardPaintIntent>>,
    pub tool_row_member_indices: Option<Vec<usize>>,
    pub tool_kinds: Option<Vec<crate::widgets::ToolCardKind>>,
    pub selected_tool_id: Option<String>,
    pub selected_entry: Option<usize>,
    pub selected_member_index: Option<usize>,
    pub selection_anchor: Option<usize>,
    pub selection_head: Option<usize>,
    pub cell_selection: Option<CellSelectionAssertion>,
    pub copy_selection_requested: Option<bool>,
    pub autoscroll: Option<bool>,
    pub scroll_offset: Option<usize>,
    /// Ordered cadence flush/finalize records from declarative scroll input.
    pub scroll_flushes: Option<Vec<ScrollFlushAssertion>>,
    pub measured_content_rows: Option<usize>,
    pub measured_viewport_rows: Option<usize>,
    pub measured_anchor_row: Option<usize>,
    pub thinking_level: Option<ThinkingLevel>,
    pub thinking_elapsed_ms: Option<u64>,
    /// Assert that an optional duration/fact has been cleared by the event
    /// sequence, without overloading YAML `null` (which also means omitted).
    pub thinking_elapsed_cleared: Option<bool>,
    /// Number of deterministic animation frames reduced by the status/feed
    /// actors. This keeps animation replay event-driven and sleep-free.
    pub animation_frame: Option<usize>,
    pub elapsed_ticks: Option<u64>,
    pub animation_demand: Option<bool>,
    pub reasoning_expanded: Option<bool>,
    pub activity_expanded: Option<bool>,
    pub follow_latest_user: Option<bool>,
    /// Effective queue policies projected by the loop actor.
    pub steering_mode: Option<runie_core::types::QueueMode>,
    pub follow_up_mode: Option<runie_core::types::QueueMode>,
    pub loop_running: Option<bool>,
    pub abort_requested: Option<bool>,
    /// Whether the feed actor still considers a turn lifecycle open.
    pub turn_started: Option<bool>,
    pub tool_execution: Option<ToolExecutionMode>,
    /// Exact actor-owned workflow projections keyed by their stable run id.
    /// YAML owns the expected state; the runner only performs generic field
    /// comparison so workflow fixtures stay recompilation-free.
    pub workflows: Option<BTreeMap<String, WorkflowStateAssertion>>,
    /// Exact actor-owned background-work projections keyed by work ID.
    pub background_work: Option<BTreeMap<String, BackgroundWorkStateAssertion>>,
}

#[derive(Debug, Deserialize, Default, Clone, PartialEq, Eq)]
pub struct ScrollFlushAssertion {
    pub kind: String,
    pub at_ms: Option<u64>,
    pub lines: i32,
    pub backlog: i32,
    pub dropped: i32,
}

#[derive(Debug, Deserialize, Default, Clone, PartialEq, Eq)]
pub struct NavigationAssertion {
    pub target_id: Option<String>,
    #[serde(default)]
    pub summarize: bool,
    pub summary_entry_id: Option<String>,
}

#[derive(Debug, Deserialize, Default, Clone, PartialEq, Eq)]
pub struct NavigationValidationAssertion {
    pub target_exists: bool,
    pub summary_exists: bool,
}

#[derive(Debug, Deserialize, Default, Clone, PartialEq, Eq)]
pub struct OperationErrorAssertion {
    pub code: String,
    pub message: String,
}

#[derive(Debug, Deserialize, Default, Clone)]
pub struct WorkflowStateAssertion {
    pub name: Option<String>,
    pub objective: Option<String>,
    pub phase: Option<String>,
    pub state: Option<String>,
    pub active_agents: Option<u32>,
    pub status: Option<String>,
    pub elapsed_ms: Option<u64>,
}

#[derive(Debug, Deserialize, Default, Clone)]
pub struct BackgroundWorkStateAssertion {
    pub description: Option<String>,
    pub activity: Option<String>,
    pub background: Option<bool>,
    pub status: Option<String>,
    pub elapsed_ms: Option<u64>,
    pub error: Option<String>,
}

#[derive(Debug, Deserialize, Default, Clone)]
pub struct VisualAssertions {
    #[serde(default = "default_visual_cols")]
    pub cols: u16,
    #[serde(default = "default_visual_rows")]
    pub rows: u16,
    #[serde(default)]
    pub screen_text: Vec<String>,
    #[serde(default)]
    pub screen_excludes: Vec<String>,
    /// Optional cell-level semantic oracle. Each field is independently
    /// optional so YAML can pin only the glyph, palette role, or modifier that
    /// matters for a scenario without duplicating a complete screen dump.
    #[serde(default)]
    pub cell_assertions: Vec<CellAssertion>,
    /// Generic actor-owned UI projection assertions. These are evaluated
    /// after declarative key steps have gone through the UiActor mailbox.
    #[serde(default)]
    pub ui: Option<UiAssertions>,
    /// Steps the TUI should perform before snapshotting the screen.
    /// Each step is a key event (e.g. "hello", "Enter", "Ctrl+C").
    #[serde(default)]
    pub steps: Vec<String>,
    /// Interaction steps applied after the declared event stream and initial
    /// prompt have settled. This is the deterministic phase for testing
    /// viewport behavior on real feed content.
    #[serde(default)]
    pub post_steps: Vec<String>,
    /// If true, also spawn the real `runie` binary in a pty and assert
    /// the same `screen_text` / `screen_excludes` substrings there.
    /// Requires the standalone PTY harness. Until that harness is wired into
    /// the YAML runner, requesting this mode is a hard assertion failure.
    #[serde(default)]
    pub pty: bool,
    /// Render captured reasoning bodies instead of Grok's collapsed `Thought`
    /// summary. This keeps reasoning-fold scenarios declarative.
    #[serde(default)]
    pub reasoning_expanded: bool,
    /// Render grouped tool member rows instead of only the activity summary.
    #[serde(default)]
    pub activity_expanded: Option<bool>,
    /// Optional actor-owned dense-group centering intent after visual steps.
    #[serde(default)]
    pub center_revealed_entry: Option<bool>,
    #[serde(default)]
    pub header_meter: Option<String>,
    #[serde(default)]
    pub waiting_chrome: Option<String>,
    /// Expected adapter geometry for a complete terminal frame. This keeps
    /// wrapping/scrolling decisions inspectable without encoding Ratatui
    /// objects in YAML.
    #[serde(default)]
    pub layout: Option<LayoutAssertions>,
    /// Additional frozen geometries for the same event/state scenario.
    #[serde(default)]
    pub layout_matrix: Vec<LayoutMatrixCase>,
    /// Optional asciinema oracle. The runner selects the first terminal frame
    /// containing every marker and compares the requested row text with the
    /// Runie TestBackend frame. YAML owns the state/marker recipe; Rust only
    /// supplies the generic dump decoder.
    #[serde(default)]
    pub reference: Option<DumpReference>,
}

#[derive(Debug, Deserialize, Default, Clone)]
pub struct CellAssertion {
    pub col: u16,
    pub row: u16,
    pub symbol: Option<String>,
    pub fg: Option<String>,
    pub bg: Option<String>,
    pub bold: Option<bool>,
    pub italic: Option<bool>,
    pub underline: Option<bool>,
    pub inverse: Option<bool>,
}

#[derive(Debug, Deserialize, Default, Clone)]
pub struct UiAssertions {
    pub show_welcome: Option<bool>,
    pub shortcuts_open: Option<bool>,
    pub command_palette_open: Option<bool>,
    pub command_palette_query: Option<String>,
    pub command_palette_index: Option<usize>,
}

#[derive(Debug, Deserialize, Default, Clone, Copy)]
pub struct LayoutAssertions {
    pub header: RegionGeometry,
    pub scrollback: RegionGeometry,
    pub prompt: RegionGeometry,
    pub status: RegionGeometry,
    pub footer_badge: RegionGeometry,
}

#[derive(Debug, Deserialize, Default, Clone, Copy)]
pub struct RegionGeometry {
    pub x: u16,
    pub y: u16,
    pub width: u16,
    pub height: u16,
}

#[derive(Debug, Deserialize, Default, Clone)]
pub struct LayoutMatrixCase {
    pub cols: u16,
    pub rows: u16,
    pub layout: Option<LayoutAssertions>,
    #[serde(default)]
    pub screen_text: Vec<String>,
    #[serde(default)]
    pub screen_excludes: Vec<String>,
}

#[derive(Debug, Deserialize, Clone)]
pub struct DumpReference {
    pub cast: String,
    /// Read a settled ANSI screen dump instead of an asciicast-v2 stream.
    /// The terminal geometry comes from the rendered YAML frame.
    #[serde(default)]
    pub format: Option<String>,
    #[serde(default)]
    pub frame_contains: Vec<String>,
    /// Select the first frame after a marker phase has appeared. This is the
    /// YAML equivalent of the cast comparator's `--frames-after` mode.
    #[serde(default)]
    pub frame_after: Vec<String>,
    #[serde(default)]
    pub rows: Vec<DumpRowReference>,
    /// Compare every terminal cell in the selected frame, not only named rows.
    #[serde(default)]
    pub exact_screen: bool,
    /// Compare symbols, colors, and text attributes for every selected cell.
    #[serde(default)]
    pub exact_attributes: bool,
    /// Require the selected cast frame to contain truecolor cells. This keeps
    /// a terminal-default recording from being mistaken for a full-color
    /// parity oracle.
    #[serde(default)]
    pub require_truecolor: bool,
    /// Optional zero-based output-frame index. When present, this takes
    /// precedence over marker matching and makes dynamic casts phase-locked.
    #[serde(default)]
    pub frame_index: Option<usize>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct DumpCell {
    symbol: String,
    width: u8,
    fg: String,
    bg: String,
    bold: bool,
    italic: bool,
    underline: bool,
    inverse: bool,
}

fn vt_color_key(color: vt100::Color) -> String {
    match color {
        vt100::Color::Default => "default".to_owned(),
        vt100::Color::Idx(index) => format!("idx:{index}"),
        vt100::Color::Rgb(red, green, blue) => format!("rgb:{red},{green},{blue}"),
    }
}

fn ratatui_color_key(color: ratatui::style::Color) -> String {
    use ratatui::style::Color;
    match color {
        Color::Reset => "default".to_owned(),
        Color::Black => "idx:0".to_owned(),
        Color::Red => "idx:1".to_owned(),
        Color::Green => "idx:2".to_owned(),
        Color::Yellow => "idx:3".to_owned(),
        Color::Blue => "idx:4".to_owned(),
        Color::Magenta => "idx:5".to_owned(),
        Color::Cyan => "idx:6".to_owned(),
        Color::Gray => "idx:7".to_owned(),
        Color::DarkGray => "idx:8".to_owned(),
        Color::LightRed => "idx:9".to_owned(),
        Color::LightGreen => "idx:10".to_owned(),
        Color::LightYellow => "idx:11".to_owned(),
        Color::LightBlue => "idx:12".to_owned(),
        Color::LightMagenta => "idx:13".to_owned(),
        Color::LightCyan => "idx:14".to_owned(),
        Color::White => "idx:15".to_owned(),
        Color::Indexed(index) => format!("idx:{index}"),
        Color::Rgb(red, green, blue) => format!("rgb:{red},{green},{blue}"),
    }
}

fn ratatui_cell_width(buffer: &Buffer, col: u16, row: u16) -> u8 {
    let cell = buffer.cell((col, row)).expect("Runie cell");
    let symbol = cell.symbol();
    if !symbol.is_empty() {
        return UnicodeWidthStr::width(symbol).min(2) as u8;
    }
    if col > 0
        && buffer
            .cell((col - 1, row))
            .is_some_and(|previous| UnicodeWidthStr::width(previous.symbol()) == 2)
    {
        0
    } else {
        1
    }
}

fn cell_symbol_key(symbol: &str) -> String {
    if symbol.is_empty() {
        " ".to_owned()
    } else {
        symbol.to_owned()
    }
}

fn dump_cells(screen: &vt100::Screen, cols: u16, rows: u16) -> Vec<DumpCell> {
    (0..rows)
        .flat_map(|row| {
            (0..cols).map(move |col| {
                let cell = screen.cell(row, col).expect("terminal cell");
                DumpCell {
                    symbol: cell_symbol_key(&cell.contents()),
                    width: if cell.is_wide_continuation() {
                        0
                    } else if cell.is_wide() {
                        2
                    } else {
                        1
                    },
                    fg: vt_color_key(cell.fgcolor()),
                    bg: vt_color_key(cell.bgcolor()),
                    bold: cell.bold(),
                    italic: cell.italic(),
                    underline: cell.underline(),
                    inverse: cell.inverse(),
                }
            })
        })
        .collect()
}

#[derive(Debug, Deserialize, Clone)]
pub struct DumpRowReference {
    pub contains: String,
    #[serde(default)]
    pub exact: bool,
    #[serde(default)]
    pub last: bool,
}

fn default_visual_cols() -> u16 {
    120
}
fn default_visual_rows() -> u16 {
    30
}

#[derive(Debug, Deserialize)]
pub struct LineAssertion {
    pub kind: LineKindName,
    pub contains: String,
}

#[derive(Debug, Deserialize, Clone, Copy)]
#[serde(rename_all = "snake_case")]
pub enum LineKindName {
    User,
    Assistant,
    Tool,
    ToolResult,
    ToolError,
    ToolOutput,
    System,
    Activity,
    Reasoning,
    ThinkingStatus,
}

impl From<LineKindName> for LineKind {
    fn from(k: LineKindName) -> Self {
        match k {
            LineKindName::User => LineKind::User,
            LineKindName::Assistant => LineKind::Assistant,
            LineKindName::Tool => LineKind::Tool,
            LineKindName::ToolResult => LineKind::ToolResult,
            LineKindName::ToolError => LineKind::ToolError,
            LineKindName::ToolOutput => LineKind::ToolOutput,
            LineKindName::System => LineKind::System,
            LineKindName::Activity => LineKind::Activity,
            LineKindName::Reasoning => LineKind::Reasoning,
            LineKindName::ThinkingStatus => LineKind::ThinkingStatus,
        }
    }
}

/// StreamFn impl driven by a `Vec<AssistantMessageEvent>`.
pub struct ScenarioStream {
    pub events: Vec<AssistantMessageEvent>,
    /// Number of `stream()` calls so far. The first call replays `events`;
    /// later calls (auto-continue after a tool batch) return a terminating
    /// `Done{stop}` so the loop does not replay the same script forever.
    pub calls: Mutex<usize>,
    pub pending_after_first: bool,
    pub options_seen: ProviderOptionsLog,
}

type ProviderOptionsLog = Arc<Mutex<Vec<SimpleStreamOptions>>>;

#[async_trait::async_trait]
impl StreamFn for ScenarioStream {
    async fn stream(
        &self,
        _model: &Model,
        _context: &AgentContext,
        options: Option<SimpleStreamOptions>,
    ) -> Result<AssistantMessageEventStream, StreamError> {
        use futures::stream;
        self.options_seen.lock().push(options.unwrap_or_default());
        let mut n = self.calls.lock();
        *n += 1;
        if *n > 1 && self.pending_after_first {
            return Ok(Box::pin(futures::stream::pending()));
        }
        if *n > 1 {
            return Ok(Box::pin(stream::iter(vec![AssistantMessageEvent::Done {
                stop_reason: StopReason::Stop,
                usage: Usage::default(),
                message: None,
            }])));
        }
        // YAML replay consumes the complete event log after the actor settles;
        // a synchronous deterministic stream avoids test-only scheduler races
        // between parallel tool dispatch and recorder completion.
        Ok(Box::pin(stream::iter(self.events.clone())))
    }
}

#[async_trait::async_trait]
impl WebSocketAdapter for ScenarioStream {
    async fn stream_websocket(
        &self,
        model: &Model,
        context: &AgentContext,
        options: Option<SimpleStreamOptions>,
    ) -> Result<AssistantMessageEventStream, StreamError> {
        // The YAML harness deliberately reuses the deterministic event source;
        // the actor boundary is the behavior under test, while provider wire
        // framing remains the responsibility of a concrete adapter.
        self.stream(model, context, options).await
    }
}

/// Echo tool that returns its args verbatim.
pub struct EchoTool {
    parameters: Option<serde_json::Value>,
}

impl EchoTool {
    fn new(parameters: Option<serde_json::Value>) -> Self {
        Self { parameters }
    }
}
#[async_trait::async_trait]
impl AgentTool for EchoTool {
    fn name(&self) -> &str {
        "echo"
    }
    fn label(&self) -> &str {
        "Echo"
    }
    fn description(&self) -> &str {
        "Echoes args."
    }
    fn parameters(&self) -> Option<serde_json::Value> {
        self.parameters.clone()
    }
    async fn execute(
        &self,
        _id: &str,
        args: serde_json::Value,
        _signal: Option<tokio_util::sync::CancellationToken>,
        _on_update: Option<Box<dyn Fn(serde_json::Value) + Send + Sync>>,
    ) -> Result<AgentToolResult, String> {
        Ok(AgentToolResult {
            content: vec![ToolResultContent::Text {
                text: args.to_string(),
            }],
            details: serde_json::Value::Null,
            usage: None,
            added_tool_names: vec![],
            terminate: false,
        })
    }
}

/// Deterministic named tool used by strict visual replays. Its output is
/// fixed so the TestBackend frame never depends on the host filesystem.
pub struct ReplayTool {
    name: String,
    label: String,
    description: String,
    parameters: Option<serde_json::Value>,
    output: String,
    error: bool,
    details: serde_json::Value,
    usage: Option<Usage>,
    media: Option<String>,
    terminate: bool,
    added_tool_names: Vec<String>,
    execution_mode: Option<ToolExecutionMode>,
    prepared_arguments: Option<serde_json::Value>,
}

impl ReplayTool {
    fn new(name: &str, output: &str) -> Self {
        Self {
            name: name.into(),
            label: name.into(),
            description: "Deterministic visual replay tool.".into(),
            parameters: None,
            output: output.into(),
            error: false,
            details: serde_json::Value::Null,
            usage: None,
            media: None,
            terminate: false,
            added_tool_names: Vec::new(),
            execution_mode: None,
            prepared_arguments: None,
        }
    }

    fn failing(name: &str, output: &str) -> Self {
        Self {
            name: name.into(),
            label: name.into(),
            description: "Deterministic visual replay tool.".into(),
            parameters: None,
            output: output.into(),
            error: true,
            details: serde_json::Value::Null,
            usage: None,
            media: None,
            terminate: false,
            added_tool_names: Vec::new(),
            execution_mode: None,
            prepared_arguments: None,
        }
    }

    fn structured(name: &str, output: &str) -> Self {
        Self {
            name: name.into(),
            label: name.into(),
            description: "Deterministic visual replay tool.".into(),
            parameters: None,
            output: output.into(),
            error: false,
            details: serde_json::Value::Null,
            usage: None,
            media: None,
            terminate: false,
            added_tool_names: Vec::new(),
            execution_mode: None,
            prepared_arguments: None,
        }
    }

    #[allow(
        clippy::too_many_arguments,
        reason = "the deterministic YAML tool keeps each Pi result field explicit"
    )]
    fn configured(
        name: &str,
        label: Option<String>,
        description: Option<String>,
        parameters: Option<serde_json::Value>,
        output: String,
        details: serde_json::Value,
        usage: Option<Usage>,
        error: bool,
        media: Option<String>,
        terminate: bool,
        added_tool_names: Vec<String>,
        execution_mode: Option<ToolExecutionMode>,
        prepared_arguments: Option<serde_json::Value>,
    ) -> Self {
        Self {
            name: name.into(),
            label: label.unwrap_or_else(|| name.into()),
            description: description.unwrap_or_else(|| "Deterministic visual replay tool.".into()),
            parameters,
            output,
            error,
            details,
            usage,
            media,
            terminate,
            added_tool_names,
            execution_mode,
            prepared_arguments,
        }
    }
}

#[async_trait::async_trait]
impl AgentTool for ReplayTool {
    fn name(&self) -> &str {
        &self.name
    }
    fn label(&self) -> &str {
        &self.label
    }
    fn description(&self) -> &str {
        &self.description
    }
    fn parameters(&self) -> Option<serde_json::Value> {
        self.parameters.clone()
    }
    fn execution_mode(&self) -> Option<ToolExecutionMode> {
        self.execution_mode
    }
    fn prepare_arguments(&self, _args: &serde_json::Value) -> Option<serde_json::Value> {
        self.prepared_arguments.clone()
    }
    async fn execute(
        &self,
        _id: &str,
        args: serde_json::Value,
        _signal: Option<tokio_util::sync::CancellationToken>,
        on_update: Option<Box<dyn Fn(serde_json::Value) + Send + Sync>>,
    ) -> Result<AgentToolResult, String> {
        if self.error {
            return Err(self.output.clone());
        }
        let output = if self.output == "$args" {
            args.to_string()
        } else {
            self.output.clone()
        };
        if let Some(on_update) = on_update {
            on_update(serde_json::json!({"output": output}));
        }
        let mut content = if output.is_empty() {
            Vec::new()
        } else {
            vec![ToolResultContent::Text { text: output }]
        };
        if let Some(mime_type) = &self.media {
            content.push(ToolResultContent::Image {
                data: "replay-image".into(),
                mime_type: mime_type.clone(),
            });
        }
        Ok(AgentToolResult {
            content,
            details: self.details.clone(),
            usage: self.usage.clone(),
            added_tool_names: self.added_tool_names.clone(),
            terminate: self.terminate,
        })
    }
}

#[derive(Clone)]
pub struct ScenarioOutcome {
    pub events: Vec<AgentEvent>,
    pub feed: FeedSnapshot,
    pub scrollback: Vec<Line>,
    pub tool_blocks: Vec<ToolBlock>,
    pub selected_tool_id: Option<String>,
    pub selected_entry: Option<usize>,
    pub scroll_offset: usize,
    pub state: runie_core::state::AgentStateSnapshot,
    pub session: runie_core::session::SessionSnapshot,
    pub status: crate::widgets::StatusSnapshot,
    pub provider_options: Vec<SimpleStreamOptions>,
    pub steering_mode: runie_core::types::QueueMode,
    pub follow_up_mode: runie_core::types::QueueMode,
    pub loop_running: bool,
    pub abort_requested: bool,
    pub tool_execution: ToolExecutionMode,
    pub listener_events: Vec<String>,
    pub scroll_flushes: Vec<ScrollFlushObservation>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ScrollFlushObservation {
    pub kind: String,
    pub at_ms: Option<u64>,
    pub lines: i32,
    pub backlog: i32,
    pub dropped: i32,
}

struct ScenarioListener {
    events: Arc<Mutex<Vec<String>>>,
}

#[async_trait::async_trait]
impl Subscriber for ScenarioListener {
    async fn handle(&mut self, event: &AgentEvent) {
        self.events.lock().push(event_kind(event).to_owned());
    }
}

pub struct ScenarioError(pub String);

impl std::fmt::Display for ScenarioError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}

#[allow(
    clippy::too_many_lines,
    clippy::cognitive_complexity,
    reason = "scenario execution keeps actor and snapshot assembly together"
)]
pub async fn run_scenario(scenario: &Scenario) -> Result<ScenarioOutcome, ScenarioError> {
    let (bus, actor, options_seen) = build_scenario_loop(scenario)?;
    let session = SessionActor::new_with_bus(&bus);
    if let Some(jsonl) = &scenario.session_restore {
        let (_, _, restored) =
            runie_core::session::SessionSnapshot::from_jsonl(jsonl).map_err(ScenarioError)?;
        session.restore_jsonl(jsonl).await.map_err(ScenarioError)?;
        actor
            .replace_messages(
                restored
                    .entries
                    .into_iter()
                    .map(|entry| entry.message)
                    .collect(),
            )
            .await;
    }
    let listener_events = Arc::new(Mutex::new(Vec::new()));
    actor
        .subscribe(Box::new(ScenarioListener {
            events: listener_events.clone(),
        }))
        .await;
    if scenario.provider_options.model_headers.is_some()
        || scenario.provider_options.model_max_tokens.is_some()
    {
        actor
            .set_model(Model {
                headers: scenario
                    .provider_options
                    .model_headers
                    .clone()
                    .unwrap_or_default(),
                max_tokens: scenario
                    .provider_options
                    .model_max_tokens
                    .unwrap_or_default(),
                ..Model::default()
            })
            .await;
    }

    let actor_snapshot = actor.clone();
    let mut events_from_task = record_and_run_scenario(actor, bus, scenario).await;
    session.flush().await;
    let declared_events = declared_control_events(scenario);
    events_from_task.extend(declared_events.iter().cloned());
    for event in &declared_events {
        actor_snapshot.apply_event(event).await;
        let _ = session.apply_event(event).await;
    }

    let (scrollback, status) = replay_scenario_events(
        &events_from_task,
        scenario.initial_prompt.is_none(),
        scenario,
    )
    .await;
    let feed = scrollback.model_snapshot();
    let provider_options = options_seen.lock().clone();
    let listener_events = listener_events.lock().clone();
    let control = actor_snapshot.control_snapshot();
    Ok(ScenarioOutcome {
        events: events_from_task,
        scrollback: feed.lines.clone(),
        tool_blocks: feed.tool_blocks.clone(),
        selected_tool_id: feed.selected_tool_id.clone(),
        selected_entry: feed.selected_entry,
        scroll_offset: feed.scroll_offset,
        feed,
        state: actor_snapshot.state_snapshot(),
        session: session.snapshot(),
        status,
        provider_options,
        steering_mode: actor_snapshot.steering_mode().await,
        follow_up_mode: actor_snapshot.follow_up_mode().await,
        loop_running: control.running,
        abort_requested: control.abort_requested,
        tool_execution: scenario.tool_execution.unwrap_or_default(),
        listener_events,
        scroll_flushes: declared_scroll_trace(scenario),
    })
}

fn declared_control_events(scenario: &Scenario) -> Vec<AgentEvent> {
    scenario
        .events
        .iter()
        .filter_map(EventSpec::waiting_event)
        .collect()
}

#[allow(
    clippy::cognitive_complexity,
    reason = "the YAML replay keeps seed, event, and navigation reduction in one deterministic pass"
)]
async fn replay_scenario_events(
    events: &[AgentEvent],
    emit_welcome: bool,
    scenario: &Scenario,
) -> (Scrollback, crate::widgets::StatusSnapshot) {
    let scrollback_actor = crate::ScrollbackActor::new();
    let status_actor = crate::StatusActor::new();
    let mut renderer =
        EventRenderer::with_actors(scrollback_actor.clone(), status_actor.clone(), emit_welcome);
    for message in declared_tool_seeds(scenario) {
        scrollback_actor.apply(message).await;
    }
    for event in events {
        renderer.apply_actor_event(event.clone()).await;
    }
    for window in declared_context_windows(scenario) {
        status_actor
            .apply_event(&runie_core::types::AgentEvent::ModelChanged {
                model: runie_core::types::Model {
                    context_window: window,
                    ..runie_core::types::Model::default()
                },
            })
            .await;
    }
    for message in declared_navigation(scenario) {
        scrollback_actor.apply(message).await;
    }
    for message in declared_scrolls(scenario) {
        scrollback_actor.apply(message).await;
    }
    for _ in 0..declared_animation_ticks(scenario) {
        status_actor
            .apply(crate::widgets::StatusMsg::AdvanceAnimation)
            .await;
        scrollback_actor
            .apply(ScrollbackMsg::AdvanceAnimation)
            .await;
    }
    (scrollback_actor.snapshot(), status_actor.model_snapshot())
}

fn declared_animation_ticks(scenario: &Scenario) -> usize {
    scenario
        .events
        .iter()
        .filter_map(|event| match event {
            EventSpec::AnimationTicks { animation_ticks } => Some(*animation_ticks),
            _ => None,
        })
        .sum()
}

fn declared_context_windows(scenario: &Scenario) -> Vec<u64> {
    scenario
        .events
        .iter()
        .filter_map(|event| match event {
            EventSpec::ContextWindow { context_window } => Some(*context_window),
            _ => None,
        })
        .collect()
}

#[allow(
    clippy::too_many_lines,
    reason = "the declarative navigation table keeps every YAML transition explicit"
)]
fn declared_navigation(scenario: &Scenario) -> Vec<ScrollbackMsg> {
    scenario
        .events
        .iter()
        .filter_map(|event| match event {
            EventSpec::ToolFold { tool_fold } => {
                Some(ScrollbackMsg::ToggleToolMode(tool_fold.clone()))
            }
            EventSpec::ToolSelect { tool_select } if tool_select == "next" => {
                Some(ScrollbackMsg::SelectNextTool)
            }
            EventSpec::ToolSelect { tool_select } if tool_select == "previous" => {
                Some(ScrollbackMsg::SelectPreviousTool)
            }
            EventSpec::ToolSelect { tool_select } if tool_select == "entry_next" => {
                Some(ScrollbackMsg::SelectNextEntry)
            }
            EventSpec::ToolSelect { tool_select } if tool_select == "entry_previous" => {
                Some(ScrollbackMsg::SelectPreviousEntry)
            }
            EventSpec::LayoutMeasured { layout_measured } => Some(ScrollbackMsg::LayoutMeasured {
                content_rows: layout_measured.content_rows,
                viewport_rows: layout_measured.viewport_rows,
                anchor_row: layout_measured.anchor_row,
            }),
            EventSpec::SelectRange { select_range } => Some(ScrollbackMsg::SelectRange {
                anchor: select_range.anchor,
                head: select_range.head,
            }),
            EventSpec::MouseSelectionStart {
                mouse_selection_start,
            } => Some(ScrollbackMsg::MouseSelectionStart(
                runie_tui_model::CellPosition {
                    row: mouse_selection_start.row,
                    column: mouse_selection_start.column,
                },
            )),
            EventSpec::MouseSelectionExtend {
                mouse_selection_extend,
            } => Some(ScrollbackMsg::MouseSelectionExtend(
                runie_tui_model::CellPosition {
                    row: mouse_selection_extend.row,
                    column: mouse_selection_extend.column,
                },
            )),
            EventSpec::Bare(step) if step == "mouse_selection_commit" => {
                Some(ScrollbackMsg::MouseSelectionCommit)
            }
            EventSpec::Bare(step) if step == "clear_cell_selection" => {
                Some(ScrollbackMsg::ClearCellSelection)
            }
            EventSpec::Bare(step) if step == "copy_selection" => {
                Some(ScrollbackMsg::RequestCopySelection)
            }
            EventSpec::Bare(step) if step == "clear_copy_request" => {
                Some(ScrollbackMsg::ClearCopyRequest)
            }
            _ => None,
        })
        .collect()
}

#[derive(Debug, Deserialize, Clone)]
pub struct LayoutMeasuredSpec {
    pub content_rows: usize,
    pub viewport_rows: usize,
    #[serde(default)]
    pub anchor_row: Option<usize>,
}

#[derive(Debug, Deserialize, Clone)]
pub struct SelectionRangeSpec {
    pub anchor: usize,
    pub head: usize,
}

#[derive(Debug, Deserialize, Clone, Copy)]
pub struct CellPositionSpec {
    pub row: u16,
    pub column: u16,
}

#[derive(Debug, Deserialize, Clone, Copy)]
pub struct CellSelectionAssertion {
    pub anchor: CellPositionSpec,
    pub head: CellPositionSpec,
}

#[derive(Debug, Deserialize, Clone)]
pub struct ScrollInputSpec {
    pub at_ms: u64,
    pub direction: String,
    #[serde(default)]
    pub mode: Option<String>,
    #[serde(default)]
    pub speed: Option<u8>,
    #[serde(default)]
    pub inverted: Option<bool>,
}

#[derive(Debug, Deserialize, Clone)]
pub struct ScrollFlushSpec {
    pub at_ms: u64,
    pub viewport_rows: u16,
}

fn declared_tool_seeds(scenario: &Scenario) -> Vec<ScrollbackMsg> {
    scenario
        .events
        .iter()
        .filter_map(|event| match event {
            EventSpec::ToolSeed { tool_seed } => Some(ScrollbackMsg::Append(
                Line::new(
                    if tool_seed.running {
                        LineKind::ToolRunning
                    } else {
                        LineKind::Tool
                    },
                    tool_seed.header.clone(),
                )
                .for_tool(tool_seed.tool_call_id.clone()),
            )),
            _ => None,
        })
        .collect()
}

#[allow(
    clippy::too_many_lines,
    reason = "the declarative scroll event table keeps legacy and cadence replay paths together"
)]
fn declared_scrolls(scenario: &Scenario) -> Vec<ScrollbackMsg> {
    let mut normalizer = runie_tui_model::ScrollNormalizer::default();
    let mut flush_state = runie_tui_model::ScrollFlushState::new(normalizer, 24);
    let mut messages = Vec::new();
    for event in &scenario.events {
        match event {
            EventSpec::Scroll { scroll } => messages.push(ScrollbackMsg::ScrollBy(*scroll)),
            EventSpec::ScrollInput { scroll_input } => {
                normalizer = configure_scroll_normalizer(normalizer, scroll_input);
                let direction = match scroll_input.direction.as_str() {
                    "up" => runie_tui_model::ScrollDirection::Up,
                    "down" => runie_tui_model::ScrollDirection::Down,
                    _ => continue,
                };
                let flush_input = flush_state.with_normalizer(normalizer);
                let (next, delta) = normalizer.push_at(scroll_input.at_ms, direction);
                normalizer = next;
                let (next_flush, _) = flush_input.input_at(scroll_input.at_ms, direction);
                flush_state = next_flush;
                if delta != 0 {
                    messages.push(ScrollbackMsg::ScrollBy(delta));
                }
            }
            EventSpec::ScrollRawInput { scroll_raw_input } => {
                normalizer = configure_scroll_normalizer(normalizer, scroll_raw_input);
                let direction = match scroll_raw_input.direction.as_str() {
                    "up" => runie_tui_model::ScrollDirection::Up,
                    "down" => runie_tui_model::ScrollDirection::Down,
                    _ => continue,
                };
                let (next, _) = flush_state
                    .with_normalizer(normalizer)
                    .input_at(scroll_raw_input.at_ms, direction);
                flush_state = next;
                normalizer = flush_state.normalizer_for_replay();
            }
            EventSpec::ScrollFlush { scroll_flush } => {
                flush_state = flush_state.with_viewport_rows(scroll_flush.viewport_rows);
                let (next, flush) = flush_state.flush_at(scroll_flush.at_ms);
                flush_state = next;
                if flush.lines != 0 {
                    messages.push(ScrollbackMsg::ScrollBy(flush.lines));
                }
            }
            EventSpec::ScrollFinalize => {
                let (next, _) = flush_state.finalize();
                flush_state = next;
            }
            EventSpec::Bare(value) if value == "scroll_finalize" => {
                let (next, _) = flush_state.finalize();
                flush_state = next;
            }
            EventSpec::RevealLatest { reveal_latest } if *reveal_latest => {
                messages.push(ScrollbackMsg::RevealLatest)
            }
            EventSpec::FollowLatest { follow_latest } => {
                messages.push(ScrollbackMsg::SetFollowLatestUser(*follow_latest))
            }
            _ => {}
        }
    }
    messages
}

#[allow(
    clippy::too_many_lines,
    reason = "the YAML flush oracle keeps raw, cadence, and finalization records in source order"
)]
fn declared_scroll_trace(scenario: &Scenario) -> Vec<ScrollFlushObservation> {
    let mut normalizer = runie_tui_model::ScrollNormalizer::default();
    let mut state = runie_tui_model::ScrollFlushState::new(normalizer, 24);
    let mut trace = Vec::new();
    for event in &scenario.events {
        match event {
            EventSpec::ScrollRawInput { scroll_raw_input } => {
                normalizer = configure_scroll_normalizer(normalizer, scroll_raw_input);
                let direction = match scroll_raw_input.direction.as_str() {
                    "up" => runie_tui_model::ScrollDirection::Up,
                    "down" => runie_tui_model::ScrollDirection::Down,
                    _ => continue,
                };
                let (next, _) = state
                    .with_normalizer(normalizer)
                    .input_at(scroll_raw_input.at_ms, direction);
                state = next;
                normalizer = state.normalizer_for_replay();
            }
            EventSpec::ScrollFlush { scroll_flush } => {
                let (next, flush) = state
                    .with_viewport_rows(scroll_flush.viewport_rows)
                    .flush_at(scroll_flush.at_ms);
                state = next;
                trace.push(ScrollFlushObservation {
                    kind: "flush".into(),
                    at_ms: Some(scroll_flush.at_ms),
                    lines: flush.lines,
                    backlog: flush.backlog,
                    dropped: 0,
                });
            }
            EventSpec::ScrollFinalize => {
                let (next, finalized) = state.finalize();
                state = next;
                trace.push(ScrollFlushObservation {
                    kind: "finalize".into(),
                    at_ms: None,
                    lines: finalized.flushed,
                    backlog: finalized.backlog,
                    dropped: finalized.dropped,
                });
            }
            EventSpec::Bare(value) if value == "scroll_finalize" => {
                let (next, finalized) = state.finalize();
                state = next;
                trace.push(ScrollFlushObservation {
                    kind: "finalize".into(),
                    at_ms: None,
                    lines: finalized.flushed,
                    backlog: finalized.backlog,
                    dropped: finalized.dropped,
                });
            }
            _ => {}
        }
    }
    trace
}

fn configure_scroll_normalizer(
    mut normalizer: runie_tui_model::ScrollNormalizer,
    input: &ScrollInputSpec,
) -> runie_tui_model::ScrollNormalizer {
    if let Some(mode) = input.mode.as_deref() {
        normalizer = normalizer.with_mode(match mode {
            "wheel" => runie_tui_model::ScrollMode::Wheel,
            "trackpad" => runie_tui_model::ScrollMode::Trackpad,
            _ => runie_tui_model::ScrollMode::Auto,
        });
    }
    if let Some(speed) = input.speed {
        normalizer = normalizer.with_speed(speed);
    }
    if let Some(inverted) = input.inverted {
        normalizer = normalizer.with_inversion(inverted);
    }
    normalizer
}

async fn record_and_run_scenario(
    actor: LoopActor,
    bus: EventBus,
    scenario: &Scenario,
) -> Vec<AgentEvent> {
    let mut rec_rx = bus.subscribe();
    let (rec_stop_tx, mut rec_stop_rx) = tokio::sync::oneshot::channel::<()>();
    // OWNER: YAML replay recorder; joined before the scenario returns.
    let rec_handle = tokio::spawn(async move {
        let mut captured = Vec::new();
        loop {
            tokio::select! {
                biased;
                _ = &mut rec_stop_rx => break,
                result = rec_rx.recv() => {
                    match result {
                        Ok(ev) => {
                            let finished = matches!(ev, AgentEvent::AgentEnd { .. });
                            captured.push(ev);
                            if finished {
                                break;
                            }
                        }
                        Err(broadcast::error::RecvError::Lagged(_)) => continue,
                        Err(broadcast::error::RecvError::Closed) => break,
                    }
                }
            }
        }
        captured
    });

    submit_scenario(actor.clone(), scenario).await;
    actor.wait_for_idle().await;

    // Signal the recorder to stop (the bus is still held by LoopActor,
    // so it doesn't close — we use a dedicated oneshot to break the loop).
    let _ = rec_stop_tx.send(());
    let events_from_task = rec_handle.await.unwrap_or_default();

    events_from_task
}

async fn submit_scenario(actor: LoopActor, scenario: &Scenario) {
    for text in &scenario.follow_up {
        actor
            .follow_up(AgentMessage::User(UserMessage {
                content: vec![UserContent::Text { text: text.clone() }],
                timestamp: 0,
            }))
            .await;
    }
    let prompts = scenario
        .initial_prompt
        .as_ref()
        .map(|text| {
            vec![AgentMessage::User(UserMessage {
                content: vec![UserContent::Text { text: text.clone() }],
                timestamp: scenario.initial_prompt_timestamp(),
            })]
        })
        .unwrap_or_default();
    if let Err(error) = actor.prompt(prompts, scenario.agent_context()).await {
        eprintln!("[yaml_runner] prompt error: {error:?}");
    }
}

#[allow(
    clippy::too_many_lines,
    reason = "the YAML harness keeps deterministic actor wiring in one declarative path"
)]
fn build_scenario_loop(
    scenario: &Scenario,
) -> Result<(EventBus, LoopActor, ProviderOptionsLog), ScenarioError> {
    let bus = EventBus::new();
    let mut registry = ToolRegistry::new();
    for tool in &scenario.tools {
        register_scenario_tool(&mut registry, tool)?;
    }
    let options_seen = Arc::new(Mutex::new(Vec::new()));
    let scenario_stream = Arc::new(ScenarioStream {
        events: scenario
            .events
            .iter()
            .enumerate()
            .filter_map(|(index, event)| event.to_assistant_event(index))
            .collect(),
        calls: Mutex::new(0),
        // `run_scenario` waits for a terminal outcome to assert state; the
        // pending capture mode is exercised by `render_visual_buffer`, which
        // snapshots before joining the deliberately pending continuation.
        pending_after_first: false,
        options_seen: options_seen.clone(),
    });
    let provider =
        ProviderActor::new_with_websocket(scenario_stream.clone(), Some(scenario_stream));
    let deps = LoopDeps {
        state: AgentStateActor::new(),
        steering: SteeringQueueActor::new(),
        follow_up: FollowUpQueueActor::new(),
        tool_executor: ToolExecutorActor::new_with_timestamp(
            Arc::new(registry),
            scenario.tool_result_timestamp,
        ),
        provider,
        bus: bus.clone(),
        subscribers: runie_core::events::SubscriberRegistry::new(),
        hooks: ToolExecHooks::default(),
        turn_hooks: runie_core::hooks::TurnHooks::default(),
        transform_context: None,
        api_key_resolver: None,
        convert_to_llm: None,
        stream_options: scenario.provider_options.stream_options(),
        abort: None,
        tool_execution_mode: scenario.tool_execution.unwrap_or_default(),
        steering_mode: scenario.steering_mode.unwrap_or_default(),
        follow_up_mode: scenario.follow_up_mode.unwrap_or_default(),
    };
    Ok((bus, LoopActor::new(deps), options_seen))
}

#[allow(
    clippy::too_many_lines,
    reason = "the YAML tool registry keeps declarative replay variants together"
)]
fn register_scenario_tool(
    registry: &mut ToolRegistry,
    tool: &ToolSpec,
) -> Result<(), ScenarioError> {
    if tool.output.is_some()
        || tool.details.is_some()
        || tool.usage.is_some()
        || tool.error.is_some()
        || tool.media.is_some()
        || tool.terminate
        || !tool.added_tool_names.is_empty()
        || tool.execution_mode.is_some()
        || tool.prepared_arguments.is_some()
    {
        registry.register(Arc::new(ReplayTool::configured(
            &tool.name,
            tool.label.clone(),
            tool.description.clone(),
            tool.parameters.clone(),
            tool.output.clone().unwrap_or_default(),
            tool.details.clone().unwrap_or(serde_json::Value::Null),
            tool.usage.clone(),
            tool.error.is_some(),
            tool.media.clone(),
            tool.terminate,
            tool.added_tool_names.clone(),
            tool.execution_mode,
            tool.prepared_arguments.clone(),
        )));
        return Ok(());
    }
    match tool.kind.as_str() {
        "echo" => registry.register(Arc::new(EchoTool::new(tool.parameters.clone()))),
        "list_dir" => registry.register(Arc::new(ReplayTool::new(
            &tool.name,
            "Cargo.toml\nsrc\ncrates",
        ))),
        "read" => registry.register(Arc::new(ReplayTool::new(
            &tool.name,
            "# runie\n\nThis is **Runie**.",
        ))),
        "edit" => registry.register(Arc::new(ReplayTool::new(
            &tool.name,
            "@@ -1 +1 @@\n-old\n+new",
        ))),
        "bash" => registry.register(Arc::new(ReplayTool::new(
            &tool.name,
            "cargo test completed",
        ))),
        "subagent" => {
            registry.register(Arc::new(ReplayTool::new(&tool.name, "subagent completed")))
        }
        "memory_search" => registry.register(Arc::new(ReplayTool::new(
            &tool.name,
            "Found 2 memory result(s):\n\n### Result 1 (score: 0.72, source: global)\n**File:** /memory/MEMORY.md (lines 0-10)\n```\nactors\n```\n\n### Result 2 (score: 0.42, source: session)\n**File:** /memory/session.md (lines 4-7)\n```\nreplay\n```",
        ))),
        "workflow" => registry.register(Arc::new(ReplayTool::new(&tool.name, "workflow done"))),
        "web_fetch" => registry.register(Arc::new(ReplayTool::new(
            &tool.name,
            "status: 200\ncontent_type: text/html\nsize: 14.2 KB\nbody",
        ))),
        "web_search" => registry.register(Arc::new(ReplayTool::new(
            &tool.name,
            "https://docs.rs/runie\nhttps://docs.rs/ratatui\nhttps://rust-lang.org/learn",
        ))),
        "error" => registry.register(Arc::new(ReplayTool::failing(&tool.name, "tool failed"))),
        "structured_update" => registry.register(Arc::new(ReplayTool::structured(
            &tool.name,
            "first\nsecond",
        ))),
        other => return Err(ScenarioError(format!("unknown tool kind: {other}"))),
    }
    Ok(())
}

pub fn assert_scenario(outcome: &ScenarioOutcome, scenario: &Scenario) -> Result<(), String> {
    // Reuse the current tokio runtime if any (the e2e binary runs each
    // scenario inside `#[tokio::main]`); otherwise spin up a fresh one.
    match tokio::runtime::Handle::try_current() {
        Ok(handle) => handle.block_on(assert_scenario_async(outcome, scenario)),
        Err(_) => {
            let rt = tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build()
                .map_err(|e| format!("build tokio runtime: {e}"))?;
            rt.block_on(assert_scenario_async(outcome, scenario))
        }
    }
}

pub async fn assert_scenario_async(
    outcome: &ScenarioOutcome,
    scenario: &Scenario,
) -> Result<(), String> {
    assert_event_expectations(outcome, scenario)?;
    assert_state_expectations(outcome, scenario)?;
    assert_provider_options(outcome, scenario)?;
    assert_transcript_expectations(outcome, scenario)?;
    if let Some(visual) = &scenario.assertions.visual {
        assert_visual_expectations(scenario, visual).await?;
    }
    Ok(())
}

#[allow(
    clippy::too_many_lines,
    clippy::cognitive_complexity,
    reason = "provider option oracle stays declarative and grouped"
)]
fn assert_provider_options(outcome: &ScenarioOutcome, scenario: &Scenario) -> Result<(), String> {
    let Some(expected) = &scenario.assertions.provider_options else {
        return Ok(());
    };
    let actual = outcome
        .provider_options
        .first()
        .ok_or_else(|| "provider options assertion saw no provider call".to_string())?;
    if let Some(value) = &expected.session_id {
        if actual.session_id.as_ref() != Some(value) {
            return Err(format!(
                "provider session id mismatch: expected {value:?}, got {:?}",
                actual.session_id
            ));
        }
    }
    if let Some(value) = &expected.api_key {
        if actual.api_key.as_ref() != Some(value) {
            return Err(format!(
                "provider api key mismatch: expected <fixture value>, got {:?}",
                actual.api_key.as_ref().map(|_| "<present>")
            ));
        }
    }
    if let Some(value) = &expected.headers {
        if actual.headers.as_ref() != Some(value) {
            return Err(format!(
                "provider headers mismatch: expected {value:?}, got {:?}",
                actual.headers
            ));
        }
    }
    if let Some(value) = &expected.env {
        if actual.env.as_ref() != Some(value) {
            return Err(format!(
                "provider env mismatch: expected {value:?}, got {:?}",
                actual.env
            ));
        }
    }
    if let Some(value) = &expected.metadata {
        if actual.metadata.as_ref() != Some(value) {
            return Err(format!(
                "provider metadata mismatch: expected {value:?}, got {:?}",
                actual.metadata
            ));
        }
    }
    if let Some(value) = &expected.transport {
        let actual_transport = actual.transport.map(provider_transport_name);
        if actual_transport != Some(value.as_str()) {
            return Err(format!(
                "provider transport mismatch: expected {value:?}, got {actual_transport:?}"
            ));
        }
    }
    if let Some(value) = &expected.cache_retention {
        let actual_retention = actual.cache_retention.map(cache_retention_name);
        if actual_retention != Some(value.as_str()) {
            return Err(format!(
                "provider cache retention mismatch: expected {value:?}, got {actual_retention:?}"
            ));
        }
    }
    if let Some(value) = expected.websocket_connect_timeout_ms {
        if actual.websocket_connect_timeout_ms != Some(value) {
            return Err(format!(
                "provider websocket connect timeout mismatch: expected {value}, got {:?}",
                actual.websocket_connect_timeout_ms
            ));
        }
    }
    if let Some(value) = expected.timeout_ms {
        if actual.timeout_ms != Some(value) {
            return Err(format!(
                "provider timeout mismatch: expected {value}, got {:?}",
                actual.timeout_ms
            ));
        }
    }
    if let Some(value) = expected.temperature {
        if actual.temperature != Some(value) {
            return Err(format!(
                "provider temperature mismatch: expected {value}, got {:?}",
                actual.temperature
            ));
        }
    }
    if let Some(value) = expected.max_tokens {
        if actual.max_tokens != Some(value) {
            return Err(format!(
                "provider max tokens mismatch: expected {value}, got {:?}",
                actual.max_tokens
            ));
        }
    }
    if let Some(value) = expected.max_retries {
        if actual.max_retries != Some(value) {
            return Err(format!(
                "provider retries mismatch: expected {value}, got {:?}",
                actual.max_retries
            ));
        }
    }
    if let Some(value) = expected.max_retry_delay_ms {
        if actual.max_retry_delay_ms != Some(value) {
            return Err(format!(
                "provider max retry delay mismatch: expected {value}, got {:?}",
                actual.max_retry_delay_ms
            ));
        }
    }
    if let Some(value) = &expected.sampling_params {
        if actual.sampling_params.as_ref() != Some(value) {
            return Err(format!(
                "provider sampling params mismatch: expected {value:?}, got {:?}",
                actual.sampling_params
            ));
        }
    }
    if let Some(value) = &expected.thinking_budgets {
        if actual.thinking_budgets.as_ref() != Some(value) {
            return Err(format!(
                "provider thinking budgets mismatch: expected {value:?}, got {:?}",
                actual.thinking_budgets
            ));
        }
    }
    if let Some(value) = expected.reasoning {
        if actual.reasoning != Some(value) {
            return Err(format!(
                "provider reasoning mismatch: expected {value:?}, got {:?}",
                actual.reasoning
            ));
        }
    }
    if let Some(value) = &expected.deferred {
        if actual.deferred.as_ref() != Some(value) {
            return Err(format!(
                "provider deferred mode mismatch: expected {value:?}, got {:?}",
                actual.deferred
            ));
        }
    }
    Ok(())
}

#[allow(
    clippy::too_many_lines,
    clippy::cognitive_complexity,
    reason = "state assertion diagnostics stay grouped by projection field"
)]
fn assert_state_expectations(outcome: &ScenarioOutcome, scenario: &Scenario) -> Result<(), String> {
    let Some(expected) = &scenario.assertions.state else {
        return Ok(());
    };
    let actual = &outcome.state;
    if let Some(expected) = &expected.status {
        let actual_status = outcome.status.state.label();
        if actual_status != *expected {
            return Err(format!(
                "state status mismatch: expected {expected:?}, got {actual_status:?}"
            ));
        }
    }
    assert_yaml_eq!(expected.theme, outcome.status.theme, "theme");
    assert_yaml_eq!(
        expected.animation_frame,
        outcome.status.animation_frame,
        "animation_frame"
    );
    assert_yaml_eq!(
        expected.elapsed_ticks,
        outcome.status.elapsed_ticks,
        "elapsed_ticks"
    );
    assert_yaml_eq!(
        expected.animation_demand,
        outcome.status.animation_demand(),
        "animation_demand"
    );
    assert_yaml_eq!(expected.is_streaming, actual.is_streaming, "is_streaming");
    let latest_assistant = || {
        actual
            .messages
            .iter()
            .rev()
            .find_map(|message| match message {
                AgentMessage::Assistant(assistant) => Some(assistant),
                _ => None,
            })
    };
    if let Some(expected_reason) = &expected.assistant_stop_reason {
        let actual_reason = latest_assistant().and_then(|message| message.stop_reason);
        let expected_reason = StopReason::from(expected_reason);
        if actual_reason != Some(expected_reason) {
            return Err(format!(
                "assistant stop reason mismatch: expected {:?}, got {:?}",
                expected_reason, actual_reason
            ));
        }
    }
    if let Some(expected_handle) = &expected.assistant_deferred {
        let actual_handle = latest_assistant().and_then(|message| message.deferred.clone());
        if actual_handle.as_ref() != Some(expected_handle) {
            return Err(format!(
                "assistant deferred handle mismatch: expected {:?}, got {:?}",
                expected_handle, actual_handle
            ));
        }
    }
    let latest_tool_result = || {
        actual
            .messages
            .iter()
            .rev()
            .find_map(|message| match message {
                AgentMessage::ToolResult(result) => Some(result),
                _ => None,
            })
    };
    let latest_tool_call_arguments = || {
        actual
            .messages
            .iter()
            .rev()
            .find_map(|message| match message {
                AgentMessage::Assistant(assistant) => {
                    assistant
                        .content
                        .iter()
                        .rev()
                        .find_map(|content| match content {
                            AssistantContent::ToolCall(call) => Some(&call.arguments),
                            _ => None,
                        })
                }
                _ => None,
            })
    };
    if let Some(expected_names) = &expected.tool_result_added_tool_names {
        let actual_names = latest_tool_result()
            .map(|result| &result.added_tool_names)
            .cloned()
            .unwrap_or_default();
        assert_yaml_eq!(
            Some(expected_names.clone()),
            actual_names,
            "tool_result_added_tool_names"
        );
    }
    if let Some(expected_details) = &expected.tool_result_details {
        let actual_details = latest_tool_result()
            .map(|result| &result.details)
            .cloned()
            .unwrap_or(serde_json::Value::Null);
        assert_yaml_eq!(
            Some(expected_details.clone()),
            actual_details,
            "tool_result_details"
        );
    }
    if let Some(expected_usage) = &expected.tool_result_usage {
        let actual_usage = latest_tool_result().and_then(|result| result.usage.clone());
        if actual_usage.as_ref() != Some(expected_usage) {
            return Err(format!(
                "state tool_result_usage mismatch: expected {:?}, got {:?}",
                expected_usage, actual_usage
            ));
        }
    }
    if let Some(expected_error) = expected.tool_result_is_error {
        let actual_error = latest_tool_result()
            .map(|result| result.is_error)
            .unwrap_or(false);
        assert_yaml_eq!(Some(expected_error), actual_error, "tool_result_is_error");
    }
    if let Some(expected_timestamp) = expected.tool_result_timestamp {
        let actual_timestamp = latest_tool_result()
            .map(|result| result.timestamp)
            .unwrap_or_default();
        assert_yaml_eq!(
            Some(expected_timestamp),
            actual_timestamp,
            "tool_result_timestamp"
        );
    }
    if let Some(expected_labels) = &expected.tool_labels {
        let actual_labels = actual
            .tools
            .iter()
            .map(|tool| tool.label().to_owned())
            .collect::<Vec<_>>();
        assert_yaml_eq!(Some(expected_labels.clone()), actual_labels, "tool_labels");
    }
    if let Some(expected_modes) = &expected.tool_execution_modes {
        let actual_modes = actual
            .tools
            .iter()
            .map(|tool| tool.execution_mode())
            .collect::<Vec<_>>();
        assert_yaml_eq!(
            Some(expected_modes.clone()),
            actual_modes,
            "tool_execution_modes"
        );
    }
    if let Some(expected_arguments) = &expected.tool_call_arguments {
        let actual_arguments = latest_tool_call_arguments()
            .cloned()
            .unwrap_or(serde_json::Value::Null);
        assert_yaml_eq!(
            Some(expected_arguments.clone()),
            actual_arguments,
            "tool_call_arguments"
        );
    }
    assert_yaml_eq!(
        expected
            .context_window
            .map(|window| (window > 0).then_some(window)),
        outcome.status.context_window,
        "context_window"
    );
    assert_yaml_eq!(
        expected.thinking_level,
        actual.thinking_level,
        "thinking_level"
    );
    assert_yaml_eq!(
        expected.thinking_elapsed_ms.map(Some),
        outcome.status.thinking_elapsed_ms,
        "thinking_elapsed_ms"
    );
    if expected.thinking_elapsed_cleared == Some(true)
        && outcome.status.thinking_elapsed_ms.is_some()
    {
        return Err(format!(
            "state thinking_elapsed_ms mismatch: expected cleared, got {:?}",
            outcome.status.thinking_elapsed_ms
        ));
    }
    assert_yaml_eq!(
        expected.pending_tool_calls,
        actual.pending_tool_calls.len(),
        "pending_tool_calls"
    );
    assert_yaml_eq!(expected.messages, actual.messages.len(), "messages");
    assert_yaml_eq!(
        expected.session_entries,
        outcome.session.entries.len(),
        "session_entries"
    );
    if let Some(expected_stats) = &expected.session_stats {
        let stats = outcome.session.stats();
        let actual_stats = serde_json::json!({
            "messageCount": stats.message_count,
            "cachedTokens": stats.cached_tokens,
            "uncachedTokens": stats.uncached_tokens,
            "totalTokens": stats.total_tokens,
            "costTotal": stats.cost_total,
        });
        if &actual_stats != expected_stats {
            return Err(format!(
                "session stats mismatch: expected {expected_stats:?}, got {actual_stats:?}"
            ));
        }
    }
    if let Some(expected_records) = &expected.session_config_records {
        let actual_records = outcome
            .session
            .config_records
            .iter()
            .map(|entry| match &entry.record {
                runie_core::session::SessionConfigRecord::ModelChanged { .. } => {
                    "model_change".to_owned()
                }
                runie_core::session::SessionConfigRecord::ThinkingLevelChanged { .. } => {
                    "thinking_level_change".to_owned()
                }
                runie_core::session::SessionConfigRecord::ActiveToolsChanged { .. } => {
                    "active_tools_change".to_owned()
                }
                runie_core::session::SessionConfigRecord::LabelChanged { .. } => "label".to_owned(),
                runie_core::session::SessionConfigRecord::BranchSummaryCreated { .. } => {
                    "branch_summary".to_owned()
                }
                runie_core::session::SessionConfigRecord::CustomSessionEntryCreated { .. } => {
                    "custom".to_owned()
                }
                runie_core::session::SessionConfigRecord::CompactionCreated { .. } => {
                    "compaction".to_owned()
                }
                runie_core::session::SessionConfigRecord::OperationRecordCreated {
                    record_type,
                    ..
                } => record_type.clone(),
            })
            .collect::<Vec<_>>();
        if actual_records.as_slice() != expected_records.as_slice() {
            return Err(format!(
                "session config records mismatch: expected {expected_records:?}, got {actual_records:?}"
            ));
        }
    }
    if let Some(expected_records) = &expected.session_lane_records {
        let actual_records = outcome
            .session
            .lane_records
            .iter()
            .map(|record| record.record_type.clone())
            .collect::<Vec<_>>();
        if actual_records.as_slice() != expected_records.as_slice() {
            return Err(format!(
                "session lane records mismatch: expected {expected_records:?}, got {actual_records:?}"
            ));
        }
    }
    if expected.session_step_run_ids.is_some() || expected.session_step_result_entry_ids.is_some() {
        let steps = outcome
            .session
            .lane_records
            .iter()
            .filter(|record| record.record_type == "step_attempt")
            .collect::<Vec<_>>();
        if let Some(expected) = &expected.session_step_run_ids {
            let actual = steps
                .iter()
                .filter_map(|record| record.data.get("runId").and_then(serde_json::Value::as_str))
                .map(str::to_owned)
                .collect::<Vec<_>>();
            if &actual != expected {
                return Err(format!(
                    "session step run IDs mismatch: expected {expected:?}, got {actual:?}"
                ));
            }
        }
        if let Some(expected) = &expected.session_step_result_entry_ids {
            let actual = steps
                .iter()
                .filter_map(|record| {
                    record
                        .data
                        .get("resultEntryId")
                        .and_then(serde_json::Value::as_str)
                })
                .map(str::to_owned)
                .collect::<Vec<_>>();
            if &actual != expected {
                return Err(format!(
                    "session step result IDs mismatch: expected {expected:?}, got {actual:?}"
                ));
            }
        }
    }
    if let Some(expected_tools) = &expected.session_tool_started {
        let actual_tools = outcome
            .session
            .lane_records
            .iter()
            .filter(|record| record.record_type == "tool_started")
            .map(|record| record.data.clone())
            .collect::<Vec<_>>();
        if &actual_tools != expected_tools {
            return Err(format!(
                "session tool-start records mismatch: expected {expected_tools:?}, got {actual_tools:?}"
            ));
        }
    }
    if let Some(expected_ids) = &expected.compaction_context_entry_ids {
        let actual_ids = outcome
            .session
            .compaction_context_projection()
            .map(|projection| {
                projection
                    .message_indices
                    .into_iter()
                    .map(|index| outcome.session.entries[index].id.clone())
                    .collect::<Vec<_>>()
            })
            .unwrap_or_default();
        if &actual_ids != expected_ids {
            return Err(format!(
                "compaction context entries mismatch: expected {expected_ids:?}, got {actual_ids:?}"
            ));
        }
    }
    if let Some(expected_roles) = &expected.compaction_context_roles {
        let actual_roles = outcome
            .session
            .compaction_context_projection()
            .map(|projection| {
                projection
                    .messages(&outcome.session.entries)
                    .into_iter()
                    .map(|message| match message {
                        runie_core::types::AgentMessage::CompactionSummary(_) => {
                            "compactionSummary".to_owned()
                        }
                        runie_core::types::AgentMessage::User(_) => "user".to_owned(),
                        runie_core::types::AgentMessage::Assistant(_) => "assistant".to_owned(),
                        runie_core::types::AgentMessage::ToolResult(_) => "toolResult".to_owned(),
                        runie_core::types::AgentMessage::Custom(custom) => {
                            custom.0.role().to_owned()
                        }
                    })
                    .collect::<Vec<_>>()
            })
            .unwrap_or_default();
        if &actual_roles != expected_roles {
            return Err(format!(
                "compaction context roles mismatch: expected {expected_roles:?}, got {actual_roles:?}"
            ));
        }
    }
    if expected.compaction_token_estimates.is_some()
        || expected.compaction_keep_recent_tokens.is_some()
    {
        let estimates = expected
            .compaction_token_estimates
            .as_ref()
            .ok_or_else(|| "compaction token estimates are required".to_owned())?;
        let keep_recent = expected
            .compaction_keep_recent_tokens
            .ok_or_else(|| "compaction keep_recent_tokens is required".to_owned())?;
        let cut = runie_core::session::find_compaction_cut_point(
            &outcome.session.entries,
            estimates,
            0,
            outcome.session.entries.len(),
            keep_recent,
        )
        .map_err(|error| format!("compaction cut point: {error}"))?;
        assert_yaml_eq!(
            expected.compaction_first_kept_entry_index,
            cut.first_kept_entry_index,
            "compaction_first_kept_entry_index"
        );
        assert_yaml_eq!(
            expected.compaction_split_turn,
            cut.is_split_turn,
            "compaction_split_turn"
        );
        if expected.compaction_history_indices.is_some()
            || expected.compaction_turn_prefix_indices.is_some()
            || expected.compaction_retained_indices.is_some()
            || expected.compaction_tokens_before.is_some()
        {
            let preparation = runie_core::session::prepare_compaction_entries(
                &outcome.session.entries,
                estimates,
                keep_recent,
            )?
            .ok_or_else(|| "compaction preparation unexpectedly empty".to_owned())?;
            if let Some(expected) = &expected.compaction_history_indices {
                assert_eq!(
                    expected, &preparation.history_indices,
                    "compaction_history_indices"
                );
            }
            if let Some(expected) = &expected.compaction_turn_prefix_indices {
                assert_eq!(
                    expected, &preparation.turn_prefix_indices,
                    "compaction_turn_prefix_indices"
                );
            }
            if let Some(expected) = &expected.compaction_retained_indices {
                assert_eq!(
                    expected, &preparation.retained_indices,
                    "compaction_retained_indices"
                );
            }
            assert_yaml_eq!(
                expected.compaction_tokens_before,
                preparation.tokens_before,
                "compaction_tokens_before"
            );
        }
    }
    if expected.compaction_context_tokens.is_some()
        || expected.compaction_reserve_tokens.is_some()
        || expected.compaction_enabled.is_some()
        || expected.compaction_should_run.is_some()
    {
        let context_tokens = expected
            .compaction_context_tokens
            .ok_or_else(|| "compaction_context_tokens is required".to_owned())?;
        let reserve_tokens = expected
            .compaction_reserve_tokens
            .ok_or_else(|| "compaction_reserve_tokens is required".to_owned())?;
        let enabled = expected.compaction_enabled.unwrap_or(true);
        let actual = runie_core::session::should_compact(
            context_tokens,
            outcome.status.context_window.unwrap_or_default(),
            runie_core::session::CompactionSettings {
                enabled,
                reserve_tokens,
                keep_recent_tokens: 0,
            },
        );
        if let Some(expected) = expected.compaction_should_run {
            if expected != actual {
                return Err(format!(
                    "compaction_should_run mismatch: expected {expected}, got {actual}"
                ));
            }
        }
    }
    if let Some(message_texts) = &expected.context_usage_messages {
        let messages = message_texts
            .iter()
            .map(|text| {
                runie_core::types::AgentMessage::User(runie_core::types::UserMessage {
                    content: vec![runie_core::types::UserContent::Text { text: text.clone() }],
                    timestamp: 0,
                })
            })
            .collect::<Vec<_>>();
        let actual = runie_core::session::estimate_context_tokens(&messages);
        if let Some(expected) = expected.context_usage_tokens {
            if expected != actual.tokens {
                return Err(format!(
                    "context_usage_tokens mismatch: expected {expected}, got {}",
                    actual.tokens
                ));
            }
        }
        if let Some(expected) = expected.context_usage_reported_tokens {
            if expected != actual.usage_tokens {
                return Err(format!(
                    "context_usage_reported_tokens mismatch: expected {expected}, got {}",
                    actual.usage_tokens
                ));
            }
        }
        if let Some(expected) = expected.context_usage_trailing_tokens {
            if expected != actual.trailing_tokens {
                return Err(format!(
                    "context_usage_trailing_tokens mismatch: expected {expected}, got {}",
                    actual.trailing_tokens
                ));
            }
        }
        if expected.context_usage_last_index != actual.last_usage_index {
            return Err(format!(
                "context_usage_last_index mismatch: expected {:?}, got {:?}",
                expected.context_usage_last_index, actual.last_usage_index
            ));
        }
    }
    assert_yaml_eq!(
        &expected.active_operations,
        &outcome.session.active_operations,
        "active_operations"
    );
    assert_yaml_eq!(
        &expected.operation_outcomes,
        &outcome.session.operation_outcomes,
        "operation_outcomes"
    );
    assert_yaml_eq!(
        &expected.operation_kinds,
        &outcome.session.operation_kinds,
        "operation_kinds"
    );
    assert_yaml_eq!(
        &expected.session_branch_entry_ids,
        &outcome.session.branch_entry_ids(),
        "session_branch_entry_ids"
    );
    if let Some(expected_errors) = &expected.operation_errors {
        let actual_errors = outcome
            .session
            .operation_errors
            .iter()
            .map(|(id, error)| {
                (
                    id.clone(),
                    OperationErrorAssertion {
                        code: error.code.clone(),
                        message: error.message.clone(),
                    },
                )
            })
            .collect::<BTreeMap<_, _>>();
        assert_yaml_eq!(
            Some(expected_errors.clone()),
            actual_errors,
            "operation_errors"
        );
    }
    if let Some(expected_navigation) = &expected.navigation {
        let actual_navigation =
            outcome
                .session
                .navigation
                .as_ref()
                .map(|navigation| NavigationAssertion {
                    target_id: navigation.target_id.clone(),
                    summarize: navigation.summarize,
                    summary_entry_id: navigation.summary_entry_id.clone(),
                });
        assert_yaml_eq!(
            Some(expected_navigation.clone()),
            actual_navigation.unwrap_or_default(),
            "navigation"
        );
    }
    if let Some(expected_validation) = &expected.navigation_validation {
        let actual_validation = outcome
            .session
            .navigation_validation()
            .map(|validation| NavigationValidationAssertion {
                target_exists: validation.target_exists,
                summary_exists: validation.summary_exists,
            })
            .unwrap_or_default();
        assert_yaml_eq!(
            Some(expected_validation.clone()),
            actual_validation,
            "navigation_validation"
        );
    }
    if let Some(expected_terminate) = expected.session_last_terminate {
        let actual_terminate = outcome
            .session
            .entries
            .last()
            .map(|entry| entry.terminate)
            .unwrap_or(false);
        if actual_terminate != expected_terminate {
            return Err(format!(
                "session last terminate mismatch: expected {expected_terminate}, got {actual_terminate}"
            ));
        }
    }
    assert_yaml_eq!(expected.tool_count, actual.tools.len(), "tool_count");
    assert_yaml_eq!(
        expected.steering_mode,
        outcome.steering_mode,
        "steering_mode"
    );
    assert_yaml_eq!(
        expected.follow_up_mode,
        outcome.follow_up_mode,
        "follow_up_mode"
    );
    assert_yaml_eq!(expected.loop_running, outcome.loop_running, "loop_running");
    assert_yaml_eq!(
        expected.abort_requested,
        outcome.abort_requested,
        "abort_requested"
    );
    assert_yaml_eq!(
        expected.turn_started,
        outcome.feed.turn_started,
        "turn_started"
    );
    assert_yaml_eq!(
        expected.tool_execution,
        outcome.tool_execution,
        "tool_execution"
    );
    if let Some(needle) = &expected.streaming_contains {
        let text = actual
            .streaming_message
            .as_ref()
            .map(message_text)
            .unwrap_or_default();
        if !text.contains(needle) {
            return Err(format!("streaming state missing {needle:?}: {text:?}"));
        }
    }
    if let Some(needle) = &expected.error_contains {
        let error = actual.error_message.as_deref().unwrap_or_default();
        if !error.contains(needle) {
            return Err(format!("state error missing {needle:?}: {error:?}"));
        }
    }
    if let Some(needle) = &expected.system_prompt_contains {
        if !actual.system_prompt.contains(needle) {
            return Err(format!(
                "state system prompt missing {needle:?}: {:?}",
                actual.system_prompt
            ));
        }
    }
    assert_tool_block_expectations(outcome, expected)?;
    if let Some(expected) = &expected.selected_tool_id {
        if outcome.feed.selected_tool_id.as_deref() != Some(expected.as_str()) {
            return Err(format!(
                "state selected_tool_id mismatch: expected {expected:?}, got {:?}",
                outcome.feed.selected_tool_id
            ));
        }
    }
    if let Some(expected) = expected.selected_entry {
        if outcome.feed.selected_entry != Some(expected) {
            return Err(format!(
                "state selected_entry mismatch: expected {expected:?}, got {:?}",
                outcome.feed.selected_entry
            ));
        }
    }
    if let Some(expected) = expected.selected_member_index {
        if outcome.feed.selected_member_index != Some(expected) {
            return Err(format!(
                "state selected_member_index mismatch: expected {expected:?}, got {:?}",
                outcome.feed.selected_member_index
            ));
        }
    }
    assert_yaml_eq!(
        expected.selection_anchor.map(Some),
        outcome.feed.selection_anchor,
        "selection_anchor"
    );
    assert_yaml_eq!(
        expected.selection_head.map(Some),
        outcome.feed.selection_head,
        "selection_head"
    );
    if let Some(expected) = expected.cell_selection {
        let actual = outcome.feed.cell_selection.map(|selection| {
            (
                (selection.anchor.row, selection.anchor.column),
                (selection.head.row, selection.head.column),
            )
        });
        let expected = Some((
            (expected.anchor.row, expected.anchor.column),
            (expected.head.row, expected.head.column),
        ));
        assert_yaml_eq!(Some(expected), actual, "cell_selection");
    }
    if let Some(expected) = expected.copy_selection_requested {
        let actual = outcome.feed.copy_selection.is_some();
        if actual != expected {
            return Err(format!(
                "state copy_selection_requested mismatch: expected {expected:?}, got {actual:?}"
            ));
        }
    }
    if let Some(expected) = expected.scroll_offset {
        if outcome.feed.scroll_offset != expected {
            return Err(format!(
                "state scroll_offset mismatch: expected {expected}, got {}",
                outcome.feed.scroll_offset
            ));
        }
    }
    if let Some(expected) = &expected.scroll_flushes {
        let actual = outcome
            .scroll_flushes
            .iter()
            .map(|record| ScrollFlushAssertion {
                kind: record.kind.clone(),
                at_ms: record.at_ms,
                lines: record.lines,
                backlog: record.backlog,
                dropped: record.dropped,
            })
            .collect::<Vec<_>>();
        if &actual != expected {
            return Err(format!(
                "state scroll_flushes mismatch: expected {expected:?}, got {actual:?}"
            ));
        }
    }
    if let Some(expected) = expected.measured_content_rows {
        if outcome.feed.measured_content_rows != expected {
            return Err(format!(
                "state measured_content_rows mismatch: expected {expected}, got {}",
                outcome.feed.measured_content_rows
            ));
        }
    }
    if let Some(expected) = expected.measured_viewport_rows {
        if outcome.feed.measured_viewport_rows != expected {
            return Err(format!(
                "state measured_viewport_rows mismatch: expected {expected}, got {}",
                outcome.feed.measured_viewport_rows
            ));
        }
    }
    if let Some(expected) = expected.measured_anchor_row {
        if outcome.feed.measured_anchor_row != Some(expected) {
            return Err(format!(
                "state measured_anchor_row mismatch: expected {expected}, got {:?}",
                outcome.feed.measured_anchor_row
            ));
        }
    }
    if let Some(expected) = expected.autoscroll {
        if outcome.feed.autoscroll != expected {
            return Err(format!(
                "state autoscroll mismatch: expected {expected}, got {}",
                outcome.feed.autoscroll
            ));
        }
    }
    assert_feed_state_expectations(outcome, expected)?;
    assert_workflow_expectations(outcome, expected)?;
    assert_background_work_expectations(outcome, expected)?;
    Ok(())
}

fn assert_feed_state_expectations(
    outcome: &ScenarioOutcome,
    expected: &StateAssertions,
) -> Result<(), String> {
    if let Some(expected) = expected.reasoning_expanded {
        if outcome.feed.reasoning_expanded != expected {
            return Err(format!(
                "state reasoning_expanded mismatch: expected {expected}, got {}",
                outcome.feed.reasoning_expanded
            ));
        }
    }
    if let Some(expected) = expected.activity_expanded {
        if outcome.feed.activity_expanded != expected {
            return Err(format!(
                "state activity_expanded mismatch: expected {expected}, got {}",
                outcome.feed.activity_expanded
            ));
        }
    }
    if let Some(expected) = expected.follow_latest_user {
        if outcome.feed.follow_latest_user != expected {
            return Err(format!(
                "state follow_latest_user mismatch: expected {expected}, got {}",
                outcome.feed.follow_latest_user
            ));
        }
    }
    Ok(())
}

fn assert_workflow_expectations(
    outcome: &ScenarioOutcome,
    expected: &StateAssertions,
) -> Result<(), String> {
    let Some(expected) = &expected.workflows else {
        return Ok(());
    };
    if outcome.state.workflows.len() != expected.len() {
        return Err(format!(
            "state workflows mismatch: expected keys {:?}, got {:?}",
            expected.keys().collect::<Vec<_>>(),
            outcome.state.workflows.keys().collect::<Vec<_>>()
        ));
    }
    for (run_id, assertion) in expected {
        let Some(actual) = outcome.state.workflows.get(run_id) else {
            return Err(format!("state workflow missing run_id {run_id:?}"));
        };
        assert_workflow_fields(run_id, assertion, actual)?;
    }
    Ok(())
}

#[allow(
    clippy::too_many_lines,
    reason = "declarative background lifecycle diagnostics stay grouped by work ID"
)]
fn assert_background_work_expectations(
    outcome: &ScenarioOutcome,
    expected: &StateAssertions,
) -> Result<(), String> {
    let Some(expected) = &expected.background_work else {
        return Ok(());
    };
    if outcome.state.background_work.len() != expected.len() {
        return Err(format!(
            "state background_work mismatch: expected keys {:?}, got {:?}",
            expected.keys().collect::<Vec<_>>(),
            outcome.state.background_work.keys().collect::<Vec<_>>()
        ));
    }
    for (work_id, assertion) in expected {
        let Some(actual) = outcome.state.background_work.get(work_id) else {
            return Err(format!("state background work missing work_id {work_id:?}"));
        };
        assert_optional_eq(
            &assertion.description,
            &actual.description,
            "background work description",
            work_id,
        )?;
        assert_optional_option_eq(
            &assertion.activity,
            &actual.activity,
            "background work activity",
            work_id,
        )?;
        if let Some(expected) = assertion.background {
            if actual.background != expected {
                return Err(format!(
                    "state background work {work_id:?} background mismatch: expected {expected}, got {}",
                    actual.background
                ));
            }
        }
        assert_optional_eq(
            &assertion.status,
            &actual.status,
            "background work status",
            work_id,
        )?;
        if let Some(expected) = assertion.elapsed_ms {
            if actual.elapsed_ms != Some(expected) {
                return Err(format!(
                    "state background work {work_id:?} elapsed_ms mismatch: expected {expected}, got {:?}",
                    actual.elapsed_ms
                ));
            }
        }
        assert_optional_option_eq(
            &assertion.error,
            &actual.error,
            "background work error",
            work_id,
        )?;
    }
    Ok(())
}

fn assert_workflow_fields(
    run_id: &str,
    assertion: &WorkflowStateAssertion,
    actual: &runie_core::state::WorkflowSnapshot,
) -> Result<(), String> {
    assert_optional_eq(&assertion.name, &actual.name, "workflow name", run_id)?;
    assert_optional_eq(
        &assertion.objective,
        &actual.objective,
        "workflow objective",
        run_id,
    )?;
    assert_optional_option_eq(&assertion.phase, &actual.phase, "workflow phase", run_id)?;
    assert_optional_option_eq(&assertion.state, &actual.state, "workflow state", run_id)?;
    if let Some(expected) = assertion.active_agents {
        if actual.active_agents != expected {
            return Err(format!(
                "state workflow {run_id:?} active_agents mismatch: expected {expected}, got {}",
                actual.active_agents
            ));
        }
    }
    assert_optional_eq(&assertion.status, &actual.status, "workflow status", run_id)?;
    if let Some(expected) = assertion.elapsed_ms {
        if actual.elapsed_ms != Some(expected) {
            return Err(format!(
                "state workflow {run_id:?} elapsed_ms mismatch: expected {expected:?}, got {:?}",
                actual.elapsed_ms
            ));
        }
    }
    Ok(())
}

fn assert_optional_eq<T: PartialEq + std::fmt::Debug>(
    expected: &Option<T>,
    actual: &T,
    field: &str,
    run_id: &str,
) -> Result<(), String> {
    if let Some(expected) = expected {
        if expected != actual {
            return Err(format!(
                "state workflow {run_id:?} {field} mismatch: expected {expected:?}, got {actual:?}"
            ));
        }
    }
    Ok(())
}

fn assert_optional_option_eq<T: PartialEq + std::fmt::Debug>(
    expected: &Option<T>,
    actual: &Option<T>,
    field: &str,
    run_id: &str,
) -> Result<(), String> {
    if let Some(expected) = expected {
        if actual.as_ref() != Some(expected) {
            return Err(format!(
                "state workflow {run_id:?} {field} mismatch: expected {expected:?}, got {actual:?}"
            ));
        }
    }
    Ok(())
}

#[allow(
    clippy::too_many_lines,
    clippy::cognitive_complexity,
    reason = "the declarative tool-card fields are kept together for fixture diagnostics"
)]
fn assert_tool_block_expectations(
    outcome: &ScenarioOutcome,
    expected: &StateAssertions,
) -> Result<(), String> {
    if let Some(value) = expected.tool_blocks {
        assert_equal(value, actual_tool_blocks(outcome), "tool_blocks")?;
    }
    if let Some(value) = expected.tool_output_lines {
        let actual = outcome
            .feed
            .tool_blocks
            .iter()
            .map(|block| block.output.len())
            .sum::<usize>();
        assert_equal(value, actual, "tool_output_lines")?;
    }
    if let Some(value) = &expected.tool_modes {
        assert_vec_equal(
            value,
            &outcome
                .feed
                .tool_blocks
                .iter()
                .map(|block| block.mode)
                .collect::<Vec<_>>(),
            "tool_modes",
        )?;
    }
    if let Some(value) = &expected.tool_running {
        assert_vec_equal(
            value,
            &outcome
                .feed
                .tool_blocks
                .iter()
                .map(|block| block.is_running)
                .collect::<Vec<_>>(),
            "tool_running",
        )?;
    }
    if let Some(value) = &expected.tool_headers {
        assert_vec_equal(
            value,
            &outcome
                .feed
                .tool_blocks
                .iter()
                .map(|block| block.header.clone())
                .collect::<Vec<_>>(),
            "tool_headers",
        )?;
    }
    if let Some(value) = &expected.tool_header_row_ids {
        let actual = outcome
            .feed
            .lines
            .iter()
            .filter(|line| {
                matches!(
                    line.kind,
                    LineKind::Tool | LineKind::ToolRunning | LineKind::ToolError
                )
            })
            .map(|line| line.tool_row_id)
            .collect::<Vec<_>>();
        assert_vec_equal(value, &actual, "tool_header_row_ids")?;
    }
    if let Some(value) = &expected.tool_header_row_active {
        let actual = outcome
            .feed
            .lines
            .iter()
            .filter(|line| {
                matches!(
                    line.kind,
                    LineKind::Tool | LineKind::ToolRunning | LineKind::ToolError
                )
            })
            .map(|line| line.is_tool_row_active())
            .collect::<Vec<_>>();
        assert_vec_equal(value, &actual, "tool_header_row_active")?;
    }
    if let Some(value) = &expected.tool_outputs {
        assert_vec_equal(
            value,
            &outcome
                .feed
                .tool_blocks
                .iter()
                .map(|block| block.output.clone())
                .collect::<Vec<_>>(),
            "tool_outputs",
        )?;
    }
    if let Some(value) = &expected.tool_row_kinds {
        let actual = runie_tui_model::project_tool_card_rows(
            &outcome.feed.lines,
            &outcome.feed.tool_names,
            &outcome
                .feed
                .tool_blocks
                .iter()
                .map(|block| (block.tool_call_id.clone(), block.mode))
                .collect(),
        )
        .into_iter()
        .map(|row| row.row_kind)
        .collect::<Vec<_>>();
        assert_vec_equal(value, &actual, "tool_row_kinds")?;
    }
    if let Some(value) = &expected.tool_row_member_indices {
        let actual = runie_tui_model::project_tool_card_rows(
            &outcome.feed.lines,
            &outcome.feed.tool_names,
            &outcome
                .feed
                .tool_blocks
                .iter()
                .map(|block| (block.tool_call_id.clone(), block.mode))
                .collect(),
        )
        .into_iter()
        .map(|row| row.member_index)
        .collect::<Vec<_>>();
        assert_vec_equal(value, &actual, "tool_row_member_indices")?;
    }
    if let Some(value) = &expected.tool_row_paint_intents {
        let actual = runie_tui_model::project_tool_card_rows(
            &outcome.feed.lines,
            &outcome.feed.tool_names,
            &outcome
                .feed
                .tool_blocks
                .iter()
                .map(|block| (block.tool_call_id.clone(), block.mode))
                .collect(),
        )
        .into_iter()
        .map(|row| row.paint_intent())
        .collect::<Vec<_>>();
        assert_vec_equal(value, &actual, "tool_row_paint_intents")?;
    }
    if let Some(value) = &expected.tool_kinds {
        assert_vec_equal(
            value,
            &outcome
                .feed
                .tool_blocks
                .iter()
                .map(|block| block.kind)
                .collect::<Vec<_>>(),
            "tool_kinds",
        )?;
    }
    Ok(())
}

fn assert_equal(expected: usize, actual: usize, field: &str) -> Result<(), String> {
    if expected == actual {
        Ok(())
    } else {
        Err(format!(
            "state {field} mismatch: expected {expected}, got {actual}"
        ))
    }
}

fn assert_vec_equal<T: std::fmt::Debug + PartialEq>(
    expected: &[T],
    actual: &[T],
    field: &str,
) -> Result<(), String> {
    if expected == actual {
        Ok(())
    } else {
        Err(format!(
            "state {field} mismatch: expected {expected:?}, got {actual:?}"
        ))
    }
}

fn actual_tool_blocks(outcome: &ScenarioOutcome) -> usize {
    outcome.feed.tool_blocks.len()
}

fn message_text(message: &runie_core::types::AgentMessage) -> String {
    match message {
        runie_core::types::AgentMessage::Assistant(message) => message
            .content
            .iter()
            .filter_map(|block| match block {
                runie_core::types::AssistantContent::Text { text } => Some(text.as_str()),
                _ => None,
            })
            .collect(),
        _ => String::new(),
    }
}

#[allow(
    clippy::too_many_lines,
    reason = "event assertions remain one declarative YAML oracle"
)]
fn assert_event_expectations(outcome: &ScenarioOutcome, scenario: &Scenario) -> Result<(), String> {
    let lifecycle_events = outcome
        .events
        .iter()
        .filter(|event| !matches!(event, AgentEvent::OperationRecordCreated { .. }));
    let kinds = lifecycle_events.clone().map(event_kind).collect::<Vec<_>>();
    for expected in &scenario.assertions.events {
        if !kinds.contains(&expected.as_str()) {
            return Err(format!("expected event kind {expected:?} not in {kinds:?}"));
        }
    }
    if let Some(expected) = &scenario.assertions.exact_events {
        if kinds != expected.iter().map(String::as_str).collect::<Vec<_>>() {
            return Err(format!(
                "exact event sequence mismatch: expected {:?}, got {:?}",
                expected, kinds
            ));
        }
    }
    if let Some(expected) = &scenario.assertions.pi_events {
        let actual = lifecycle_events
            .map(|event| {
                runie_core::PiAgentEvent::try_from(event.clone())
                    .map_err(|event| format!("non-Pi event: {}", event_kind(&event)))
                    .and_then(|event| pi_event_wire_kind(&event))
            })
            .collect::<Result<Vec<_>, _>>()?;
        let expected = expected.iter().map(String::as_str).collect::<Vec<_>>();
        if actual != expected {
            return Err(format!(
                "exact Pi event sequence mismatch: expected {expected:?}, got {actual:?}"
            ));
        }
    }
    if let Some(expected) = &scenario.assertions.listener_events {
        if &outcome.listener_events != expected {
            return Err(format!(
                "listener event sequence mismatch: expected {expected:?}, got {:?}",
                outcome.listener_events
            ));
        }
    }
    if let Some(expected) = scenario.assertions.turn_starts {
        let actual = kinds.iter().filter(|kind| **kind == "turn_start").count();
        if actual != expected {
            return Err(format!(
                "expected {expected} turn_start events, got {actual}"
            ));
        }
    }
    Ok(())
}

fn event_kind(event: &runie_core::types::AgentEvent) -> &'static str {
    use runie_core::types::AgentEvent::*;
    match event {
        AgentStart => "agent_start",
        AgentEnd { .. } => "agent_end",
        Error { .. } => "error",
        ThinkingLevelChanged { .. } => "thinking_level_changed",
        Reset => "reset",
        TurnStart => "turn_start",
        Waiting { .. } => "waiting",
        ThemeChanged { .. } => "theme_changed",
        ModelChanged { .. } => "model_changed",
        ActiveToolsChanged { .. } => "active_tools_changed",
        SessionLabelChanged { .. } => "session_label_changed",
        BranchSummaryCreated { .. } => "branch_summary_created",
        CustomSessionEntryCreated { .. } => "custom_session_entry_created",
        CompactionCreated { .. } => "compaction_created",
        OperationRecordCreated { .. } => "operation_record_created",
        ToolDisplayModeChanged { .. } => "tool_display_mode_changed",
        TurnEnd { .. } => "turn_end",
        MessageStart { .. } => "message_start",
        MessageUpdate { .. } => "message_update",
        MessageEnd { .. } => "message_end",
        ToolExecutionStart { .. } => "tool_execution_start",
        ToolExecutionUpdate { .. } => "tool_execution_update",
        ToolExecutionEnd { .. } => "tool_execution_end",
        BackgroundWorkStarted { .. } => "background_work_started",
        BackgroundWorkProgress { .. } => "background_work_progress",
        BackgroundWorkFinished { .. } => "background_work_finished",
        BackgroundWorkCancelled { .. } => "background_work_cancelled",
        WorkflowStarted { .. } => "workflow_started",
        WorkflowProgress { .. } => "workflow_progress",
        WorkflowFinished { .. } => "workflow_finished",
    }
}

fn pi_event_wire_kind(event: &runie_core::PiAgentEvent) -> Result<String, String> {
    serde_json::to_value(event)
        .map_err(|error| format!("Pi event serialization failed: {error}"))?
        .get("type")
        .and_then(serde_json::Value::as_str)
        .map(str::to_owned)
        .ok_or_else(|| "Pi event serialization omitted its type tag".to_owned())
}

fn assert_transcript_expectations(
    outcome: &ScenarioOutcome,
    scenario: &Scenario,
) -> Result<String, String> {
    let haystack = outcome
        .scrollback
        .iter()
        .map(|line| line.text.clone())
        .collect::<Vec<_>>()
        .join("\n");
    for needle in &scenario.assertions.transcript_contains {
        if !haystack.contains(needle) {
            return Err(format!(
                "transcript missing {needle:?}; full haystack:\n{haystack}"
            ));
        }
    }
    for assertion in &scenario.assertions.scrollback_lines {
        let kind: LineKind = assertion.kind.into();
        if !outcome
            .scrollback
            .iter()
            .any(|line| line.kind == kind && line.text.contains(&assertion.contains))
        {
            return Err(format!(
                "expected {kind:?} line containing {:?}; scrollback:\n{haystack}",
                assertion.contains
            ));
        }
    }
    Ok(haystack)
}

async fn assert_visual_expectations(
    scenario: &Scenario,
    visual: &VisualAssertions,
) -> Result<(), String> {
    let buffer = render_visual_buffer(scenario, visual).await?;
    if let Some(expected) = visual.layout {
        assert_layout_expectations(visual.cols, visual.rows, expected)?;
    }
    assert_layout_matrix(scenario, visual).await?;
    let screen = buffer_to_screen(&buffer);
    for needle in &visual.screen_text {
        if !screen.contains(needle) {
            return Err(format!("screen missing {needle:?}\nscreen:\n{screen}"));
        }
    }
    for needle in &visual.screen_excludes {
        if screen.contains(needle) {
            return Err(format!(
                "screen unexpectedly contains {needle:?}\nscreen:\n{screen}"
            ));
        }
    }
    assert_cell_expectations(&buffer, &visual.cell_assertions)?;
    if let Some(reference) = &visual.reference {
        assert_dump_reference(&buffer, reference)?;
    }
    if visual.pty {
        return Err(
            "PTY assertion requested, but the YAML runner has no PTY harness; use an external "
                .to_owned()
                + "tmux/asciinema capture or disable `pty` rather than accepting a false pass",
        );
    }
    Ok(())
}

async fn assert_layout_matrix(
    scenario: &Scenario,
    visual: &VisualAssertions,
) -> Result<(), String> {
    for case in &visual.layout_matrix {
        let mut matrix_visual = visual.clone();
        matrix_visual.cols = case.cols;
        matrix_visual.rows = case.rows;
        let matrix_buffer = render_visual_buffer(scenario, &matrix_visual).await?;
        if let Some(layout) = case.layout {
            assert_layout_expectations(case.cols, case.rows, layout)?;
        }
        let matrix_screen = buffer_to_screen(&matrix_buffer);
        for needle in &case.screen_text {
            if !matrix_screen.contains(needle) {
                return Err(format!(
                    "matrix screen missing {needle:?} at {}x{}\nscreen:\n{matrix_screen}",
                    case.cols, case.rows
                ));
            }
        }
        for needle in &case.screen_excludes {
            if matrix_screen.contains(needle) {
                return Err(format!(
                    "matrix screen unexpectedly contains {needle:?} at {}x{}\nscreen:\n{matrix_screen}",
                    case.cols, case.rows
                ));
            }
        }
    }
    Ok(())
}

#[allow(
    clippy::too_many_lines,
    reason = "the YAML cell oracle reports each optional terminal attribute precisely"
)]
fn assert_cell_expectations(
    buffer: &ratatui::buffer::Buffer,
    assertions: &[CellAssertion],
) -> Result<(), String> {
    for expected in assertions {
        let Some(cell) = buffer.cell((expected.col, expected.row)) else {
            return Err(format!(
                "cell assertion is outside frame: ({}, {}) in {:?}",
                expected.col, expected.row, buffer.area
            ));
        };
        let actual_symbol = cell_symbol_key(cell.symbol());
        if let Some(symbol) = &expected.symbol {
            let expected_symbol = cell_symbol_key(symbol);
            if actual_symbol != expected_symbol {
                return Err(format!(
                    "cell ({}, {}) symbol mismatch: expected {:?}, got {:?}",
                    expected.col, expected.row, expected_symbol, actual_symbol
                ));
            }
        }
        let actual_fg = ratatui_color_key(cell.fg);
        if expected
            .fg
            .as_deref()
            .is_some_and(|value| value != actual_fg)
        {
            return Err(format!(
                "cell ({}, {}) foreground mismatch: expected {:?}, got {:?}",
                expected.col, expected.row, expected.fg, actual_fg
            ));
        }
        let actual_bg = ratatui_color_key(cell.bg);
        if expected
            .bg
            .as_deref()
            .is_some_and(|value| value != actual_bg)
        {
            return Err(format!(
                "cell ({}, {}) background mismatch: expected {:?}, got {:?}",
                expected.col, expected.row, expected.bg, actual_bg
            ));
        }
        let modifiers = [
            (
                "bold",
                expected.bold,
                cell.modifier.contains(ratatui::style::Modifier::BOLD),
            ),
            (
                "italic",
                expected.italic,
                cell.modifier.contains(ratatui::style::Modifier::ITALIC),
            ),
            (
                "underline",
                expected.underline,
                cell.modifier.contains(ratatui::style::Modifier::UNDERLINED),
            ),
            (
                "inverse",
                expected.inverse,
                cell.modifier.contains(ratatui::style::Modifier::REVERSED),
            ),
        ];
        for (name, expected_value, actual_value) in modifiers {
            if expected_value.is_some_and(|value| value != actual_value) {
                return Err(format!(
                    "cell ({}, {}) {name} mismatch: expected {:?}, got {actual_value}",
                    expected.col, expected.row, expected_value
                ));
            }
        }
    }
    Ok(())
}

fn assert_layout_expectations(
    cols: u16,
    rows: u16,
    expected: LayoutAssertions,
) -> Result<(), String> {
    let layout = crate::layout::chat_layout_with_prompt_height(
        ratatui::layout::Rect::new(0, 0, cols, rows),
        crate::layout::PROMPT_HEIGHT,
    );
    let actual = [
        ("header", layout.header, expected.header),
        ("scrollback", layout.scrollback, expected.scrollback),
        ("prompt", layout.prompt, expected.prompt),
        ("status", layout.status, expected.status),
        ("footer_badge", layout.footer_badge, expected.footer_badge),
    ];
    for (name, region, expected) in actual {
        let actual = (region.x, region.y, region.width, region.height);
        let expected = (expected.x, expected.y, expected.width, expected.height);
        if actual != expected {
            return Err(format!(
                "layout {name} mismatch: expected {expected:?}, got {actual:?}"
            ));
        }
    }
    Ok(())
}

#[allow(
    clippy::too_many_lines,
    reason = "the generic dump oracle keeps decode, frame selection, and row diagnostics together"
)]
#[allow(clippy::cognitive_complexity)]
#[allow(clippy::too_many_arguments)]
fn assert_dump_reference(buffer: &Buffer, reference: &DumpReference) -> Result<(), String> {
    let path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../artifacts")
        .join(&reference.cast);
    let raw_dump = std::fs::read_to_string(&path)
        .map_err(|error| format!("read dump {}: {error}", path.display()))?;
    let dump = if reference.format.as_deref() == Some("ansi") {
        let output = serde_json::to_string(&raw_dump)
            .map_err(|error| format!("encode ANSI dump {}: {error}", path.display()))?;
        format!(
            "{{\"version\":2,\"term\":{{\"cols\":{},\"rows\":{}}}}}\n[0.0,\"o\",{}]",
            buffer.area.width, buffer.area.height, output
        )
    } else {
        raw_dump
    };
    let mut lines = dump.lines();
    let header: serde_json::Value = serde_json::from_str(
        lines
            .next()
            .ok_or_else(|| format!("dump {} has no header", path.display()))?,
    )
    .map_err(|error| format!("parse dump header {}: {error}", path.display()))?;
    let cols = header["term"]["cols"]
        .as_u64()
        .ok_or_else(|| format!("dump {} has no terminal width", path.display()))?
        as u16;
    let rows = header["term"]["rows"]
        .as_u64()
        .ok_or_else(|| format!("dump {} has no terminal height", path.display()))?
        as u16;
    let mut parser = vt100::Parser::new(rows, cols, 0);
    let mut selected = None;
    let mut selected_cells = None;
    let mut selected_frame_index = None;
    let mut output_frame = 0usize;
    let mut after_armed = reference.frame_after.is_empty();
    for line in lines {
        let event: serde_json::Value = serde_json::from_str(line)
            .map_err(|error| format!("parse dump event {}: {error}", path.display()))?;
        if event[1].as_str() != Some("o") {
            continue;
        }
        parser.process(
            event[2]
                .as_str()
                .ok_or_else(|| format!("dump {} has invalid output event", path.display()))?
                .as_bytes(),
        );
        let contents = parser.screen().contents();
        let marker_match = reference
            .frame_contains
            .iter()
            .all(|marker| contents.contains(marker));
        let after_match = reference
            .frame_after
            .iter()
            .all(|marker| contents.contains(marker));
        let frame_selected = match reference.frame_index {
            Some(index) => output_frame == index,
            None => after_armed && marker_match,
        };
        output_frame += 1;
        if after_match {
            after_armed = true;
        }
        if frame_selected {
            selected = Some(contents);
            selected_cells = Some(dump_cells(parser.screen(), cols, rows));
            selected_frame_index = Some(output_frame - 1);
            break;
        }
    }
    let reference_screen = selected.ok_or_else(|| {
        format!(
            "dump {} has no matching frame (index {:?}, markers {:?}, after {:?})",
            path.display(),
            reference.frame_index,
            reference.frame_contains,
            reference.frame_after
        )
    })?;
    if reference.exact_attributes {
        let expected = selected_cells.as_ref().expect("selected frame cells");
        if reference.require_truecolor
            && !expected
                .iter()
                .any(|cell| cell.fg.starts_with("rgb:") || cell.bg.starts_with("rgb:"))
        {
            return Err(
                "reference frame has no RGB cells; capture with COLORTERM=truecolor".to_owned(),
            );
        }
        let expected_width = cols;
        let expected_height = rows;
        if buffer.area.width != expected_width || buffer.area.height != expected_height {
            return Err(format!(
                "full dump dimensions differ: expected {expected_width}x{expected_height}, actual {}x{}",
                buffer.area.width, buffer.area.height
            ));
        }
        let actual = (0..buffer.area.height)
            .flat_map(|row| {
                (0..buffer.area.width).map(move |col| {
                    let cell = buffer.cell((col, row)).expect("Runie cell");
                    DumpCell {
                        symbol: cell_symbol_key(cell.symbol()),
                        width: ratatui_cell_width(buffer, col, row),
                        fg: ratatui_color_key(cell.fg),
                        bg: ratatui_color_key(cell.bg),
                        bold: cell.modifier.contains(ratatui::style::Modifier::BOLD),
                        italic: cell.modifier.contains(ratatui::style::Modifier::ITALIC),
                        underline: cell.modifier.contains(ratatui::style::Modifier::UNDERLINED),
                        inverse: cell.modifier.contains(ratatui::style::Modifier::REVERSED),
                    }
                })
            })
            .collect::<Vec<_>>();
        if expected.as_slice() != actual.as_slice() {
            let width = buffer.area.width as usize;
            let mut details = Vec::new();
            for (index, (left, right)) in expected.iter().zip(&actual).enumerate() {
                if left == right {
                    continue;
                }
                let row = index / width;
                let col = index % width;
                let mut fields = Vec::new();
                if left.symbol != right.symbol {
                    fields.push(format!("symbol {:?} -> {:?}", left.symbol, right.symbol));
                }
                if left.fg != right.fg {
                    fields.push(format!("fg {} -> {}", left.fg, right.fg));
                }
                if left.bg != right.bg {
                    fields.push(format!("bg {} -> {}", left.bg, right.bg));
                }
                if left.bold != right.bold {
                    fields.push(format!("bold {} -> {}", left.bold, right.bold));
                }
                if left.italic != right.italic {
                    fields.push(format!("italic {} -> {}", left.italic, right.italic));
                }
                if left.underline != right.underline {
                    fields.push(format!(
                        "underline {} -> {}",
                        left.underline, right.underline
                    ));
                }
                if left.inverse != right.inverse {
                    fields.push(format!("inverse {} -> {}", left.inverse, right.inverse));
                }
                details.push(format!("({col},{row}): {}", fields.join(", ")));
                if details.len() == 12 {
                    break;
                }
            }
            return Err(format!(
                "full dump cell attribute mismatch ({} differing cells; first differences):\n{}",
                expected
                    .iter()
                    .zip(&actual)
                    .filter(|(left, right)| left != right)
                    .count(),
                details.join("\n")
            ));
        }
    }
    if reference.exact_screen {
        let expected = selected_cells
            .as_ref()
            .expect("selected frame cells for exact symbols");
        let actual = (0..buffer.area.height)
            .flat_map(|row| {
                (0..buffer.area.width).map(move |col| {
                    cell_symbol_key(buffer.cell((col, row)).expect("Runie cell").symbol())
                })
            })
            .collect::<Vec<_>>();
        let first_difference = expected
            .iter()
            .zip(&actual)
            .position(|(left, right)| left.symbol != *right);
        if let Some(index) = first_difference {
            let width = buffer.area.width as usize;
            let row = index / width;
            let col = index % width;
            let expected_row = expected
                .iter()
                .skip(row * width)
                .take(width)
                .map(|cell| cell.symbol.as_str())
                .collect::<String>();
            let actual_row = (0..buffer.area.width)
                .map(|column| {
                    cell_symbol_key(
                        buffer
                            .cell((column, row as u16))
                            .expect("Runie cell")
                            .symbol(),
                    )
                })
                .collect::<String>();
            let expected_context = (row.saturating_sub(2)..=(row + 2).min(rows as usize - 1))
                .map(|context_row| {
                    expected
                        .iter()
                        .skip(context_row * width)
                        .take(width)
                        .map(|cell| cell.symbol.as_str())
                        .collect::<String>()
                })
                .collect::<Vec<_>>();
            let actual_context = (row.saturating_sub(2)..=(row + 2).min(rows as usize - 1))
                .map(|context_row| {
                    (0..buffer.area.width)
                        .map(|column| {
                            cell_symbol_key(
                                buffer
                                    .cell((column, context_row as u16))
                                    .expect("Runie cell")
                                    .symbol(),
                            )
                        })
                        .collect::<String>()
                })
                .collect::<Vec<_>>();
            return Err(format!(
                "full dump symbol mismatch for {} frame {:?} at ({col},{row}): expected {:?}, actual {:?}; expected row {:?}, actual row {:?}; expected context {:?}, actual context {:?}",
                reference.cast,
                selected_frame_index,
                expected[index].symbol,
                actual[index],
                expected_row,
                actual_row,
                expected_context,
                actual_context
            ));
        }
    }
    let screen = buffer_to_screen(buffer);
    let runie_rows = screen.lines().collect::<Vec<_>>();
    for row in &reference.rows {
        let expected = if row.last {
            reference_screen
                .lines()
                .rev()
                .find(|line| line.contains(&row.contains))
        } else {
            reference_screen
                .lines()
                .find(|line| line.contains(&row.contains))
        }
        .ok_or_else(|| format!("dump row missing {:?}", row.contains))?;
        let actual = runie_rows
            .iter()
            .find(|line| line.contains(&row.contains))
            .ok_or_else(|| format!("Runie row missing {:?}", row.contains))?;
        if row.exact && expected.trim_end() != actual.trim_end() {
            return Err(format!(
                "dump row mismatch {:?}\nexpected: {:?}\nactual:   {:?}",
                row.contains,
                expected.trim_end(),
                actual.trim_end()
            ));
        }
    }
    Ok(())
}

/// Drive the TUI App via `TestBackend` and return the rendered screen text.
///
/// Mirrors grok-build's `harness.screen_contents()` contract: the result is
/// the full viewport (rows × cols) joined with `\n`, suitable for substring
/// assertions via the YAML `screen_text` / `screen_excludes` lists.
#[allow(
    clippy::cognitive_complexity,
    clippy::too_many_lines,
    reason = "keeps the deterministic YAML visual harness in one replay transaction"
)]
pub async fn render_visual_buffer(
    scenario: &Scenario,
    vis: &VisualAssertions,
) -> Result<Buffer, String> {
    use crate::app::App;
    use crate::widgets::PromptOutcome;

    // Build the same wiring as run_scenario.
    let mut activity_expanded = vis.activity_expanded.unwrap_or(true);
    let bus = runie_core::events::EventBus::new();
    let state = runie_core::state::AgentStateActor::new();
    let steering = runie_core::queues::SteeringQueueActor::new();
    let follow_up = runie_core::queues::FollowUpQueueActor::new();
    let mut reg = ToolRegistry::new();
    for t in &scenario.tools {
        register_scenario_tool(&mut reg, t).map_err(|error| error.to_string())?;
    }
    let tool_executor =
        ToolExecutorActor::new_with_timestamp(Arc::new(reg), scenario.tool_result_timestamp);
    let scenario_stream = Arc::new(ScenarioStream {
        events: scenario
            .events
            .iter()
            .enumerate()
            .filter_map(|(index, event)| event.to_assistant_event(index))
            .collect(),
        calls: Mutex::new(0),
        pending_after_first: scenario.capture_while_waiting,
        options_seen: Arc::new(Mutex::new(Vec::new())),
    });
    let provider =
        ProviderActor::new_with_websocket(scenario_stream.clone(), Some(scenario_stream));
    let deps = LoopDeps {
        state,
        steering,
        follow_up,
        tool_executor,
        provider,
        bus: bus.clone(),
        subscribers: runie_core::events::SubscriberRegistry::new(),
        hooks: ToolExecHooks::default(),
        turn_hooks: runie_core::hooks::TurnHooks::default(),
        transform_context: None,
        api_key_resolver: None,
        convert_to_llm: None,
        stream_options: scenario.provider_options.stream_options(),
        abort: None,
        tool_execution_mode: scenario.tool_execution.unwrap_or_default(),
        steering_mode: scenario.steering_mode.unwrap_or_default(),
        follow_up_mode: scenario.follow_up_mode.unwrap_or_default(),
    };
    let actor = LoopActor::new(deps);
    let app = App::new_with_welcome(actor, bus.clone());

    // Collect bus events in parallel: the recorder task subscribes once and
    // drains the bus into a `Vec` while the loop runs. This avoids a
    // runtime-scheduling race where the renderer misses events on a
    // `current_thread` runtime.
    let collected: Arc<Mutex<Vec<runie_core::types::AgentEvent>>> =
        Arc::new(Mutex::new(Vec::new()));
    let collected_clone = collected.clone();
    let rec_bus = bus.subscribe();
    let (rec_stop_tx, mut rec_stop_rx) = tokio::sync::oneshot::channel::<()>();
    let (tool_done_tx, tool_done_rx) = tokio::sync::oneshot::channel::<()>();
    // OWNER: YAML replay recorder; joined before the scenario returns.
    let rec_handle = tokio::spawn(async move {
        let mut rx = rec_bus;
        let mut tool_done_tx = Some(tool_done_tx);
        let mut tool_batch_finished = false;
        loop {
            tokio::select! {
                biased;
                _ = &mut rec_stop_rx => break,
                result = rx.recv() => {
                    match result {
                        Ok(ev) => {
                            let tool_finished = matches!(&ev, runie_core::types::AgentEvent::ToolExecutionEnd { .. });
                            let next_turn = matches!(&ev, runie_core::types::AgentEvent::TurnStart);
                            tool_batch_finished |= tool_finished;
                            collected_clone.lock().push(ev);
                            if next_turn && tool_batch_finished {
                                if let Some(tx) = tool_done_tx.take() {
                                    let _ = tx.send(());
                                }
                            }
                        },
                        Err(broadcast::error::RecvError::Lagged(_)) => continue,
                        Err(broadcast::error::RecvError::Closed) => break,
                    }
                }
            }
        }
    });

    // Pre-push follow-ups.
    for text in &scenario.follow_up {
        app.loop_actor
            .follow_up(AgentMessage::User(UserMessage {
                content: vec![UserContent::Text { text: text.clone() }],
                timestamp: 0,
            }))
            .await;
    }

    // If no prompt and no events, the loop never runs and the bus stays
    // empty. Synthesise a minimal AgentStart/AgentEnd pair so the welcome
    // modal is emitted (matches grok's idle screen).
    if scenario.events.is_empty() && scenario.initial_prompt.is_none() {
        use runie_core::types::AgentEvent;
        bus.publish(AgentEvent::AgentStart);
        tokio::task::yield_now().await;
        bus.publish(AgentEvent::AgentEnd { messages: vec![] });
        tokio::task::yield_now().await;
    }

    // Apply keystrokes.
    for step in &vis.steps {
        if step == "Ctrl+J" {
            app.scroll_scrollback_by(1).await;
            continue;
        }
        if step == "Ctrl+K" {
            app.scroll_scrollback_by(-1).await;
            continue;
        }
        if step == "Up" {
            app.select_previous_tool().await;
            continue;
        }
        if step == "Down" {
            app.select_next_tool().await;
            continue;
        }
        if step == "e" && scenario.initial_prompt.is_some() {
            activity_expanded = !activity_expanded;
            continue;
        }
        if step == "Ctrl+L" {
            app.prompt.open_file_search().await;
            app.hide_welcome().await;
            continue;
        }
        if step == "Ctrl+X" {
            app.toggle_shortcuts().await;
            app.hide_welcome().await;
            continue;
        }
        if step == "Ctrl+P" || step == "?" {
            app.toggle_command_palette().await;
            continue;
        }
        if step == "PaletteUp" {
            app.command_palette_key(crate::app::UiMsg::CommandPaletteMove(-1))
                .await;
            continue;
        }
        if step == "PaletteDown" {
            app.command_palette_key(crate::app::UiMsg::CommandPaletteMove(1))
                .await;
            continue;
        }
        if step == "PaletteEnter" {
            let mut ui_commands = app.subscribe_ui_commands();
            app.activate_command_palette().await;
            if matches!(
                ui_commands.recv().await,
                Ok(crate::app::UiCommand::ActivatePaletteEntry(
                    crate::app::PaletteAction::NewSession,
                ))
            ) {
                // Route the reset through the core actor's event boundary and
                // await its acknowledgement so YAML observes the reduced UI
                // state rather than racing the broadcast subscriber.
                let _ = app.reset_session().await;
            }
            continue;
        }
        if step == "Esc" && app.ui.snapshot().command_palette_open {
            app.command_palette_key(crate::app::UiMsg::CommandPaletteEscape)
                .await;
            continue;
        }
        if step == "Backspace" && app.ui.snapshot().command_palette_open {
            app.command_palette_key(crate::app::UiMsg::CommandPaletteBackspace)
                .await;
            continue;
        }
        if app.ui.snapshot().command_palette_open {
            for ch in step.chars() {
                app.command_palette_key(crate::app::UiMsg::CommandPaletteChar(ch))
                    .await;
            }
            continue;
        }
        if step == "Tab" {
            let _ = app
                .prompt
                .handle_key(crossterm::event::KeyEvent {
                    code: crossterm::event::KeyCode::Tab,
                    modifiers: crossterm::event::KeyModifiers::NONE,
                    kind: crossterm::event::KeyEventKind::Press,
                    state: crossterm::event::KeyEventState::NONE,
                })
                .await;
            app.hide_welcome().await;
            continue;
        }
        if step == "Shift+Tab" {
            app.prompt.cycle_mode().await;
            app.hide_welcome().await;
            continue;
        }
        let modified_enter = match step.as_str() {
            "Shift+Enter" => Some(crossterm::event::KeyModifiers::SHIFT),
            "Alt+Enter" => Some(crossterm::event::KeyModifiers::ALT),
            _ => None,
        };
        if step == "Enter" || modified_enter.is_some() {
            let outcome = app
                .prompt
                .handle_key(crossterm::event::KeyEvent {
                    code: crossterm::event::KeyCode::Enter,
                    modifiers: modified_enter.unwrap_or(crossterm::event::KeyModifiers::NONE),
                    kind: crossterm::event::KeyEventKind::Press,
                    state: crossterm::event::KeyEventState::NONE,
                })
                .await;
            if let PromptOutcome::Submitted(text) = outcome {
                let user_msg = AgentMessage::User(UserMessage {
                    content: vec![UserContent::Text { text }],
                    timestamp: scenario.initial_prompt_timestamp(),
                });
                let _ = app
                    .loop_actor
                    .prompt(vec![user_msg], scenario.agent_context())
                    .await;
            } else if matches!(outcome, PromptOutcome::Edited) {
                app.hide_welcome().await;
            }
        } else {
            for ch in step.chars() {
                let outcome = app
                    .prompt
                    .handle_key(crossterm::event::KeyEvent {
                        code: crossterm::event::KeyCode::Char(ch),
                        modifiers: crossterm::event::KeyModifiers::NONE,
                        kind: crossterm::event::KeyEventKind::Press,
                        state: crossterm::event::KeyEventState::NONE,
                    })
                    .await;
                if matches!(outcome, PromptOutcome::Edited) {
                    app.hide_welcome().await;
                }
            }
        }
    }

    let palette = app.ui.snapshot();
    if let Some(expected) = &vis.ui {
        if let Some(value) = expected.show_welcome {
            if palette.show_welcome != value {
                return Err(format!(
                    "ui.show_welcome mismatch: expected {value}, got {}",
                    palette.show_welcome
                ));
            }
        }
        if let Some(value) = expected.shortcuts_open {
            if palette.shortcuts_open != value {
                return Err(format!(
                    "ui.shortcuts_open mismatch: expected {value}, got {}",
                    palette.shortcuts_open
                ));
            }
        }
        if let Some(value) = expected.command_palette_open {
            if palette.command_palette_open != value {
                return Err(format!(
                    "ui.command_palette_open mismatch: expected {value}, got {}",
                    palette.command_palette_open
                ));
            }
        }
        if let Some(value) = &expected.command_palette_query {
            if palette.command_palette_query != *value {
                return Err(format!(
                    "ui.command_palette_query mismatch: expected {value:?}, got {:?}",
                    palette.command_palette_query
                ));
            }
        }
        if let Some(value) = expected.command_palette_index {
            if palette.command_palette_index != value {
                return Err(format!(
                    "ui.command_palette_index mismatch: expected {value}, got {}",
                    palette.command_palette_index
                ));
            }
        }
    }
    // Grok clears the idle welcome surface as soon as editing begins; the
    // synthetic idle events above must not remain in the typed frame.
    if !vis.steps.is_empty() && scenario.initial_prompt.is_none() {
        app.apply_scrollback(ScrollbackMsg::Clear).await;
    }

    // If scenario has an initial_prompt and no Enter step, submit it.
    let mut active_run = None;
    let mut captured_events = None;
    if let Some(text) = &scenario.initial_prompt {
        app.hide_welcome().await;
        if !vis.steps.iter().any(|s| s == "Enter") {
            let user_msg = AgentMessage::User(UserMessage {
                content: vec![UserContent::Text { text: text.clone() }],
                timestamp: scenario.initial_prompt_timestamp(),
            });
            if scenario.capture_while_waiting {
                let actor = app.loop_actor.clone();
                let context = scenario.agent_context();
                // OWNER: YAML visual runner; joined after the pending frame is captured.
                active_run = Some(tokio::spawn(async move {
                    actor.prompt(vec![user_msg], context).await
                }));
            } else {
                let _ = app
                    .loop_actor
                    .prompt(vec![user_msg], scenario.agent_context())
                    .await;
            }
        }
    }

    if scenario.capture_while_waiting {
        tool_done_rx
            .await
            .map_err(|_| "waiting capture ended before tool execution".to_owned())?;
        captured_events = Some(collected.lock().clone());
        app.loop_actor.abort().await;
        if let Some(run) = active_run.take() {
            let _ = run.await;
        }
    }

    for step in &vis.post_steps {
        match step.as_str() {
            "Ctrl+J" => app.scroll_scrollback_by(1).await,
            "Ctrl+K" => app.scroll_scrollback_by(-1).await,
            _ => return Err(format!("unsupported post visual step: {step}")),
        }
    }

    // Let the recorder make progress without introducing timing-dependent
    // sleeps into visual tests.
    for _ in 0..3 {
        tokio::task::yield_now().await;
    }

    // Stop the recorder task. The bus is held by LoopActor so dropping it
    // would not close the channel — we use a dedicated oneshot to break
    // the recorder's recv loop.
    let _ = rec_stop_tx.send(());
    let _ = rec_handle.await;

    // Apply every collected event SYNCHRONOUSLY to a fresh renderer. This
    // bypasses the runtime-scheduling race that prevented the
    // bus-driven renderer from seeing all events on a `current_thread`
    // runtime.
    // The live capture already projected the events once. Reuse the same
    // actor instances only after clearing them so the deterministic replay
    // remains a single event-to-state reduction rather than a second append.
    app.apply_scrollback(ScrollbackMsg::Clear).await;
    let mut renderer = EventRenderer::with_actors(
        app.scrollback_actor.clone(),
        app.status_actor.clone(),
        scenario.initial_prompt.is_none(),
    );
    app.apply_scrollback(ScrollbackMsg::SetReasoningExpanded(vis.reasoning_expanded))
        .await;
    app.apply_scrollback(ScrollbackMsg::SetActivityExpanded(activity_expanded))
        .await;
    if let Some(timestamp) = scenario.prompt_timestamp.clone() {
        app.apply_scrollback(ScrollbackMsg::SetPromptTimestamp(Some(timestamp)))
            .await;
    }
    let mut events = captured_events.unwrap_or_else(|| collected.lock().clone());
    events.extend(declared_control_events(scenario));
    for message in declared_tool_seeds(scenario) {
        app.apply_scrollback(message).await;
    }
    for ev in events.into_iter() {
        renderer.apply_actor_event(ev).await;
    }
    for _ in 0..declared_animation_ticks(scenario) {
        app.status_actor
            .apply(crate::widgets::StatusMsg::AdvanceAnimation)
            .await;
        app.apply_scrollback(ScrollbackMsg::AdvanceAnimation).await;
    }
    // The deterministic renderer applies the event directly to its projection
    // actors. Re-publish the final theme through App's shared bus as well so
    // PromptActor receives the same event boundary; otherwise prompt chrome
    // can retain GrokNight while scrollback is already TerminalNative.
    if let Some(theme) = scenario.events.iter().rev().find_map(|event| match event {
        EventSpec::Theme { theme } => Some(parse_theme(theme)),
        _ => None,
    }) {
        app.set_theme(theme).await;
    }
    // Visual fixtures are deterministic settled snapshots; keep their
    // viewport phase independent from live follow mode so wrapped responses
    // remain visible while the real app follows newly submitted prompts.
    app.apply_scrollback(ScrollbackMsg::SetFollowLatestUser(false))
        .await;
    // YAML viewport controls are explicit reducer events, not timing
    // assumptions. Apply them after transcript replay so a fixture can name
    // the exact follow/reveal phase it wants to inspect.
    for message in declared_scrolls(scenario) {
        app.apply_scrollback(message).await;
    }
    // Replay events establish the transcript first; navigation keystrokes are
    // then applied to the actor snapshot so visual assertions observe the
    // same ordering as the live application.
    for step in &vis.steps {
        match step.as_str() {
            "Up" => app.select_previous_tool().await,
            "Down" => app.select_next_tool().await,
            _ => {}
        }
    }
    for step in &vis.post_steps {
        match step.as_str() {
            "Ctrl+J" => app.scroll_scrollback_by(1).await,
            "Ctrl+K" => app.scroll_scrollback_by(-1).await,
            _ => {}
        }
    }
    if let Some(expected) = vis.center_revealed_entry {
        let actual = app
            .scrollback_actor
            .snapshot()
            .model_snapshot()
            .center_revealed_entry;
        if actual != expected {
            return Err(format!(
                "visual center_revealed_entry mismatch: expected {expected}, got {actual}"
            ));
        }
    }
    if scenario.capture_while_waiting {
        app.apply_scrollback_batch(vec![
            ScrollbackMsg::RemoveKind(crate::widgets::LineKind::ThinkingStatus),
            ScrollbackMsg::NormalizeActivitySpacing,
            ScrollbackMsg::SetPromptTimestamp(Some("9:27 PM".to_owned())),
        ])
        .await;
        app.prompt.set_placeholder_visible(false).await;
    }
    if !vis.steps.is_empty() && scenario.initial_prompt.is_none() {
        app.apply_scrollback(ScrollbackMsg::Clear).await;
    }
    let event_status = scenario.capture_while_waiting
        || (scenario.initial_prompt.is_none()
            && scenario
                .events
                .iter()
                .any(|event| matches!(event, EventSpec::Bare(kind) if kind == "start")));
    let event_phase = if scenario.capture_while_waiting
        || scenario
            .events
            .iter()
            .any(|event| matches!(event, EventSpec::TextDelta { .. }))
    {
        Some(if scenario.capture_while_waiting {
            crate::widgets::TurnStatusPhase::Waiting
        } else {
            crate::widgets::TurnStatusPhase::Thinking
        })
    } else {
        None
    };
    // Keep the declarative visual request and the actor-owned render input on
    // the same event boundary. Without this check a fixture can pass a state
    // assertion while the final buffer silently renders a stale fold mode.
    let projected_activity_expanded = app.scrollback_actor.model_snapshot().activity_expanded;
    if projected_activity_expanded != activity_expanded {
        return Err(format!(
            "visual activity_expanded delivery mismatch: expected {activity_expanded}, got {projected_activity_expanded}"
        ));
    }
    let app_projected_activity_expanded = app.model_snapshot().feed.activity_expanded;
    if app_projected_activity_expanded != activity_expanded {
        return Err(format!(
            "visual app feed activity mismatch: expected {activity_expanded}, got {app_projected_activity_expanded}"
        ));
    }
    draw_visual_frame(
        &app,
        vis,
        event_status || (!vis.steps.is_empty() && scenario.initial_prompt.is_none()),
        event_phase,
    )
}

#[allow(
    clippy::too_many_lines,
    reason = "keeps YAML visual-frame rendering in one auditable path"
)]
#[allow(clippy::cognitive_complexity)]
fn draw_visual_frame(
    app: &crate::app::App,
    vis: &VisualAssertions,
    show_turn_status: bool,
    event_phase: Option<crate::widgets::TurnStatusPhase>,
) -> Result<Buffer, String> {
    use crate::layout::chat_layout_with_prompt_height;
    use crate::widgets::WelcomeWidget;
    use ratatui::backend::TestBackend;
    use ratatui::widgets::Widget;
    use ratatui::Terminal;

    let backend = TestBackend::new(vis.cols, vis.rows);
    let mut terminal = Terminal::new(backend).map_err(|e| e.to_string())?;
    let theme = app.status_snapshot().theme();
    terminal
        .draw(|f| {
            let layout =
                chat_layout_with_prompt_height(f.area(), app.prompt.snapshot().render_height());
            let prompt_area = if vis.waiting_chrome.is_some() {
                ratatui::layout::Rect {
                    // Grok keeps the prompt box at the normal prompt origin;
                    // the waiting rows are overlaid above it.
                    y: layout.prompt.y,
                    ..layout.prompt
                }
            } else {
                layout.prompt
            };
            if app.ui.snapshot().show_welcome && event_phase.is_none() {
                WelcomeWidget.render_with_theme(layout.scrollback, f.buffer_mut(), theme);
                if vis.cols >= 100 {
                    WelcomeWidget::render_hero_footer_badge(layout.footer_badge, f.buffer_mut());
                }
            } else {
                let scrollback = app.scrollback_snapshot();
                scrollback.render_with_terminal_height(layout.scrollback, vis.rows, f.buffer_mut());
            }
            if show_turn_status {
                let projected =
                    (!matches!(event_phase, Some(crate::widgets::TurnStatusPhase::Waiting)))
                        .then(|| app.status_snapshot().turn_status())
                        .flatten();
                let fallback = event_phase
                    .map(|phase| {
                        crate::widgets::TurnStatus::new(
                            if phase == crate::widgets::TurnStatusPhase::Waiting {
                                21
                            } else {
                                0
                            },
                        )
                        .phase(phase)
                        .with_chrome(vis.waiting_chrome.as_deref().unwrap_or(" 0.0s ⇣0 [stop]"))
                    })
                    .or_else(|| show_turn_status.then(|| crate::widgets::TurnStatus::new(0)));
                if let Some(status) = projected.or(fallback) {
                    status.render(
                        ratatui::layout::Rect {
                            x: layout.scrollback.x,
                            y: layout
                                .prompt
                                .y
                                .saturating_sub(if vis.waiting_chrome.is_some() { 4 } else { 2 }),
                            width: layout.scrollback.width,
                            height: 1,
                        },
                        f.buffer_mut(),
                    );
                }
            }
            if vis.waiting_chrome.is_some() {
                ratatui::widgets::Paragraph::new(doctor_line()).render(
                    ratatui::layout::Rect {
                        x: layout.scrollback.x,
                        y: prompt_area.y.saturating_sub(2),
                        width: layout.scrollback.width,
                        height: 1,
                    },
                    f.buffer_mut(),
                );
            }
            Widget::render(app.prompt.snapshot(), prompt_area, f.buffer_mut());
            if vis.waiting_chrome.is_some() {
                ratatui::widgets::Paragraph::new(doctor_line()).render(
                    ratatui::layout::Rect {
                        x: layout.scrollback.x,
                        y: prompt_area.y.saturating_sub(2),
                        width: layout.scrollback.width,
                        height: 1,
                    },
                    f.buffer_mut(),
                );
            }
            app.status_snapshot().render(layout.status, f.buffer_mut());
            Widget::render(
                ratatui::widgets::Paragraph::new(" main ~/Code/GitHub/runie-tests/runie"),
                layout.header,
                f.buffer_mut(),
            );
            if let Some(meter) = &vis.header_meter {
                let x = layout.header.right().saturating_sub(meter.len() as u16);
                f.buffer_mut().set_string(
                    x,
                    layout.header.y,
                    meter,
                    ratatui::style::Style::default(),
                );
            }
            let palette = app.ui.snapshot();
            if palette.command_palette_open {
                crate::widgets::CommandPaletteWidget::new(
                    palette.command_palette_query,
                    palette.command_palette_index,
                )
                .render(f.area(), f.buffer_mut());
            }
            if palette.shortcuts_open {
                crate::widgets::shortcuts::render(f.area(), f.buffer_mut(), theme);
            }
            f.set_cursor_position(app.prompt.snapshot().cursor_position(prompt_area));
        })
        .map_err(|e| e.to_string())?;
    Ok(terminal.backend().buffer().clone())
}

fn doctor_line<'a>() -> ratatui::text::Line<'a> {
    ratatui::text::Line::from(vec![
        ratatui::text::Span::raw("Run "),
        ratatui::text::Span::styled(
            "/doctor",
            ratatui::style::Style::default().add_modifier(ratatui::style::Modifier::BOLD),
        ),
        ratatui::text::Span::raw(" for details and fixes."),
    ])
}

pub async fn render_visual(scenario: &Scenario, vis: &VisualAssertions) -> Result<String, String> {
    let buf = render_visual_buffer(scenario, vis).await?;
    Ok(buffer_to_screen(&buf))
}

fn buffer_to_screen(buf: &Buffer) -> String {
    let mut out = String::with_capacity((buf.area.width as usize + 1) * (buf.area.height as usize));
    for y in 0..buf.area.height {
        for x in 0..buf.area.width {
            if let Some(c) = buf.cell((x, y)) {
                out.push_str(c.symbol());
            }
        }
        out.push('\n');
    }
    out
}

pub fn load_scenario(path: &Path) -> Result<Scenario, String> {
    let text = std::fs::read_to_string(path).map_err(|e| e.to_string())?;
    serde_yaml::from_str(&text).map_err(|e| e.to_string())
}

#[cfg(test)]
mod tests {
    use super::{assert_cell_expectations, CellAssertion};
    use ratatui::{
        buffer::Buffer,
        layout::Rect,
        style::{Color, Modifier, Style},
    };

    #[test]
    fn yaml_cell_oracle_checks_glyph_palette_and_all_modifiers() {
        let mut buffer = Buffer::empty(Rect::new(0, 0, 3, 1));
        buffer.set_string(
            0,
            0,
            "X",
            Style::default()
                .fg(Color::Rgb(1, 2, 3))
                .bg(Color::Rgb(4, 5, 6))
                .add_modifier(
                    Modifier::BOLD | Modifier::ITALIC | Modifier::UNDERLINED | Modifier::REVERSED,
                ),
        );
        let assertions = vec![
            CellAssertion {
                col: 0,
                row: 0,
                symbol: Some("X".into()),
                fg: Some("rgb:1,2,3".into()),
                bg: Some("rgb:4,5,6".into()),
                bold: Some(true),
                italic: Some(true),
                underline: Some(true),
                inverse: Some(true),
            },
            CellAssertion {
                col: 1,
                row: 0,
                symbol: Some(" ".into()),
                bg: Some("default".into()),
                ..CellAssertion::default()
            },
        ];
        assert_cell_expectations(&buffer, &assertions).expect("cell assertions pass");
    }
}
