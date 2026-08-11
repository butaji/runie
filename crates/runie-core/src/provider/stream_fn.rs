//! `StreamFn` trait — abstract LLM streaming interface.
//!
//! Adapters implement this for concrete providers (Anthropic, OpenAI,
//! Bedrock, etc.). The agent loop calls `stream` exactly once per assistant
//! turn; events arrive on the returned stream.

use std::pin::Pin;
use std::sync::{Arc, OnceLock};

use futures::Stream;

use crate::types::{AssistantMessageEvent, Model, SimpleStreamOptions};

pub type AssistantMessageEventStream = Pin<Box<dyn Stream<Item = AssistantMessageEvent> + Send>>;

/// Provider-scoped WebSocket capability. The generic HTTP actor must not
/// infer a WebSocket wire protocol; a concrete adapter owns the socket,
/// envelope, decoder, fallback policy, and cleanup lifecycle.
#[async_trait::async_trait]
pub trait WebSocketAdapter: Send + Sync + 'static {
    async fn stream_websocket(
        &self,
        model: &Model,
        context: &crate::types::AgentContext,
        options: Option<SimpleStreamOptions>,
    ) -> Result<AssistantMessageEventStream, StreamError>;
}

/// Module-level default stream fn singleton (pi `stream-fn.ts:15`).
static DEFAULT_STREAM_FN: OnceLock<Arc<dyn StreamFn>> = OnceLock::new();

/// Set the module-level default stream fn.
pub fn set_default_stream_fn(f: Arc<dyn StreamFn>) {
    let _ = DEFAULT_STREAM_FN.set(f);
}

/// Get the module-level default stream fn, or a pi-matched error if unset.
pub fn get_default_stream_fn() -> Result<Arc<dyn StreamFn>, StreamError> {
    DEFAULT_STREAM_FN
        .get()
        .cloned()
        .ok_or_else(|| StreamError::Api(
            "No default stream function configured. Pass streamFn explicitly or call set_default_stream_fn()."
                .into(),
        ))
}

/// Salvage-parse a streaming JSON fragment for tool-call arguments (pi
/// `parseStreamingJson`, proxy.ts:310). Tries a full parse first; if that
/// fails, closes any open string and matching brackets/braces to extract the
/// best-effort value.
pub fn parse_streaming_json(input: &str) -> Option<serde_json::Value> {
    if let Ok(v) = serde_json::from_str(input) {
        return Some(v);
    }
    serde_json::from_str(&close_streaming_fragment(input)).ok()
}

fn close_streaming_fragment(input: &str) -> String {
    let mut out = String::new();
    let mut in_string = false;
    let mut escaped = false;
    let mut stack: Vec<char> = Vec::new();
    for ch in input.chars() {
        if in_string {
            out.push(ch);
            if escaped {
                escaped = false;
            } else if ch == '\\' {
                escaped = true;
            } else if ch == '"' {
                in_string = false;
            }
            continue;
        }
        match ch {
            '"' => {
                in_string = true;
                out.push(ch);
            }
            '{' | '[' => {
                stack.push(ch);
                out.push(ch);
            }
            '}' => {
                stack.pop();
                out.push(ch);
            }
            ']' => {
                stack.pop();
                out.push(ch);
            }
            _ => out.push(ch),
        }
    }
    finish_streaming_fragment(out, in_string, stack)
}

fn finish_streaming_fragment(mut out: String, in_string: bool, mut stack: Vec<char>) -> String {
    if in_string {
        out.push('"');
    }
    while let Some(open) = stack.pop() {
        out.push(if open == '{' { '}' } else { ']' });
    }
    out
}

#[derive(Debug, Clone, thiserror::Error)]
pub enum StreamError {
    #[error("network: {0}")]
    Network(String),
    #[error("api: {0}")]
    Api(String),
    #[error("provider ({status:?}): {message}")]
    Provider {
        message: String,
        status: Option<u16>,
        headers: std::collections::HashMap<String, String>,
    },
    #[error("aborted")]
    Aborted,
    #[error("invalid: {0}")]
    Invalid(String),
}

macro_rules! provider_failure_kinds {
    ($(($variant:ident, $wire:literal)),+ $(,)?) => {
        #[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
        #[serde(rename_all = "snake_case")]
        pub enum ProviderFailureKind {
            $($variant),+
        }

        impl ProviderFailureKind {
            pub const fn wire_name(self) -> &'static str {
                match self { $(Self::$variant => $wire),+ }
            }

            pub fn from_wire_name(name: &str) -> Option<Self> {
                match name { $($wire => Some(Self::$variant),)+ _ => None }
            }
        }
    };
}

provider_failure_kinds! {
    (Network, "network"),
    (Api, "api"),
    (Provider, "provider"),
    (RateLimited, "rate_limited"),
    (UnsupportedEffort, "unsupported_effort"),
    (Aborted, "aborted"),
    (Invalid, "invalid"),
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct ProviderFailure {
    pub kind: ProviderFailureKind,
    pub message: String,
    pub status: Option<u16>,
    pub retryable: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub retry_after: Option<String>,
}

impl ProviderFailure {
    pub fn terminal_line(&self) -> String {
        let status = self
            .status
            .map(|value| format!(" status={value}"))
            .unwrap_or_default();
        let retry_after = self
            .retry_after
            .as_deref()
            .map(|value| format!(" retry_after={value}"))
            .unwrap_or_default();
        format!(
            "{}{}{} retryable={} · {}",
            self.kind.wire_name(),
            status,
            retry_after,
            self.retryable,
            self.message
        )
    }
}

pub fn classify_failure(error: &StreamError) -> ProviderFailure {
    let (kind, message, status, retryable, retry_after) = match error {
        StreamError::Network(message) => (
            ProviderFailureKind::Network,
            message.clone(),
            None,
            true,
            None,
        ),
        StreamError::Api(message) => (ProviderFailureKind::Api, message.clone(), None, false, None),
        StreamError::Provider {
            message,
            status,
            headers,
        } => classify_provider_failure(message, *status, headers),
        StreamError::Aborted => (
            ProviderFailureKind::Aborted,
            "aborted".into(),
            None,
            false,
            None,
        ),
        StreamError::Invalid(message) => (
            ProviderFailureKind::Invalid,
            message.clone(),
            None,
            false,
            None,
        ),
    };
    ProviderFailure {
        kind,
        message,
        status,
        retryable,
        retry_after,
    }
}

fn classify_provider_failure(
    message: &str,
    status: Option<u16>,
    headers: &std::collections::HashMap<String, String>,
) -> (
    ProviderFailureKind,
    String,
    Option<u16>,
    bool,
    Option<String>,
) {
    (
        if status != Some(429) && unsupported_effort(message) {
            ProviderFailureKind::UnsupportedEffort
        } else if status == Some(429) {
            ProviderFailureKind::RateLimited
        } else {
            ProviderFailureKind::Provider
        },
        message.to_owned(),
        status,
        provider_retryable(status, headers),
        retry_after_header(headers),
    )
}

fn unsupported_effort(message: &str) -> bool {
    let message = message.to_ascii_lowercase();
    message.contains("effort")
        && ["unsupported", "invalid", "not support", "unknown"]
            .iter()
            .any(|marker| message.contains(marker))
}

fn retry_after_header(headers: &std::collections::HashMap<String, String>) -> Option<String> {
    headers
        .iter()
        .find(|(key, _)| {
            key.eq_ignore_ascii_case("retry-after-ms") || key.eq_ignore_ascii_case("retry-after")
        })
        .map(|(_, value)| value.chars().take(128).collect())
}

pub(crate) fn provider_retryable(
    status: Option<u16>,
    headers: &std::collections::HashMap<String, String>,
) -> bool {
    if let Some(value) = headers
        .iter()
        .find(|(key, _)| key.eq_ignore_ascii_case("x-should-retry"))
        .map(|(_, value)| value)
    {
        return value.eq_ignore_ascii_case("true");
    }
    matches!(status, Some(408 | 409 | 425 | 429 | 500..=599))
}

#[async_trait::async_trait]
pub trait StreamFn: Send + Sync + 'static {
    async fn stream(
        &self,
        model: &Model,
        context: &crate::types::AgentContext,
        options: Option<SimpleStreamOptions>,
    ) -> Result<AssistantMessageEventStream, StreamError>;

    /// Optional Pi compaction-summary capability. Providers own summary
    /// generation; the session actor owns preparation and publication.
    async fn summarize_compaction(
        &self,
        _request: &crate::session::CompactionSummaryRequest,
    ) -> Result<crate::session::CompactionSummary, StreamError> {
        Err(StreamError::Invalid(
            "provider does not support compaction summaries".into(),
        ))
    }

    /// Optional Pi model-catalog capability. Providers own discovery and
    /// authentication; the model actor owns admission of the result.
    async fn list_models(&self) -> Result<Vec<Model>, StreamError> {
        Err(StreamError::Invalid(
            "provider does not support model discovery".into(),
        ))
    }

    /// Optional Pi deferred-response polling capability.
    async fn fetch_deferred(
        &self,
        _model: &Model,
        _handle: &crate::types::DeferredHandle,
        _options: Option<SimpleStreamOptions>,
    ) -> Result<AssistantMessageEventStream, StreamError> {
        Err(StreamError::Invalid(
            "provider does not support deferred responses".into(),
        ))
    }

    /// Optional Pi deferred-response cancellation capability.
    async fn cancel_deferred(
        &self,
        _model: &Model,
        _handle: &crate::types::DeferredHandle,
        _options: Option<SimpleStreamOptions>,
    ) -> Result<(), StreamError> {
        Err(StreamError::Invalid(
            "provider cannot cancel deferred responses".into(),
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    struct TestFn;
    #[async_trait::async_trait]
    impl StreamFn for TestFn {
        async fn stream(
            &self,
            _model: &Model,
            _context: &crate::types::AgentContext,
            _options: Option<SimpleStreamOptions>,
        ) -> Result<AssistantMessageEventStream, StreamError> {
            use futures::stream;
            Ok(Box::pin(stream::empty()))
        }
    }

    #[tokio::test]
    async fn trait_object_works() {
        let _f: std::sync::Arc<dyn StreamFn> = std::sync::Arc::new(TestFn);
    }

    #[test]
    fn default_stream_fn_unset_returns_pi_error() {
        // The test binary may have set it elsewhere in the same process; only
        // assert when it is unset.
        if let Err(e) = get_default_stream_fn() {
            assert!(
                e.to_string()
                    .contains("No default stream function configured"),
                "error should match pi"
            );
        }
    }

    #[test]
    fn parse_streaming_json_full_and_salvage() {
        #[derive(serde::Deserialize)]
        struct Case {
            input: String,
            expected: Option<serde_json::Value>,
        }
        let cases: Vec<Case> = serde_yaml::from_str(include_str!("stream-json.yaml")).unwrap();
        for case in cases {
            assert_eq!(parse_streaming_json(&case.input), case.expected);
        }
    }

    #[test]
    fn failure_classification_preserves_kind_status_and_retry_policy() {
        let error = StreamError::Provider {
            message: "busy".into(),
            status: Some(503),
            headers: [("x-should-retry".into(), "true".into())]
                .into_iter()
                .collect(),
        };
        let failure = classify_failure(&error);
        assert_eq!(failure.kind, ProviderFailureKind::Provider);
        assert_eq!(failure.status, Some(503));
        assert!(failure.retryable);
        let decoded: ProviderFailure =
            serde_json::from_value(serde_json::to_value(failure).unwrap()).unwrap();
        assert_eq!(decoded.message, "busy");
    }

    #[test]
    fn provider_failure_kinds_round_trip_through_wire_names() {
        for kind in [
            ProviderFailureKind::Network,
            ProviderFailureKind::Api,
            ProviderFailureKind::Provider,
            ProviderFailureKind::RateLimited,
            ProviderFailureKind::Aborted,
            ProviderFailureKind::Invalid,
        ] {
            assert_eq!(
                ProviderFailureKind::from_wire_name(kind.wire_name()),
                Some(kind)
            );
        }
        assert_eq!(ProviderFailureKind::from_wire_name("unknown"), None);
    }

    #[test]
    fn rate_limit_failures_keep_retryable_provider_context() {
        let failure = classify_failure(&StreamError::Provider {
            message: "slow down".into(),
            status: Some(429),
            headers: Default::default(),
        });
        assert_eq!(failure.kind, ProviderFailureKind::RateLimited);
        assert!(failure.retryable);
        assert_eq!(
            failure.terminal_line(),
            "rate_limited status=429 retryable=true · slow down"
        );
    }

    #[test]
    fn provider_failure_terminal_line_preserves_retry_context() {
        let failure = ProviderFailure {
            kind: ProviderFailureKind::Provider,
            message: "busy".into(),
            status: Some(503),
            retryable: true,
            retry_after: None,
        };
        assert_eq!(
            failure.terminal_line(),
            "provider status=503 retryable=true · busy"
        );
    }

    #[test]
    fn provider_failure_infers_transient_status_and_honors_explicit_false() {
        let transient = StreamError::Provider {
            message: "busy".into(),
            status: Some(503),
            headers: Default::default(),
        };
        assert!(classify_failure(&transient).retryable);

        let explicit_no_retry = StreamError::Provider {
            message: "invalid request".into(),
            status: Some(503),
            headers: [("X-Should-Retry".into(), "false".into())]
                .into_iter()
                .collect(),
        };
        assert!(!classify_failure(&explicit_no_retry).retryable);
    }

    #[test]
    fn retry_status_matrix_is_shared_by_provider_boundaries() {
        for (status, expected) in [
            (Some(408), true),
            (Some(409), true),
            (Some(425), true),
            (Some(429), true),
            (Some(500), true),
            (Some(404), false),
            (None, false),
        ] {
            assert_eq!(provider_retryable(status, &Default::default()), expected);
        }
        assert!(!provider_retryable(
            Some(500),
            &[("X-Should-Retry".into(), "false".into())]
                .into_iter()
                .collect()
        ));
        assert!(provider_retryable(
            Some(400),
            &[("x-should-retry".into(), "true".into())]
                .into_iter()
                .collect()
        ));
    }
}
