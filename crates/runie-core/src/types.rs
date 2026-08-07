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

/// Plain text content block.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TextContent {
    pub text: String,
}

/// Image content block.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ImageContent {
    /// Base64-encoded image data, matching pi-ai's wire representation.
    pub data: String,
    #[serde(rename = "mimeType")]
    pub mime_type: String,
}

/// Single content block on a user message.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum UserContent {
    Text {
        text: String,
    },
    Image {
        data: String,
        #[serde(rename = "mimeType")]
        mime_type: String,
    },
}

/// A user message.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct UserMessage {
    pub content: Vec<UserContent>,
    pub timestamp: i64,
}

/// Pi's internal context message emitted after session compaction. It is not
/// a user-authored message; the provider conversion layer wraps its summary
/// in Pi's compaction delimiters and emits a user wire message.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CompactionSummaryMessage {
    pub summary: String,
    pub tokens_before: u64,
    pub timestamp: i64,
}

impl<'de> Deserialize<'de> for UserMessage {
    fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        #[derive(Deserialize)]
        struct WireUserMessage {
            content: serde_json::Value,
            timestamp: i64,
        }

        let wire = WireUserMessage::deserialize(deserializer)?;
        let content = match wire.content {
            serde_json::Value::String(text) => vec![UserContent::Text { text }],
            value => serde_json::from_value(value).map_err(serde::de::Error::custom)?,
        };
        Ok(Self {
            content,
            timestamp: wire.timestamp,
        })
    }
}

/// Partial or complete tool call emitted by the assistant.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ToolCall {
    pub id: String,
    pub name: String,
    pub arguments: serde_json::Value,
    /// Provider-specific opaque signature (pi: `thoughtSignature`).
    #[serde(default)]
    pub thought_signature: Option<String>,
}

/// Redacted provider/runtime diagnostic attached to an assistant response.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AssistantMessageDiagnostic {
    #[serde(rename = "type")]
    pub diagnostic_type: String,
    pub timestamp: i64,
    #[serde(default)]
    pub error: Option<DiagnosticErrorInfo>,
    #[serde(default)]
    pub details: Option<serde_json::Map<String, serde_json::Value>>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DiagnosticErrorInfo {
    #[serde(default)]
    pub name: Option<String>,
    pub message: String,
    #[serde(default)]
    pub stack: Option<String>,
    #[serde(default)]
    pub code: Option<serde_json::Value>,
}

/// Content block on an assistant message.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum AssistantContent {
    Text {
        text: String,
    },
    Thinking {
        #[serde(rename = "thinking")]
        text: String,
    },
    ToolCall(ToolCall),
}

/// Provider handle returned when Pi defers completion to a later fetch.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DeferredHandle {
    pub provider: String,
    pub model_id: String,
    pub api: String,
    pub id: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub expires_at: Option<i64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub poll_after_ms: Option<u64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub data: Option<serde_json::Value>,
}

/// A (possibly partial) assistant message.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct AssistantMessage {
    pub content: Vec<AssistantContent>,
    pub stop_reason: Option<StopReason>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub deferred: Option<DeferredHandle>,
    pub model: String,
    /// Provider id (pi: `api`). Mirrors `AssistantMessage.api` + `.provider`.
    #[serde(default)]
    pub api: String,
    #[serde(default)]
    pub provider: String,
    /// Concrete response model when a provider routes a requested alias.
    #[serde(default)]
    pub response_model: Option<String>,
    /// Provider response/message identifier, when available.
    #[serde(default)]
    pub response_id: Option<String>,
    /// Redacted provider/runtime diagnostics for failures and recoveries.
    #[serde(default)]
    pub diagnostics: Vec<AssistantMessageDiagnostic>,
    /// Token usage for the finished message (pi: `usage`).
    #[serde(default)]
    pub usage: Usage,
    /// Server-derived thinking duration carried into the terminal message.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub thinking_elapsed_ms: Option<u64>,
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

impl AssistantMessage {
    /// Build the minimal assistant partial used by Pi's tool-call stream
    /// events. The provider partial is a complete assistant message, even
    /// when it currently contains only one tool-call content block.
    pub fn with_tool_call(call: ToolCall) -> Self {
        Self {
            content: vec![AssistantContent::ToolCall(call)],
            ..Self::default()
        }
    }

    pub fn with_error(reason: StopReason, message: impl Into<String>) -> Self {
        Self {
            stop_reason: Some(reason),
            error_message: Some(message.into()),
            ..Self::default()
        }
    }

    pub fn error_text(&self) -> String {
        self.error_message
            .clone()
            .unwrap_or_else(|| "assistant stream failed".to_owned())
    }
}

/// Tool result content block.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ToolResultContent {
    Text {
        text: String,
    },
    Image {
        data: String,
        #[serde(rename = "mimeType")]
        mime_type: String,
    },
}

/// Result returned by a tool invocation, attached to the transcript as a
/// `ToolResultMessage`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ToolResultMessage {
    pub tool_call_id: String,
    pub tool_name: String,
    pub content: Vec<ToolResultContent>,
    /// Structured details (pi: `details?`).
    #[serde(default, skip_serializing_if = "serde_json::Value::is_null")]
    pub details: serde_json::Value,
    /// Token usage reported by the tool (pi: `usage?`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub usage: Option<Usage>,
    /// Tool names discovered/added by this tool (pi: `addedToolNames?`).
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
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
#[serde(rename_all = "camelCase")]
#[serde(default)]
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

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CostBreakdown {
    pub input: f64,
    pub output: f64,
    pub cache_read: f64,
    pub cache_write: f64,
    pub total: f64,
}

// Cost values are wire-level numeric USD amounts. The model never produces
// NaN, so equality remains a valid state comparison for actor snapshots.
impl Eq for CostBreakdown {}

/// Model pricing rates and optional request-wide pricing tiers (pi `ModelCost`).
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ModelCost {
    pub input: f64,
    pub output: f64,
    pub cache_read: f64,
    pub cache_write: f64,
    #[serde(default)]
    pub tiers: Vec<ModelCostTier>,
}

impl Eq for ModelCost {}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ModelCostTier {
    pub input: f64,
    pub output: f64,
    pub cache_read: f64,
    pub cache_write: f64,
    #[serde(rename = "inputTokensAbove")]
    pub input_tokens_above: u64,
}

impl Eq for ModelCostTier {}

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
#[allow(
    clippy::large_enum_variant,
    reason = "AgentMessage preserves Pi's inline assistant payload"
)]
pub enum AgentMessage {
    User(UserMessage),
    Assistant(AssistantMessage),
    ToolResult(ToolResultMessage),
    CompactionSummary(CompactionSummaryMessage),
    /// App-defined message type. Stored alongside the standard union so apps
    /// can introduce new roles without modifying the core.
    Custom(CustomMessage),
}

impl Serialize for AgentMessage {
    fn serialize<S: serde::Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        let (role, value) = match self {
            AgentMessage::User(m) => ("user", serde_json::to_value(m)),
            AgentMessage::Assistant(m) => ("assistant", serde_json::to_value(m)),
            AgentMessage::ToolResult(m) => ("toolResult", serde_json::to_value(m)),
            AgentMessage::CompactionSummary(m) => ("compactionSummary", serde_json::to_value(m)),
            AgentMessage::Custom(_) => {
                // Custom is opaque on the wire; represent as null.
                return s.serialize_none();
            }
        };
        let mut value = value.map_err(serde::ser::Error::custom)?;
        let object = value
            .as_object_mut()
            .ok_or_else(|| serde::ser::Error::custom("agent message must serialize as object"))?;
        object.insert("role".into(), serde_json::Value::String(role.into()));
        value.serialize(s)
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
            "compactionSummary" | "compaction_summary" => Ok(AgentMessage::CompactionSummary(
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
            Self::CompactionSummary(m) => m.timestamp,
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
        details: serde_json::Value,
        usage: Option<Usage>,
        added_tool_names: Vec<String>,
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

/// Provider-specific reasoning-effort mappings (pi: `thinkingLevelMap?`).
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct ThinkingLevelMap {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub off: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub minimal: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub low: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub medium: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub high: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub xhigh: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub max: Option<String>,
}

/// Static model description.
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

/// Response metadata exposed to pi-compatible provider response hooks.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ProviderResponse {
    pub status: u16,
    pub headers: std::collections::HashMap<String, String>,
}

pub type PayloadHook = std::sync::Arc<
    dyn Fn(
            serde_json::Value,
            Model,
        ) -> futures::future::BoxFuture<'static, Option<serde_json::Value>>
        + Send
        + Sync,
>;

pub type ResponseHook = std::sync::Arc<
    dyn Fn(ProviderResponse, Model) -> futures::future::BoxFuture<'static, ()> + Send + Sync,
>;

pub type RetryDelayHook = std::sync::Arc<
    dyn Fn(
            u64,
            Option<tokio::sync::watch::Receiver<bool>>,
        ) -> futures::future::BoxFuture<'static, Result<(), crate::provider::StreamError>>
        + Send
        + Sync,
>;

pub type RetryJitterHook = std::sync::Arc<dyn Fn() -> f64 + Send + Sync>;

/// Options passed to a `StreamFn::stream` call.
#[derive(Clone, Default)]
pub struct SimpleStreamOptions {
    /// Optional actor-owned Pi telemetry capability; never serialized into a
    /// provider request.
    pub telemetry: Option<crate::telemetry::TelemetryActor>,
    pub session_id: Option<String>,
    pub api_key: Option<String>,
    /// Additional provider request headers (pi: `headers`).
    pub headers: Option<std::collections::HashMap<String, String>>,
    /// Provider-scoped environment and metadata carried with the request.
    pub env: Option<std::collections::HashMap<String, String>>,
    pub metadata: Option<std::collections::HashMap<String, serde_json::Value>>,
    /// Preferred Pi transport (`sse`, `websocket`, `websocket-cached`, or `auto`).
    pub transport: Option<ProviderTransport>,
    /// Pi prompt-cache retention preference for provider adapters.
    pub cache_retention: Option<CacheRetention>,
    /// WebSocket open-handshake timeout (pi: `websocketConnectTimeoutMs`).
    pub websocket_connect_timeout_ms: Option<u64>,
    pub signal: Option<tokio::sync::watch::Receiver<bool>>,
    pub thinking_budgets: Option<ThinkingBudgets>,
    /// Pi `SimpleStreamOptions.reasoning` override for provider adapters.
    pub reasoning: Option<ThinkingLevel>,
    /// Pi deferred-response request mode. Providers may ignore it when
    /// unsupported, but the typed boundary must not discard it.
    pub deferred: Option<DeferredRequest>,
    /// Explicit Pi stream temperature, kept separate from arbitrary sampling
    /// parameters because providers may map the two contracts differently.
    pub temperature: Option<f64>,
    /// Effective provider output limit (pi: `maxTokens`).
    pub max_tokens: Option<u64>,
    /// Provider request timeout in milliseconds (pi: `timeoutMs`).
    pub timeout_ms: Option<u64>,
    /// Maximum additional attempts after the initial provider request (pi:
    /// `maxRetries`).
    pub max_retries: Option<u32>,
    /// Maximum provider-requested retry delay (pi: `maxRetryDelayMs`). A
    /// value of zero disables the cap.
    pub max_retry_delay_ms: Option<u64>,
    /// Per-request sampling overrides. The loop merges these over
    /// `Model::sampling_params`, matching Pi's `StreamOptions` contract.
    pub sampling_params: Option<std::collections::HashMap<String, serde_json::Value>>,
    /// pi `onPayload`: provider adapters may inspect or replace request data.
    pub on_payload: Option<PayloadHook>,
    /// pi `onResponse`: provider adapters may observe response metadata.
    pub on_response: Option<ResponseHook>,
    /// Injectable scheduler for provider retry delays. Production uses an
    /// abortable Tokio timer; replay tests can record decisions without time.
    pub retry_delay: Option<RetryDelayHook>,
    /// Injectable `Math.random` equivalent for Pi's exponential retry jitter.
    pub retry_jitter: Option<RetryJitterHook>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum DeferredRequest {
    Enabled(bool),
    Window { window: DeferredWindow },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum DeferredWindow {
    #[serde(rename = "15m")]
    FifteenMinutes,
    #[serde(rename = "1h")]
    OneHour,
    #[serde(rename = "24h")]
    OneDay,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProviderTransport {
    Sse,
    Websocket,
    WebsocketCached,
    Auto,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CacheRetention {
    None,
    Short,
    Long,
}

impl std::fmt::Debug for SimpleStreamOptions {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("SimpleStreamOptions")
            .field("session_id", &self.session_id)
            .field("telemetry", &self.telemetry.is_some())
            .field("api_key", &self.api_key.as_ref().map(|_| "<redacted>"))
            .field("headers", &self.headers)
            .field("env", &self.env)
            .field("metadata", &self.metadata)
            .field("transport", &self.transport)
            .field("cache_retention", &self.cache_retention)
            .field("signal", &self.signal.is_some())
            .field("thinking_budgets", &self.thinking_budgets)
            .field("reasoning", &self.reasoning)
            .field("deferred", &self.deferred)
            .field("temperature", &self.temperature)
            .field("timeout_ms", &self.timeout_ms)
            .field("max_retries", &self.max_retries)
            .field("max_retry_delay_ms", &self.max_retry_delay_ms)
            .field("sampling_params", &self.sampling_params)
            .field("on_payload", &self.on_payload.is_some())
            .field("on_response", &self.on_response.is_some())
            .field("retry_delay", &self.retry_delay.is_some())
            .field("retry_jitter", &self.retry_jitter.is_some())
            .finish()
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
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
    /// `None` matches Pi's omitted optional tools field; `Some(empty)` means
    /// the caller explicitly supplied no tools.
    pub tools: Option<Vec<std::sync::Arc<dyn AgentTool>>>,
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
    /// Optional JSON Schema equivalent of Pi's TypeBox `parameters` field.
    /// The executor applies Pi-compatible scalar/object coercions before the
    /// custom validator and tool execution.
    fn parameters(&self) -> Option<serde_json::Value> {
        None
    }
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
fn is_false(value: &bool) -> bool {
    !*value
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AgentToolResult {
    pub content: Vec<ToolResultContent>,
    pub details: serde_json::Value,
    pub usage: Option<Usage>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub added_tool_names: Vec<String>,
    #[serde(skip_serializing_if = "is_false")]
    pub terminate: bool,
}

/// Event emitted by the agent for UI updates and for downstream subscribers.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", tag = "kind")]
pub enum WaitingReason {
    Model,
    Subagent,
    TaskOutput {
        task_ids: Vec<String>,
        subject: String,
    },
    TasksComplete,
    Sleep,
}

impl WaitingReason {
    /// Grok's user-facing turn-status subject for each typed wait state.
    pub fn label(&self) -> String {
        match self {
            Self::Model => "Waiting for response…".to_owned(),
            Self::Subagent => "Waiting on subagent…".to_owned(),
            Self::TaskOutput { subject, .. } if !subject.trim().is_empty() => {
                format!("{}…", clamp_wait_subject(subject))
            }
            Self::TaskOutput { .. } => "Waiting on task output…".to_owned(),
            Self::TasksComplete => "Waiting on tasks…".to_owned(),
            Self::Sleep => "Sleeping…".to_owned(),
        }
    }
}

fn clamp_wait_subject(subject: &str) -> String {
    const MAX_WAIT_SUBJECT_CHARS: usize = 40;
    let subject = subject.trim();
    if subject.chars().count() <= MAX_WAIT_SUBJECT_CHARS {
        subject.to_owned()
    } else {
        subject.chars().take(MAX_WAIT_SUBJECT_CHARS).collect()
    }
}

/// Named appearance variants shared by the event bus and TUI projections.
/// Rendering layers may quantize their palette to terminal capabilities.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum ThemeKind {
    #[default]
    GrokNight,
    GrokDay,
    TokyoNight,
    RosePineMoon,
    OscuraMidnight,
    Auto,
    /// Grok minimal-mode terminal-native palette; emits default/ANSI colors.
    TerminalNative,
}

/// Per-tool block display state used by Grok's scrollback renderer.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ToolDisplayMode {
    Collapsed,
    Truncated,
    Expanded,
}

/// Event emitted by the agent for UI updates and for downstream subscribers.
#[derive(Debug, Clone, Serialize, Deserialize)]
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
#[allow(
    clippy::field_reassign_with_default,
    clippy::too_many_lines,
    reason = "serialization tests exercise complete parity payloads in one round-trip"
)]
mod tests {
    use super::*;

    #[test]
    fn agent_context_distinguishes_omitted_and_explicit_empty_tools() {
        assert!(AgentContext::default().tools.is_none());
        assert!(AgentContext {
            tools: Some(Vec::new()),
            ..AgentContext::default()
        }
        .tools
        .expect("explicit empty tools")
        .is_empty());
    }

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
            response_model: Some("claude-3".into()),
            response_id: Some("resp-1".into()),
            diagnostics: vec![AssistantMessageDiagnostic {
                diagnostic_type: "recovery".into(),
                timestamp: 8,
                error: None,
                details: None,
            }],
            usage: usage.clone(),
            deferred: None,
            thinking_elapsed_ms: None,
            error_message: Some("boom".into()),
            raw_stop_reason: Some("max_tokens".into()),
            timestamp: 7,
        };
        let json = serde_json::to_value(&m).unwrap();
        // pi AssistantMessage carries these keys.
        for key in [
            "content",
            "stopReason",
            "model",
            "api",
            "provider",
            "responseModel",
            "responseId",
            "diagnostics",
            "usage",
            "errorMessage",
            "rawStopReason",
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
    fn agent_message_serialization_injects_pi_roles() {
        let messages = [
            AgentMessage::User(UserMessage {
                content: vec![UserContent::Text {
                    text: "hello".into(),
                }],
                timestamp: 1,
            }),
            AgentMessage::Assistant(AssistantMessage::default()),
            AgentMessage::ToolResult(ToolResultMessage::default()),
            AgentMessage::CompactionSummary(CompactionSummaryMessage {
                summary: "s".into(),
                tokens_before: 1,
                timestamp: 2,
            }),
        ];
        let roles = ["user", "assistant", "toolResult", "compactionSummary"];
        for (message, role) in messages.into_iter().zip(roles) {
            let json = serde_json::to_value(&message).expect("message wire value");
            assert_eq!(json["role"], role);
            let round_trip: AgentMessage = serde_json::from_value(json).expect("message decode");
            assert_eq!(round_trip, message);
        }
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
            "toolCallId",
            "toolName",
            "content",
            "details",
            "usage",
            "addedToolNames",
            "isError",
            "timestamp",
        ] {
            assert!(json.get(key).is_some(), "missing {key}");
        }
        let back: ToolResultMessage = serde_json::from_value(json).unwrap();
        assert_eq!(back, m);
        assert_eq!(back.added_tool_names, vec!["lister".to_string()]);
    }

    #[test]
    fn tool_result_message_omits_absent_optional_pi_fields() {
        let json = serde_json::to_value(ToolResultMessage::default()).unwrap();
        assert!(json.get("details").is_none());
        assert!(json.get("usage").is_none());
        assert!(json.get("addedToolNames").is_none());

        let back: ToolResultMessage = serde_json::from_value(json).unwrap();
        assert_eq!(back.details, serde_json::Value::Null);
        assert_eq!(back.usage, None);
        assert!(back.added_tool_names.is_empty());
    }

    #[test]
    #[allow(
        clippy::cognitive_complexity,
        reason = "the serde contract test compares the complete model/usage wire shape"
    )]
    fn model_and_usage_round_trip_new_parity_fields() {
        let cost = CostBreakdown {
            input: 3.25,
            output: 15.75,
            ..Default::default()
        };
        let usage = Usage {
            cache_write_1h: 9,
            reasoning: 4,
            ..Default::default()
        };
        let m = Model {
            id: "m".into(),
            name: "m".into(),
            api: "a".into(),
            provider: "p".into(),
            base_url: "b".into(),
            reasoning: true,
            thinking_level_map: Some(ThinkingLevelMap {
                high: Some("extended".into()),
                max: Some("maximum".into()),
                ..Default::default()
            }),
            input: vec![InputKind::Text, InputKind::Image],
            cost: ModelCost {
                input: cost.input,
                output: cost.output,
                cache_read: cost.cache_read,
                cache_write: cost.cache_write,
                tiers: vec![ModelCostTier {
                    input_tokens_above: 10_000,
                    ..Default::default()
                }],
            },
            context_window: 128,
            max_tokens: 64,
            sampling_params: Some(
                [("temperature".to_string(), serde_json::json!(0.2))]
                    .into_iter()
                    .collect(),
            ),
            headers: [("X-Test".to_string(), "1".to_string())]
                .into_iter()
                .collect(),
            compat: Some(serde_json::json!({"supports_reasoning": true})),
        };
        let json = serde_json::to_value(&m).unwrap();
        assert_eq!(json["cost"]["input"], serde_json::json!(3.25));
        assert_eq!(json["cost"]["output"], serde_json::json!(15.75));
        assert_eq!(json["cost"]["tiers"][0]["inputTokensAbove"], 10_000);
        assert_eq!(json["compat"]["supports_reasoning"], true);
        assert_eq!(json["thinkingLevelMap"]["high"], "extended");
        assert_eq!(json["thinkingLevelMap"]["max"], "maximum");
        assert_eq!(
            json["samplingParams"]["temperature"],
            serde_json::json!(0.2)
        );
        for key in [
            "id",
            "name",
            "api",
            "provider",
            "baseUrl",
            "reasoning",
            "thinkingLevelMap",
            "input",
            "cost",
            "contextWindow",
            "maxTokens",
            "samplingParams",
            "headers",
        ] {
            assert!(json.get(key).is_some(), "Model missing {key}");
        }
        let back: Model = serde_json::from_value(json).unwrap();
        assert_eq!(back, m);

        let ujson = serde_json::to_value(&usage).unwrap();
        assert!(ujson.get("cacheWrite1h").is_some());
        assert!(ujson.get("totalTokens").is_some());
        assert!(ujson.get("reasoning").is_some());
        let uback: Usage = serde_json::from_value(ujson).unwrap();
        assert_eq!(uback, usage);
    }

    #[test]
    fn assistant_message_event_subkinds_round_trip() {
        let events = vec![
            AssistantMessageEvent::Start {
                partial: AssistantMessage::default(),
            },
            AssistantMessageEvent::TextStart {
                index: 0,
                partial: AssistantMessage::default(),
            },
            AssistantMessageEvent::TextDelta {
                index: 0,
                delta: "hi".into(),
                partial: AssistantMessage::default(),
            },
            AssistantMessageEvent::TextEnd {
                index: 0,
                content: "hi".into(),
                partial: AssistantMessage::default(),
            },
            AssistantMessageEvent::ThinkingStart {
                index: 1,
                partial: AssistantMessage::default(),
            },
            AssistantMessageEvent::ThinkingDelta {
                index: 1,
                delta: "think".into(),
                partial: AssistantMessage::default(),
            },
            AssistantMessageEvent::ThinkingEnd {
                index: 1,
                content: "think".into(),
                elapsed_ms: None,
                partial: AssistantMessage::default(),
            },
            AssistantMessageEvent::ToolCallStart {
                index: 0,
                partial: AssistantMessage::with_tool_call(ToolCall {
                    id: "c".into(),
                    name: "x".into(),
                    arguments: serde_json::json!({}),
                    thought_signature: None,
                }),
            },
            AssistantMessageEvent::ToolCallDelta {
                index: 0,
                delta: "{\"path\":\"a.rs\"}".into(),
                partial: AssistantMessage::with_tool_call(ToolCall {
                    id: "c".into(),
                    name: "x".into(),
                    arguments: serde_json::json!({}),
                    thought_signature: None,
                }),
            },
            AssistantMessageEvent::ToolCallEnd {
                index: 0,
                tool_call: ToolCall {
                    id: "c".into(),
                    name: "x".into(),
                    arguments: serde_json::json!({}),
                    thought_signature: None,
                },
                partial: AssistantMessage::default(),
            },
            AssistantMessageEvent::Done {
                stop_reason: StopReason::Stop,
                usage: Usage::default(),
                message: None,
            },
            AssistantMessageEvent::Error {
                reason: StopReason::Error,
                error: AssistantMessage::with_error(StopReason::Error, "boom"),
            },
        ];
        for e in events {
            let json = serde_json::to_value(&e).unwrap();
            if matches!(&e, AssistantMessageEvent::ToolCallDelta { .. }) {
                assert_eq!(json["type"], "toolcall_delta");
                assert_eq!(json["contentIndex"], 0);
                assert_eq!(json["delta"], "{\"path\":\"a.rs\"}");
            }
            let back: AssistantMessageEvent = serde_json::from_value(json.clone()).unwrap();
            assert_eq!(serde_json::to_value(back).unwrap(), json);
        }
    }

    #[test]
    fn agent_message_timestamp_dispatch() {
        let user = AgentMessage::User(UserMessage {
            content: vec![UserContent::Text { text: "hi".into() }],
            timestamp: 42,
        });
        assert_eq!(user.timestamp(), 42);
    }

    #[test]
    fn image_content_uses_pi_base64_wire_strings() {
        let message = AgentMessage::User(UserMessage {
            content: vec![UserContent::Image {
                data: "aGVsbG8=".into(),
                mime_type: "image/png".into(),
            }],
            timestamp: 42,
        });
        let json = serde_json::to_value(&message).expect("image message serializes");
        assert_eq!(json["role"], "user");
        assert_eq!(json["content"][0]["type"], "image");
        assert_eq!(json["content"][0]["data"], "aGVsbG8=");
        assert_eq!(json["content"][0]["mimeType"], "image/png");
        let decoded: AgentMessage = serde_json::from_value(json).expect("image message decodes");
        assert_eq!(decoded, message);

        let thinking = serde_json::to_value(AssistantContent::Thinking {
            text: "considering".into(),
        })
        .expect("thinking content serializes");
        assert_eq!(thinking["thinking"], "considering");
        assert!(thinking.get("text").is_none());
    }

    #[test]
    fn user_message_accepts_pi_string_content_sugar() {
        let message: UserMessage = serde_json::from_value(serde_json::json!({
            "content": "hello",
            "timestamp": 42
        }))
        .expect("pi string content decodes");
        assert_eq!(
            message.content,
            vec![UserContent::Text {
                text: "hello".into()
            }]
        );
        assert_eq!(message.timestamp, 42);
    }

    #[test]
    #[allow(
        clippy::cognitive_complexity,
        reason = "one wire-contract test keeps all pi event key assertions together"
    )]
    fn event_wire_shapes_use_pi_tags_and_camel_case_fields() {
        let event = AgentEvent::ToolExecutionStart {
            tool_call_id: "call-1".into(),
            tool_name: "read".into(),
            args: serde_json::json!({"path": "README.md"}),
        };
        let json = serde_json::to_value(&event).expect("agent event serializes");
        assert_eq!(json["type"], "tool_execution_start");
        assert_eq!(json["toolCallId"], "call-1");
        assert!(json.get("tool_call_id").is_none());

        let lifecycle = [
            AgentEvent::ToolExecutionUpdate {
                tool_call_id: "call-1".into(),
                tool_name: "read".into(),
                args: serde_json::json!({"path": "README.md"}),
                partial_result: serde_json::json!({"output": "line"}),
            },
            AgentEvent::ToolExecutionEnd {
                tool_call_id: "call-1".into(),
                tool_name: "read".into(),
                result: serde_json::json!({"content": [{"type": "text", "text": "done"}]}),
                is_error: false,
            },
        ];
        for event in lifecycle {
            let json = serde_json::to_value(event).expect("tool lifecycle serializes");
            assert_eq!(json["toolCallId"], "call-1");
            assert_eq!(json["toolName"], "read");
            assert!(json.get("tool_call_id").is_none());
            assert!(json.get("tool_name").is_none());
        }

        let message_update = AgentEvent::MessageUpdate {
            message: AgentMessage::Assistant(AssistantMessage::default()),
            event: AssistantMessageEvent::TextDelta {
                index: 0,
                delta: "hi".into(),
                partial: AssistantMessage::default(),
            },
        };
        let update_json = serde_json::to_value(message_update).expect("message update serializes");
        assert!(update_json.get("event").is_none());
        assert_eq!(update_json["assistantMessageEvent"]["type"], "text_delta");

        let stream_start = serde_json::to_value(AssistantMessageEvent::TextStart {
            index: 2,
            partial: AssistantMessage::default(),
        })
        .expect("text start serializes");
        assert_eq!(stream_start["contentIndex"], 2);
        assert!(stream_start.get("index").is_none());

        let assistant_start = serde_json::to_value(AssistantMessageEvent::Start {
            partial: AssistantMessage::default(),
        })
        .expect("assistant start serializes");
        assert_eq!(assistant_start["type"], "start");
        assert!(assistant_start["partial"].is_object());

        let text_delta = serde_json::to_value(AssistantMessageEvent::TextDelta {
            index: 2,
            delta: "hi".into(),
            partial: AssistantMessage::default(),
        })
        .expect("text delta serializes");
        assert_eq!(text_delta["contentIndex"], 2);
        assert_eq!(text_delta["delta"], "hi");
        assert!(text_delta["partial"].is_object());

        let text_end = serde_json::to_value(AssistantMessageEvent::TextEnd {
            index: 2,
            content: "hello".into(),
            partial: AssistantMessage::default(),
        })
        .expect("text end serializes");
        assert_eq!(text_end["contentIndex"], 2);
        assert_eq!(text_end["content"], "hello");
        assert!(text_end["partial"].is_object());

        let thinking_delta = serde_json::to_value(AssistantMessageEvent::ThinkingDelta {
            index: 3,
            delta: "considering".into(),
            partial: AssistantMessage::default(),
        })
        .expect("thinking delta serializes");
        assert_eq!(thinking_delta["contentIndex"], 3);
        assert!(thinking_delta["partial"].is_object());

        let thinking_end = serde_json::to_value(AssistantMessageEvent::ThinkingEnd {
            index: 3,
            content: "considering".into(),
            elapsed_ms: Some(500),
            partial: AssistantMessage::default(),
        })
        .expect("thinking end serializes");
        assert_eq!(thinking_end["contentIndex"], 3);
        assert_eq!(thinking_end["content"], "considering");
        assert_eq!(thinking_end["elapsedMs"], 500);
        assert!(thinking_end["partial"].is_object());

        let done = serde_json::to_value(AssistantMessageEvent::Done {
            stop_reason: StopReason::ToolUse,
            usage: Usage::default(),
            message: None,
        })
        .expect("done event serializes");
        assert_eq!(done["reason"], "toolUse");
        assert!(done.get("stopReason").is_none());
        assert!(done.get("message").is_some());

        let error = serde_json::to_value(AssistantMessageEvent::Error {
            reason: StopReason::Aborted,
            error: AssistantMessage::with_error(StopReason::Aborted, "aborted"),
        })
        .expect("error event serializes");
        assert_eq!(error["reason"], "aborted");
        assert_eq!(error["error"]["errorMessage"], "aborted");

        let stream = AssistantMessageEvent::ToolCallEnd {
            index: 0,
            tool_call: ToolCall {
                id: "call-1".into(),
                name: "read".into(),
                arguments: serde_json::json!({}),
                thought_signature: Some("sig".into()),
            },
            partial: AssistantMessage::default(),
        };
        let stream_json = serde_json::to_value(&stream).expect("stream event serializes");
        assert_eq!(stream_json["type"], "toolcall_end");
        assert!(stream_json.get("tool_call").is_none());
        assert!(stream_json.get("toolCall").is_some());
        assert_eq!(stream_json["toolCall"]["thoughtSignature"], "sig");
        assert!(stream_json["partial"].is_object());
    }

    #[test]
    fn background_work_events_round_trip_with_pi_style_tags() {
        let events = [
            AgentEvent::BackgroundWorkStarted {
                work_id: "worker-1".into(),
                description: "inspect".into(),
                background: true,
            },
            AgentEvent::BackgroundWorkProgress {
                work_id: "worker-1".into(),
                description: "inspect".into(),
                activity: "reading".into(),
            },
            AgentEvent::BackgroundWorkFinished {
                work_id: "worker-1".into(),
                description: "inspect".into(),
                is_error: true,
                elapsed_ms: Some(900),
                error: Some("provider stopped".into()),
            },
        ];
        for event in events {
            let json = serde_json::to_value(&event).expect("background event serializes");
            assert!(json["type"]
                .as_str()
                .is_some_and(|kind| kind.starts_with("background_work_")));
            assert_eq!(json["workId"], "worker-1");
            let decoded: AgentEvent =
                serde_json::from_value(json).expect("background event decodes");
            assert_eq!(
                serde_json::to_value(decoded).expect("decoded event serializes"),
                serde_json::to_value(event).expect("original event serializes")
            );
        }
    }

    #[test]
    fn waiting_reason_labels_match_grok_subjects() {
        assert_eq!(WaitingReason::Model.label(), "Waiting for response…");
        assert_eq!(WaitingReason::Subagent.label(), "Waiting on subagent…");
        assert_eq!(WaitingReason::TasksComplete.label(), "Waiting on tasks…");
        assert_eq!(WaitingReason::Sleep.label(), "Sleeping…");
        assert_eq!(
            WaitingReason::TaskOutput {
                task_ids: vec!["t1".into()],
                subject: "compile project".into(),
            }
            .label(),
            "compile project…"
        );
    }

    #[test]
    fn tool_hook_payloads_use_pi_camel_case_keys() {
        let result = AgentToolResult {
            content: vec![],
            details: serde_json::json!({"ok": true}),
            usage: None,
            added_tool_names: vec!["search".into()],
            terminate: true,
        };
        let json = serde_json::to_value(result).expect("tool result serializes");
        assert_eq!(json["addedToolNames"], serde_json::json!(["search"]));
        assert_eq!(json["terminate"], true);
        assert!(json.get("added_tool_names").is_none());
        let empty_json =
            serde_json::to_value(AgentToolResult::default()).expect("empty tool result serializes");
        assert!(empty_json.get("addedToolNames").is_none());
        assert!(empty_json.get("terminate").is_none());

        let override_result = AfterToolCallResult {
            content: None,
            details: None,
            is_error: Some(true),
            usage: None,
            terminate: Some(false),
        };
        let override_json = serde_json::to_value(override_result).expect("override serializes");
        assert_eq!(override_json["isError"], true);
        assert_eq!(override_json["terminate"], false);
    }
}
