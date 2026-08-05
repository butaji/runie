//! Core types ported from `@earendil-works/pi-agent-core`.
//!
//! Pinned to pi-agent-core commit: see the project README for the tracked
//! upstream version this port mirrors.

use serde::{Deserialize, Serialize};

/// Reasoning level requested for the next turn. Some providers only support a
/// subset; consult the model's metadata before using `XHigh` / `Max`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ThinkingLevel {
    Off,
    Minimal,
    Low,
    Medium,
    High,
    XHigh,
    Max,
}

impl Default for ThinkingLevel {
    fn default() -> Self {
        Self::Off
    }
}

/// Tool dispatch mode for a single batch of tool calls from one assistant
/// message.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ToolExecutionMode {
    Sequential,
    Parallel,
}

impl Default for ToolExecutionMode {
    fn default() -> Self {
        Self::Parallel
    }
}

/// How many queued user messages to drain at a queue-drain point.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum QueueMode {
    OneAtATime,
    All,
}

impl Default for QueueMode {
    fn default() -> Self {
        Self::OneAtATime
    }
}

/// Why an assistant message finished generating.
///
/// `Pending` mirrors pi's initial streaming partial (`stopReason: "pending"`,
/// `pi/packages/agent/src/proxy.ts:124`): it marks an in-progress assistant
/// message and is replaced by a final reason when the stream ends. It is
/// never a terminal stop reason.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum StopReason {
    Stop,
    ToolUse,
    MaxTokens,
    Error,
    Aborted,
    Pending,
}

/// Plain text content block.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TextContent {
    pub text: String,
}

/// Image content block.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ImageContent {
    pub data: Vec<u8>,
    pub mime_type: String,
}

/// Single content block on a user message.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum UserContent {
    Text { text: String },
    Image { data: Vec<u8>, mime_type: String },
}

/// A user message.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct UserMessage {
    pub content: Vec<UserContent>,
    pub timestamp: i64,
}

/// Partial or complete tool call emitted by the assistant.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ToolCall {
    pub id: String,
    pub name: String,
    pub arguments: serde_json::Value,
}

/// Content block on an assistant message.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum AssistantContent {
    Text { text: String },
    Thinking { text: String },
    ToolCall(ToolCall),
}

/// A (possibly partial) assistant message.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AssistantMessage {
    pub content: Vec<AssistantContent>,
    pub stop_reason: Option<StopReason>,
    pub model: String,
    /// Provider id (pi: `api`). Mirrors `AssistantMessage.api` + `.provider`.
    #[serde(default)]
    pub api: String,
    #[serde(default)]
    pub provider: String,
    /// Token usage for the finished message (pi: `usage`).
    #[serde(default)]
    pub usage: Usage,
    /// Failure detail when the stream ended in `error`/`aborted`
    /// (pi: `errorMessage?`).
    #[serde(default)]
    pub error_message: Option<String>,
    /// The raw provider stop reason string before normalization
    /// (pi: `rawStopReason?`).
    #[serde(default)]
    pub raw_stop_reason: Option<String>,
    pub timestamp: i64,
}

impl Default for AssistantMessage {
    fn default() -> Self {
        Self {
            content: Vec::new(),
            stop_reason: None,
            model: String::new(),
            api: String::new(),
            provider: String::new(),
            usage: Usage::default(),
            error_message: None,
            raw_stop_reason: None,
            timestamp: 0,
        }
    }
}

/// Tool result content block.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ToolResultContent {
    Text { text: String },
    Image { data: Vec<u8>, mime_type: String },
}

/// Result returned by a tool invocation, attached to the transcript as a
/// `ToolResultMessage`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ToolResultMessage {
    pub tool_call_id: String,
    pub tool_name: String,
    pub content: Vec<ToolResultContent>,
    /// Structured details (pi: `details?`).
    #[serde(default)]
    pub details: serde_json::Value,
    /// Token usage reported by the tool (pi: `usage?`).
    #[serde(default)]
    pub usage: Option<Usage>,
    /// Tool names discovered/added by this tool (pi: `addedToolNames?`).
    #[serde(default)]
    pub added_tool_names: Vec<String>,
    pub is_error: bool,
    pub timestamp: i64,
}

impl Default for ToolResultMessage {
    fn default() -> Self {
        Self {
            tool_call_id: String::new(),
            tool_name: String::new(),
            content: Vec::new(),
            details: serde_json::Value::Null,
            usage: None,
            added_tool_names: Vec::new(),
            is_error: false,
            timestamp: 0,
        }
    }
}

/// Token usage + cost accounting. Cost is per-million tokens in USD.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Usage {
    pub input: u64,
    pub output: u64,
    pub cache_read: u64,
    pub cache_write: u64,
    /// 1h cache-write tokens (pi: `cacheWrite1h?`).
    #[serde(default)]
    pub cache_write_1h: u64,
    /// Reasoning tokens (pi: `reasoning?`).
    #[serde(default)]
    pub reasoning: u64,
    pub total_tokens: u64,
    #[serde(default)]
    pub cost: CostBreakdown,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct CostBreakdown {
    pub input: u64,
    pub output: u64,
    pub cache_read: u64,
    pub cache_write: u64,
    pub total: u64,
}

/// Extension trait for app-level custom message types. Mirrors the TS
/// declaration-merging API.
pub trait AgentMessageExt: Send + Sync + 'static {
    fn role(&self) -> &str;
    fn timestamp(&self) -> i64;
}

/// An agent message: standard LLM message union + a custom escape hatch.
///
/// `Custom` is intentionally not part of the wire format; trait objects are
/// not (de)serializable without an explicit converter. Persisting custom
/// messages is the app's responsibility (use the role + timestamp).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AgentMessage {
    User(UserMessage),
    Assistant(AssistantMessage),
    ToolResult(ToolResultMessage),
    /// App-defined message type. Stored alongside the standard union so apps
    /// can introduce new roles without modifying the core.
    Custom(CustomMessage),
}

impl Serialize for AgentMessage {
    fn serialize<S: serde::Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        match self {
            AgentMessage::User(m) => m.serialize(s),
            AgentMessage::Assistant(m) => m.serialize(s),
            AgentMessage::ToolResult(m) => m.serialize(s),
            AgentMessage::Custom(_) => {
                // Custom is opaque on the wire; represent as null.
                s.serialize_none()
            }
        }
    }
}

impl<'de> Deserialize<'de> for AgentMessage {
    fn deserialize<D: serde::Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        let value = serde_json::Value::deserialize(d)?;
        if value.is_null() {
            return Err(serde::de::Error::custom(
                "cannot deserialize null AgentMessage",
            ));
        }
        let role = value.get("role").and_then(|v| v.as_str()).unwrap_or("");
        match role {
            "user" => Ok(AgentMessage::User(
                serde_json::from_value(value).map_err(serde::de::Error::custom)?,
            )),
            "assistant" => Ok(AgentMessage::Assistant(
                serde_json::from_value(value).map_err(serde::de::Error::custom)?,
            )),
            "toolResult" | "tool_result" => Ok(AgentMessage::ToolResult(
                serde_json::from_value(value).map_err(serde::de::Error::custom)?,
            )),
            _ => Err(serde::de::Error::custom(format!("unknown role: {role}"))),
        }
    }
}

/// Wrapper for app-level custom message types. The boxed trait object is
/// `serde`-opaque; persistence is the app's responsibility.
#[derive(Clone)]
pub struct CustomMessage(pub std::sync::Arc<dyn AgentMessageExt>);

impl std::fmt::Debug for CustomMessage {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("CustomMessage")
            .field("role", &self.0.role())
            .field("timestamp", &self.0.timestamp())
            .finish()
    }
}

impl PartialEq for CustomMessage {
    fn eq(&self, other: &Self) -> bool {
        self.0.role() == other.0.role() && self.0.timestamp() == other.0.timestamp()
    }
}

impl Eq for CustomMessage {}

impl AgentMessage {
    pub fn timestamp(&self) -> i64 {
        match self {
            Self::User(m) => m.timestamp,
            Self::Assistant(m) => m.timestamp,
            Self::ToolResult(m) => m.timestamp,
            Self::Custom(m) => m.0.timestamp(),
        }
    }
}

/// Wire-level message emitted to the LLM (after `convert_to_llm`).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "role", rename_all = "snake_case")]
pub enum WireMessage {
    User {
        content: Vec<UserContent>,
        timestamp: i64,
    },
    Assistant {
        content: Vec<AssistantContent>,
        stop_reason: Option<StopReason>,
        model: String,
        timestamp: i64,
    },
    ToolResult {
        tool_call_id: String,
        tool_name: String,
        content: Vec<ToolResultContent>,
        is_error: bool,
        timestamp: i64,
    },
}

/// Modality a model accepts (pi: `input: ("text"|"image")[]`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum InputKind {
    Text,
    Image,
}

/// Per-thinking-level token budgets (pi: `thinkingLevelMap?`).
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct ThinkingLevelMap {
    #[serde(default)]
    pub minimal: Option<u64>,
    #[serde(default)]
    pub low: Option<u64>,
    #[serde(default)]
    pub medium: Option<u64>,
    #[serde(default)]
    pub high: Option<u64>,
    #[serde(default)]
    pub xhigh: Option<u64>,
}

/// Static model description.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
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
    pub cost: CostBreakdown,
    #[serde(default)]
    pub context_window: u64,
    #[serde(default)]
    pub max_tokens: u64,
    /// Extra HTTP headers for provider requests (pi: `headers?`).
    #[serde(default)]
    pub headers: std::collections::HashMap<String, String>,
}

/// Options passed to a `StreamFn::stream` call.
#[derive(Debug, Clone, Default)]
pub struct SimpleStreamOptions {
    pub session_id: Option<String>,
    pub api_key: Option<String>,
    pub signal: Option<tokio::sync::watch::Receiver<bool>>,
    pub thinking_budgets: Option<ThinkingBudgets>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ThinkingBudgets {
    pub minimal: Option<u64>,
    pub low: Option<u64>,
    pub medium: Option<u64>,
    pub high: Option<u64>,
    pub xhigh: Option<u64>,
}

/// Context snapshot passed into a single loop iteration.
#[derive(Clone, Default)]
pub struct AgentContext {
    pub system_prompt: String,
    pub messages: Vec<AgentMessage>,
    pub tools: Vec<std::sync::Arc<dyn AgentTool>>,
}

/// Mutable agent state. Read-only fields are computed from the mutable ones
/// by `AgentStateActor`; external code accesses them through the snapshot
/// accessor.
#[derive(Clone, Default)]
pub struct AgentState {
    pub system_prompt: String,
    pub model: Model,
    pub thinking_level: ThinkingLevel,
    pub messages: Vec<AgentMessage>,
    pub tools: Vec<std::sync::Arc<dyn AgentTool>>,
}

/// Tool definition. Implementors populate name/label/description; the core
/// runtime invokes `execute` after validating `parameters` and running
/// `before_tool_call` / `after_tool_call` hooks.
#[async_trait::async_trait]
pub trait AgentTool: Send + Sync + 'static {
    fn name(&self) -> &str;
    fn label(&self) -> &str;
    fn description(&self) -> &str;
    /// Optional schema validation hook (returns Ok if valid).
    fn validate_arguments(&self, _args: &serde_json::Value) -> Result<(), String> {
        Ok(())
    }
    /// Optional argument preparation (pi `prepareArguments`, agent-loop.ts:586).
    /// Returns `Some(new_args)` to replace the tool call's arguments, or
    /// `None` to leave them unchanged.
    fn prepare_arguments(&self, _args: &serde_json::Value) -> Option<serde_json::Value> {
        None
    }
    /// Per-tool execution mode override. Default = None (use global).
    fn execution_mode(&self) -> Option<ToolExecutionMode> {
        None
    }
    /// Execute the tool. Throw on failure (return Err); the agent surfaces
    /// errors as `is_error: true` toolResult messages.
    async fn execute(
        &self,
        tool_call_id: &str,
        args: serde_json::Value,
        signal: Option<tokio_util::sync::CancellationToken>,
        on_update: Option<Box<dyn Fn(serde_json::Value) + Send + Sync>>,
    ) -> Result<AgentToolResult, String>;
}

/// Final or partial result produced by a tool.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct AgentToolResult {
    pub content: Vec<ToolResultContent>,
    pub details: serde_json::Value,
    pub usage: Option<Usage>,
    pub added_tool_names: Vec<String>,
    pub terminate: bool,
}

/// Event emitted by the agent for UI updates and for downstream subscribers.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum AgentEvent {
    AgentStart,
    AgentEnd {
        messages: Vec<AgentMessage>,
    },
    TurnStart,
    TurnEnd {
        message: AgentMessage,
        tool_results: Vec<ToolResultMessage>,
    },
    MessageStart {
        message: AgentMessage,
    },
    MessageUpdate {
        message: AgentMessage,
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
}

/// Per-event payload from a streaming assistant message.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum AssistantMessageEvent {
    Start,
    TextDelta {
        delta: String,
    },
    ThinkingDelta {
        delta: String,
    },
    ToolCallDelta {
        index: usize,
        partial: ToolCall,
    },
    Done {
        stop_reason: StopReason,
        usage: Usage,
    },
    Error {
        error: String,
    },
}

/// Returned by `before_tool_call`. `{ block: true }` short-circuits to a
/// synthetic error tool result.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct BeforeToolCallResult {
    pub block: bool,
    pub reason: Option<String>,
}

/// Returned by `after_tool_call`. Field-by-field override: any `Some` field
/// replaces the corresponding field on the executed result.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct AfterToolCallResult {
    pub content: Option<Vec<ToolResultContent>>,
    pub details: Option<serde_json::Value>,
    pub is_error: Option<bool>,
    pub usage: Option<Usage>,
    pub terminate: Option<bool>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BeforeToolCallContext {
    pub assistant_message: AssistantMessage,
    pub tool_call: ToolCall,
    pub args: serde_json::Value,
}

#[derive(Debug, Clone)]
pub struct AfterToolCallContext {
    pub assistant_message: AssistantMessage,
    pub tool_call: ToolCall,
    pub args: serde_json::Value,
    pub result: AgentToolResult,
    pub is_error: bool,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn thinking_level_serde_round_trip() {
        for level in [
            ThinkingLevel::Off,
            ThinkingLevel::Minimal,
            ThinkingLevel::Low,
            ThinkingLevel::Medium,
            ThinkingLevel::High,
            ThinkingLevel::XHigh,
            ThinkingLevel::Max,
        ] {
            let json = serde_json::to_string(&level).unwrap();
            let back: ThinkingLevel = serde_json::from_str(&json).unwrap();
            assert_eq!(level, back);
        }
    }

    #[test]
    fn assistant_message_round_trips_new_parity_fields() {
        let mut usage = Usage::default();
        usage.input = 10;
        usage.output = 20;
        let m = AssistantMessage {
            content: vec![AssistantContent::Text { text: "hi".into() }],
            stop_reason: Some(StopReason::Pending),
            model: "m".into(),
            api: "anthropic".into(),
            provider: "anthropic".into(),
            usage: usage.clone(),
            error_message: Some("boom".into()),
            raw_stop_reason: Some("max_tokens".into()),
            timestamp: 7,
        };
        let json = serde_json::to_value(&m).unwrap();
        // pi AssistantMessage carries these keys.
        for key in [
            "content",
            "stop_reason",
            "model",
            "api",
            "provider",
            "usage",
            "error_message",
            "raw_stop_reason",
            "timestamp",
        ] {
            assert!(json.get(key).is_some(), "missing {key}");
        }
        let back: AssistantMessage = serde_json::from_value(json).unwrap();
        assert_eq!(back, m);
        assert_eq!(back.usage.input, 10);
        assert_eq!(back.usage.output, 20);
    }

    #[test]
    fn tool_result_message_round_trips_new_parity_fields() {
        let m = ToolResultMessage {
            tool_call_id: "c1".into(),
            tool_name: "read".into(),
            content: vec![ToolResultContent::Text { text: "ok".into() }],
            details: serde_json::json!({ "lines": 3 }),
            usage: Some(Usage::default()),
            added_tool_names: vec!["lister".into()],
            is_error: false,
            timestamp: 1,
        };
        let json = serde_json::to_value(&m).unwrap();
        for key in [
            "tool_call_id",
            "tool_name",
            "content",
            "details",
            "usage",
            "added_tool_names",
            "is_error",
            "timestamp",
        ] {
            assert!(json.get(key).is_some(), "missing {key}");
        }
        let back: ToolResultMessage = serde_json::from_value(json).unwrap();
        assert_eq!(back, m);
        assert_eq!(back.added_tool_names, vec!["lister".to_string()]);
    }

    #[test]
    fn model_and_usage_round_trip_new_parity_fields() {
        let mut cost = CostBreakdown::default();
        cost.input = 3;
        cost.output = 15;
        let mut usage = Usage::default();
        usage.cache_write_1h = 9;
        usage.reasoning = 4;
        let m = Model {
            id: "m".into(),
            name: "m".into(),
            api: "a".into(),
            provider: "p".into(),
            base_url: "b".into(),
            reasoning: true,
            thinking_level_map: Some(ThinkingLevelMap {
                high: Some(1_000),
                ..Default::default()
            }),
            input: vec![InputKind::Text, InputKind::Image],
            cost: cost.clone(),
            context_window: 128,
            max_tokens: 64,
            headers: [("X-Test".to_string(), "1".to_string())]
                .into_iter()
                .collect(),
        };
        let json = serde_json::to_value(&m).unwrap();
        for key in [
            "id",
            "name",
            "api",
            "provider",
            "base_url",
            "reasoning",
            "thinking_level_map",
            "input",
            "cost",
            "context_window",
            "max_tokens",
            "headers",
        ] {
            assert!(json.get(key).is_some(), "Model missing {key}");
        }
        let back: Model = serde_json::from_value(json).unwrap();
        assert_eq!(back, m);

        let ujson = serde_json::to_value(&usage).unwrap();
        assert!(ujson.get("cache_write_1h").is_some());
        assert!(ujson.get("reasoning").is_some());
        let uback: Usage = serde_json::from_value(ujson).unwrap();
        assert_eq!(uback, usage);
    }

    #[test]
    fn agent_message_timestamp_dispatch() {
        let user = AgentMessage::User(UserMessage {
            content: vec![UserContent::Text { text: "hi".into() }],
            timestamp: 42,
        });
        assert_eq!(user.timestamp(), 42);
    }
}
