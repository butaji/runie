use super::*;
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

macro_rules! provider_wire_projection {
    ($type:ty, $(($variant:path, $wire:literal)),+ $(,)?) => {
        impl $type {
            pub const fn wire_name(self) -> &'static str {
                match self { $($variant => $wire),+ }
            }
        }
    };
}

provider_wire_projection! {
    DeferredWindow,
    (DeferredWindow::FifteenMinutes, "15m"),
    (DeferredWindow::OneHour, "1h"),
    (DeferredWindow::OneDay, "24h"),
}

macro_rules! provider_transports {
    ($(($variant:ident, $wire:literal)),+ $(,)?) => {
        #[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
        #[serde(rename_all = "kebab-case")]
        pub enum ProviderTransport {
            $($variant),+
        }

        impl ProviderTransport {
            pub const fn wire_name(self) -> &'static str {
                match self {
                    $(Self::$variant => $wire),+
                }
            }
        }
    };
}

provider_transports! {
    (Sse, "sse"),
    (Websocket, "websocket"),
    (WebsocketCached, "websocket-cached"),
    (Auto, "auto"),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum CacheRetention {
    None,
    Short,
    Long,
}

provider_wire_projection! {
    CacheRetention,
    (CacheRetention::None, "none"),
    (CacheRetention::Short, "short"),
    (CacheRetention::Long, "long"),
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
    /// Optional exclusive resource key. Calls sharing a key are not run in
    /// the same parallel batch (for example, two tools writing one workspace).
    fn resource_key(&self, _args: &serde_json::Value) -> Option<String> {
        None
    }
    /// Optional input modality required before exposing this tool to a model.
    fn required_input(&self) -> Option<crate::types::InputKind> {
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

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ToolResultStatus {
    Ok,
    Error,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ToolResultTerminalProjection {
    pub status: ToolResultStatus,
    pub content_blocks: usize,
    pub detail_keys: Vec<String>,
    pub has_usage: bool,
    pub terminated: bool,
}

impl AgentToolResult {
    /// Bounded renderer-neutral metadata shared by TUI and noninteractive hosts.
    pub fn terminal_projection(&self, is_error: bool) -> ToolResultTerminalProjection {
        let mut detail_keys = self
            .details
            .as_object()
            .map(|details| details.keys().cloned().collect::<Vec<_>>())
            .unwrap_or_default();
        detail_keys.sort();
        ToolResultTerminalProjection {
            status: if is_error {
                ToolResultStatus::Error
            } else {
                ToolResultStatus::Ok
            },
            content_blocks: self.content.len(),
            detail_keys,
            has_usage: self.usage.is_some(),
            terminated: self.terminate,
        }
    }
}

#[cfg(test)]
mod terminal_projection_tests {
    use super::*;

    #[test]
    fn tool_result_projection_is_bounded_metadata() {
        let result = AgentToolResult {
            content: vec![ToolResultContent::Text {
                text: "large".into(),
            }],
            details: serde_json::json!({"z": 1, "a": 2}),
            usage: Some(Usage::default()),
            terminate: true,
            ..AgentToolResult::default()
        };
        assert_eq!(
            result.terminal_projection(true),
            ToolResultTerminalProjection {
                status: ToolResultStatus::Error,
                content_blocks: 1,
                detail_keys: vec!["a".into(), "z".into()],
                has_usage: true,
                terminated: true,
            }
        );
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum OperationRecordKind {
    OperationStarted,
    AbortRequested,
    OperationFinished,
    StepAttempt,
    ToolStarted,
    QueueEnqueued,
    QueueCancelled,
    WriteDeferred,
    Usage,
}

impl OperationRecordKind {
    /// Decode a Pi wire record name at the compatibility edge. Producers and
    /// internal events should use the closed enum; unknown names remain
    /// available to the lossless legacy record path.
    pub fn from_wire_name(name: &str) -> Option<Self> {
        operation_record_kind_from_wire(name)
    }
}

macro_rules! operation_record_kinds {
    ($(($kind:ident, $wire_name:literal)),+ $(,)?) => {
        impl OperationRecordKind {
            pub const fn wire_name(self) -> &'static str {
                match self { $(Self::$kind => $wire_name,)+ }
            }
        }

        fn operation_record_kind_from_wire(name: &str) -> Option<OperationRecordKind> {
            Some(match name { $($wire_name => OperationRecordKind::$kind,)+ _ => return None })
        }
    };
}

operation_record_kinds! {
    (OperationStarted, "operation_started"),
    (AbortRequested, "abort_requested"),
    (OperationFinished, "operation_finished"),
    (StepAttempt, "step_attempt"),
    (ToolStarted, "tool_started"),
    (QueueEnqueued, "queue_enqueued"),
    (QueueCancelled, "queue_cancelled"),
    (WriteDeferred, "write_deferred"),
    (Usage, "usage"),
}
