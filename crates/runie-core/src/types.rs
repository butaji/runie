//! Core types ported from `@earendil-works/pi-agent-core`.
//!
//! Pinned to pi-agent-core commit: see the project README for the tracked
//! upstream version this port mirrors.

use serde::{Deserialize, Serialize};

mod assistant_message_wire {
    use super::AssistantMessage;
    use serde::{Deserialize, Deserializer, Serialize, Serializer};

    pub fn serialize<S>(value: &Option<AssistantMessage>, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        value.clone().unwrap_or_default().serialize(serializer)
    }

    pub fn deserialize<'de, D>(deserializer: D) -> Result<Option<AssistantMessage>, D::Error>
    where
        D: Deserializer<'de>,
    {
        AssistantMessage::deserialize(deserializer).map(Some)
    }
}

#[path = "types_provider.rs"]
mod types_provider;
pub use types_provider::*;
#[path = "types_events.rs"]
mod types_events;
pub use types_events::*;
#[path = "types_stop_reason.rs"]
mod types_stop_reason;
/// Reasoning level requested for the next turn. Some providers only support a
/// subset; consult the model's metadata before using `XHigh` / `Max`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum ThinkingLevel {
    #[default]
    Off,
    Minimal,
    Low,
    Medium,
    High,
    XHigh,
    Max,
}

/// Tool dispatch mode for a single batch of tool calls from one assistant
/// message.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum ToolExecutionMode {
    Sequential,
    #[default]
    Parallel,
}

/// How many queued user messages to drain at a queue-drain point.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Default)]
#[serde(rename_all = "kebab-case")]
pub enum QueueMode {
    #[default]
    OneAtATime,
    All,
}

/// Why an assistant message finished generating.
///
/// `Pending` mirrors pi's initial streaming partial (`stopReason: "pending"`,
/// `pi/packages/agent/src/proxy.ts:124`): it marks an in-progress assistant
/// message and is replaced by a final reason when the stream ends. It is
/// never a terminal stop reason.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum StopReason {
    #[serde(rename = "stop")]
    Stop,
    #[serde(rename = "toolUse")]
    ToolUse,
    #[serde(rename = "length")]
    MaxTokens,
    #[serde(rename = "error")]
    Error,
    #[serde(rename = "aborted")]
    Aborted,
    #[serde(rename = "pending")]
    Pending,
    #[serde(rename = "deferred")]
    Deferred,
}
#[cfg(test)]
mod stop_reason_tests {
    use super::{AssistantMessage, DeferredHandle, StopReason};

    #[test]
    fn serializes_pi_wire_values() {
        let cases = [
            (StopReason::Stop, "stop"),
            (StopReason::ToolUse, "toolUse"),
            (StopReason::MaxTokens, "length"),
            (StopReason::Error, "error"),
            (StopReason::Aborted, "aborted"),
            (StopReason::Pending, "pending"),
            (StopReason::Deferred, "deferred"),
        ];

        for (reason, wire) in cases {
            let encoded = serde_json::to_string(&reason).expect("stop reason serializes");
            assert_eq!(encoded, format!("\"{wire}\""));
            assert_eq!(
                serde_json::from_str::<StopReason>(&encoded).unwrap(),
                reason
            );
        }
    }

    #[test]
    fn deferred_handle_round_trips_pi_fields() {
        let message = AssistantMessage {
            stop_reason: Some(StopReason::Deferred),
            deferred: Some(DeferredHandle {
                provider: "replay".into(),
                model_id: "model-1".into(),
                api: "replay-api".into(),
                id: "deferred-1".into(),
                expires_at: Some(42),
                poll_after_ms: Some(250),
                data: Some(serde_json::json!({"batch": "deferred-1"})),
            }),
            ..AssistantMessage::default()
        };
        let value = serde_json::to_value(&message).expect("deferred message serializes");
        assert_eq!(value["stopReason"], "deferred");
        assert_eq!(value["deferred"]["modelId"], "model-1");
        assert_eq!(value["deferred"]["pollAfterMs"], 250);
        assert!(value["deferred"].get("model_id").is_none());
        let round_trip: AssistantMessage = serde_json::from_value(value).expect("round trip");
        assert_eq!(round_trip, message);
    }
}

#[path = "types_messages.rs"]
mod types_messages;
pub use types_messages::*;
#[path = "types_media.rs"]
mod types_media;
pub use types_media::*;
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Model {
    pub id: String,
    pub name: String,
    pub api: String,
    pub provider: String,
    #[serde(default)]
    pub base_url: String,
    #[serde(default)]
    pub reasoning: bool,
    #[serde(default)]
    pub thinking_level_map: Option<ThinkingLevelMap>,
    /// Accepted input modalities (pi: `input`).
    #[serde(default)]
    pub input: Vec<InputKind>,
    /// Cost in USD per million tokens (pi: `cost`).
    #[serde(default)]
    pub cost: ModelCost,
    #[serde(default)]
    pub context_window: u64,
    #[serde(default)]
    pub max_tokens: u64,
    /// Default sampling parameters merged into provider requests (pi:
    /// `samplingParams`). Values remain provider-neutral JSON so adapters can
    /// preserve the configured shape exactly.
    #[serde(default)]
    pub sampling_params: Option<std::collections::HashMap<String, serde_json::Value>>,
    /// Extra HTTP headers for provider requests (pi: `headers?`).
    #[serde(default)]
    pub headers: std::collections::HashMap<String, String>,
    /// Provider-specific compatibility overrides (pi `compat?`).
    #[serde(default)]
    pub compat: Option<serde_json::Value>,
}

#[path = "media.rs"]
mod media;
pub use media::{encode_user_content, encode_user_contents, MediaWireFormat};
#[path = "model_capabilities.rs"]
mod model_capabilities;
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(
    tag = "type",
    rename_all = "snake_case",
    rename_all_fields = "camelCase"
)]
#[allow(
    clippy::large_enum_variant,
    reason = "AgentEvent is the shared typed bus boundary; boxing would obscure the event DSL"
)]
pub enum AgentEvent {
    AgentStart,
    AgentEnd {
        messages: Vec<AgentMessage>,
    },
    Error {
        message: String,
    },
    ThinkingLevelChanged {
        level: ThinkingLevel,
    },
    Reset,
    TurnStart,
    Waiting {
        reason: WaitingReason,
    },
    ThemeChanged {
        theme: ThemeKind,
    },
    /// Runie application event for propagating the selected model to UI
    /// projections. It is intentionally outside the closed Pi wire contract.
    ModelChanged {
        model: Model,
    },
    /// Session-journal configuration event. This is application-owned and is
    /// intentionally outside the closed Pi agent event boundary.
    ActiveToolsChanged {
        tool_names: Vec<String>,
    },
    /// Pi-compatible session label fact, applied by the session actor.
    SessionLabelChanged {
        target_id: String,
        label: Option<String>,
    },
    /// Pi-compatible session name fact, applied by the session actor.
    SessionNameChanged {
        name: String,
    },
    /// Pi session-tree lane mutation, separate from operation-lane records.
    SessionLaneChanged {
        lane: String,
        leaf_id: Option<String>,
        create: bool,
    },
    /// Append a session message to a named Pi lane through SessionActor.
    SessionEntryAppended {
        lane: String,
        message: AgentMessage,
    },
    /// Application-owned session branch summary; navigation identity is kept
    /// in the event so the journal cannot reduce an anonymous summary.
    BranchSummaryCreated {
        from_id: String,
        summary: String,
        details: Option<serde_json::Value>,
    },
    /// Extension-owned session journal payload. It is preserved as data and
    /// never interpreted by core or the TUI.
    CustomSessionEntryCreated {
        custom_type: String,
        data: Option<serde_json::Value>,
    },
    /// Persist a Pi compaction result; the compaction algorithm remains
    /// agent-owned while this event preserves its journal payload.
    CompactionCreated {
        summary: String,
        retained_tail: Vec<AgentMessage>,
        tokens_before: u64,
        details: Option<serde_json::Value>,
        usage: Option<Usage>,
    },
    /// Lossless Pi harness operation-lane record. Admission and lifecycle
    /// policy remain owned by the loop/harness; the session actor stores data.
    OperationRecordCreated {
        record_type: String,
        data: serde_json::Value,
    },
    /// Typed operation-lane fact for live producers. Generic records remain
    /// accepted at the replay and persistence compatibility edges.
    TypedOperationRecordCreated {
        kind: OperationRecordKind,
        data: serde_json::Value,
    },
    ToolDisplayModeChanged {
        tool_call_id: String,
        mode: ToolDisplayMode,
    },
    TurnEnd {
        message: AgentMessage,
        tool_results: Vec<ToolResultMessage>,
    },
    MessageStart {
        message: AgentMessage,
    },
    MessageUpdate {
        message: AgentMessage,
        #[serde(rename = "assistantMessageEvent")]
        event: AssistantMessageEvent,
    },
    MessageEnd {
        message: AgentMessage,
    },
    ToolExecutionStart {
        tool_call_id: String,
        tool_name: String,
        args: serde_json::Value,
    },
    ToolExecutionUpdate {
        tool_call_id: String,
        tool_name: String,
        args: serde_json::Value,
        partial_result: serde_json::Value,
    },
    ToolExecutionEnd {
        tool_call_id: String,
        tool_name: String,
        result: serde_json::Value,
        is_error: bool,
    },
    BackgroundWorkStarted {
        work_id: String,
        description: String,
        background: bool,
    },
    BackgroundWorkProgress {
        work_id: String,
        description: String,
        activity: String,
    },
    BackgroundWorkFinished {
        work_id: String,
        description: String,
        is_error: bool,
        #[serde(default)]
        elapsed_ms: Option<u64>,
        #[serde(default)]
        error: Option<String>,
    },
    BackgroundWorkCancelled {
        work_id: String,
        description: String,
        #[serde(default)]
        elapsed_ms: Option<u64>,
    },
    WorkflowStarted {
        run_id: String,
        name: String,
        objective: String,
    },
    WorkflowProgress {
        run_id: String,
        phase: String,
        state: String,
        active_agents: u32,
    },
    WorkflowFinished {
        run_id: String,
        status: String,
        #[serde(default)]
        elapsed_ms: Option<u64>,
    },
}

/// Per-event payload from a streaming assistant message.
///
/// Mirrors pi's granular `AssistantMessageEvent` (pi-ai types.ts:501): the
/// sectional `*_start` / `*_end` markers delimit content blocks, the `*_delta`
/// events carry the streaming text, and `done`/`error` terminate the message.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(
    tag = "type",
    rename_all = "snake_case",
    rename_all_fields = "camelCase"
)]
pub enum AssistantMessageEvent {
    Start {
        partial: AssistantMessage,
    },
    TextStart {
        #[serde(rename = "contentIndex")]
        index: usize,
        partial: AssistantMessage,
    },
    TextDelta {
        #[serde(rename = "contentIndex")]
        index: usize,
        delta: String,
        partial: AssistantMessage,
    },
    TextEnd {
        #[serde(rename = "contentIndex")]
        index: usize,
        content: String,
        partial: AssistantMessage,
    },
    ThinkingStart {
        #[serde(rename = "contentIndex")]
        index: usize,
        partial: AssistantMessage,
    },
    ThinkingDelta {
        #[serde(rename = "contentIndex")]
        index: usize,
        delta: String,
        partial: AssistantMessage,
    },
    ThinkingEnd {
        #[serde(rename = "contentIndex")]
        index: usize,
        content: String,
        /// Server-derived thinking duration, when the provider supplies it.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        elapsed_ms: Option<u64>,
        partial: AssistantMessage,
    },
    #[serde(rename = "toolcall_start")]
    ToolCallStart {
        #[serde(rename = "contentIndex")]
        index: usize,
        partial: AssistantMessage,
    },
    #[serde(rename = "toolcall_delta")]
    ToolCallDelta {
        #[serde(rename = "contentIndex")]
        index: usize,
        /// Raw argument/name delta emitted by pi-ai before partial reduction.
        #[serde(default)]
        delta: String,
        partial: AssistantMessage,
    },
    #[serde(rename = "toolcall_end")]
    ToolCallEnd {
        #[serde(rename = "contentIndex")]
        index: usize,
        tool_call: ToolCall,
        partial: AssistantMessage,
    },
    Done {
        #[serde(rename = "reason")]
        stop_reason: StopReason,
        usage: Usage,
        /// Full terminal assistant payload when the provider supplies it.
        /// Internal synthetic streams may leave this absent.
        #[serde(default, with = "assistant_message_wire")]
        message: Option<AssistantMessage>,
    },
    Error {
        #[serde(rename = "reason")]
        reason: StopReason,
        /// Pi's error event carries the terminal assistant message.
        error: AssistantMessage,
    },
}

/// Returned by `before_tool_call`. `{ block: true }` short-circuits to a
/// synthetic error tool result.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BeforeToolCallResult {
    pub block: bool,
    pub reason: Option<String>,
}

/// Returned by `after_tool_call`. Field-by-field override: any `Some` field
/// replaces the corresponding field on the executed result.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AfterToolCallResult {
    pub content: Option<Vec<ToolResultContent>>,
    pub details: Option<serde_json::Value>,
    pub is_error: Option<bool>,
    pub usage: Option<Usage>,
    pub terminate: Option<bool>,
}

#[derive(Clone)]
pub struct BeforeToolCallContext {
    pub assistant_message: AssistantMessage,
    pub tool_call: ToolCall,
    pub args: serde_json::Value,
    pub context: AgentContext,
    pub signal: tokio_util::sync::CancellationToken,
}

#[derive(Clone)]
pub struct AfterToolCallContext {
    pub assistant_message: AssistantMessage,
    pub tool_call: ToolCall,
    pub args: serde_json::Value,
    pub result: AgentToolResult,
    pub is_error: bool,
    pub context: AgentContext,
    pub signal: tokio_util::sync::CancellationToken,
}

#[cfg(test)]
#[path = "types_tests.rs"]
mod tests;
#[cfg(test)]
#[path = "types_tests_extra.rs"]
mod tests_extra;
