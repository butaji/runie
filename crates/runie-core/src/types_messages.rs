use super::*;
pub struct TextContent {
    pub text: String,
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
    Video {
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

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProviderOutcome {
    #[serde(default)]
    pub model: String,
    #[serde(default)]
    pub api: String,
    #[serde(default)]
    pub provider: String,
    pub finish_reason: Option<StopReason>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub raw_finish_reason: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub response_id: Option<String>,
    pub response_model: Option<String>,
    pub usage: Usage,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub raw_response: Option<serde_json::Value>,
}

impl AssistantMessage {
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

    pub fn provider_outcome(&self) -> ProviderOutcome {
        ProviderOutcome {
            model: self.model.clone(),
            api: self.api.clone(),
            provider: self.provider.clone(),
            finish_reason: self.stop_reason,
            raw_finish_reason: self.raw_stop_reason.clone(),
            response_id: self.response_id.clone(),
            response_model: self.response_model.clone(),
            usage: self.usage.clone(),
            raw_response: None,
        }
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

/// Modality a model accepts.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum InputKind {
    Text,
    Image,
    Video,
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
