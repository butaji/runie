//! Pure wire helpers for Pi's OpenAI Codex Responses provider.

use std::collections::{HashMap, HashSet};
use std::sync::Arc;

use crate::provider::stream_fn::{
    AssistantMessageEventStream, StreamError, StreamFn, WebSocketAdapter,
};
use crate::types::{AgentContext, Model, SimpleStreamOptions};

const DEFAULT_CODEX_BASE_URL: &str = "https://chatgpt.com/backend-api";
pub const CODEX_WEBSOCKET_BETA_HEADER: &str = "responses_websockets=2026-02-06";

/// Provider-owned continuation state for one cached Codex session/account
/// connection. The socket itself remains an injected transport concern.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct WebSocketContinuation {
    pub last_response_id: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct WebSocketSessionKey {
    pub session_id: String,
    pub account_id: String,
}

/// Provider-owned continuation/fallback state. A concrete socket adapter
/// owns this value and is responsible for closing any socket represented by a
/// removed continuation before dropping it.
#[derive(Debug, Default)]
pub struct WebSocketSessionCache {
    continuations: HashMap<WebSocketSessionKey, WebSocketContinuation>,
    sse_fallback_sessions: HashSet<String>,
}

#[async_trait::async_trait]
pub trait CodexWebSocket: Send {
    async fn send_text(&mut self, text: String) -> Result<(), StreamError>;
    async fn next_text(&mut self) -> Result<Option<String>, StreamError>;
    async fn close(&mut self);
}

#[async_trait::async_trait]
pub trait CodexWebSocketConnector: Send + Sync + 'static {
    async fn connect(
        &self,
        url: String,
        headers: HashMap<String, String>,
        timeout_ms: Option<u64>,
    ) -> Result<Box<dyn CodexWebSocket>, StreamError>;
}

pub type CodexRequestBuilder = Arc<
    dyn Fn(&Model, &AgentContext, Option<&SimpleStreamOptions>) -> Result<serde_json::Value, String>
        + Send
        + Sync,
>;

/// A provider-scoped Codex Responses WebSocket adapter. Network construction
/// is injected so production can supply a real socket while replay tests use
/// an owned deterministic connector.
pub struct CodexWebSocketAdapter {
    connector: Arc<dyn CodexWebSocketConnector>,
    request_builder: CodexRequestBuilder,
    base_url: Option<String>,
    headers: HashMap<String, String>,
    fallback: Option<Arc<dyn StreamFn>>,
}

impl CodexWebSocketAdapter {
    pub fn new(
        connector: Arc<dyn CodexWebSocketConnector>,
        request_builder: CodexRequestBuilder,
        base_url: Option<String>,
        headers: HashMap<String, String>,
        fallback: Option<Arc<dyn StreamFn>>,
    ) -> Self {
        Self {
            connector,
            request_builder,
            base_url,
            headers,
            fallback,
        }
    }
}

#[async_trait::async_trait]
#[allow(
    clippy::too_many_lines,
    reason = "the provider adapter keeps socket lifecycle and decoder settlement in one boundary"
)]
impl WebSocketAdapter for CodexWebSocketAdapter {
    async fn stream_websocket(
        &self,
        model: &Model,
        context: &AgentContext,
        options: Option<SimpleStreamOptions>,
    ) -> Result<AssistantMessageEventStream, StreamError> {
        let url = resolve_websocket_url(self.base_url.as_deref()).map_err(StreamError::Invalid)?;
        let body = (self.request_builder)(model, context, options.as_ref())
            .map_err(StreamError::Invalid)?;
        let frame = response_create_continuation_frame(body, None).map_err(StreamError::Invalid)?;
        let mut headers = self.headers.clone();
        headers.insert("OpenAI-Beta".into(), CODEX_WEBSOCKET_BETA_HEADER.into());
        let timeout_ms = options
            .as_ref()
            .and_then(|value| value.websocket_connect_timeout_ms);
        let mut socket = match self.connector.connect(url, headers, timeout_ms).await {
            Ok(socket) => socket,
            Err(error) => {
                if let Some(fallback) = &self.fallback {
                    return fallback.stream(model, context, options).await;
                }
                return Err(error);
            }
        };
        let send_result = socket.send_text(frame.to_string()).await;
        if let Err(error) = send_result {
            socket.close().await;
            if let Some(fallback) = &self.fallback {
                return fallback.stream(model, context, options).await;
            }
            return Err(error);
        }
        let mut messages = Vec::new();
        let mut started = false;
        loop {
            match socket.next_text().await {
                Err(error) => {
                    socket.close().await;
                    return Err(error);
                }
                Ok(Some(message)) => {
                    let value = match serde_json::from_str::<serde_json::Value>(&message) {
                        Ok(value) => value,
                        Err(error) => {
                            socket.close().await;
                            return Err(StreamError::Invalid(format!(
                                "invalid Codex WebSocket JSON: {error}"
                            )));
                        }
                    };
                    if !value.is_object() {
                        socket.close().await;
                        return Err(StreamError::Invalid(
                            "Codex WebSocket message must be a JSON object".into(),
                        ));
                    }
                    started = true;
                    let terminal = matches!(
                        value.get("type").and_then(|value| value.as_str()),
                        Some("response.completed")
                            | Some("response.incomplete")
                            | Some("response.failed")
                            | Some("error")
                    );
                    messages.push(message);
                    if terminal {
                        break;
                    }
                }
                Ok(None) => break,
            }
        }
        socket.close().await;
        if !started {
            return Err(StreamError::Network(
                "Codex WebSocket closed before streaming".into(),
            ));
        }
        let provider = crate::provider::replay::ReplayProvider::from_websocket_messages(messages)?;
        provider.stream(model, context, options).await
    }
}

impl WebSocketSessionCache {
    pub fn continuation(&self, key: &WebSocketSessionKey) -> Option<&WebSocketContinuation> {
        self.continuations.get(key)
    }

    pub fn store_continuation(
        &mut self,
        key: WebSocketSessionKey,
        continuation: WebSocketContinuation,
    ) {
        self.continuations.insert(key, continuation);
    }

    pub fn mark_sse_fallback(&mut self, session_id: impl Into<String>) {
        self.sse_fallback_sessions.insert(session_id.into());
    }

    pub fn is_sse_fallback_active(&self, session_id: &str) -> bool {
        self.sse_fallback_sessions.contains(session_id)
    }

    /// Remove all account connections and fallback state for one session.
    pub fn clear_session(&mut self, session_id: &str) {
        self.continuations
            .retain(|key, _| key.session_id != session_id);
        self.sse_fallback_sessions.remove(session_id);
    }

    /// Clear all provider-owned session state during global cleanup.
    pub fn clear(&mut self) {
        self.continuations.clear();
        self.sse_fallback_sessions.clear();
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WebSocketFailureDecision {
    RetryConnection,
    RetryContinuation,
    FallbackToSse,
    Propagate,
}

/// Classify the failure boundary used by Pi's Codex WebSocket adapter.
pub fn classify_websocket_failure(
    started: bool,
    error_code: Option<&str>,
    retried_connection_limit: bool,
    retried_missing_continuation: bool,
) -> WebSocketFailureDecision {
    if error_code == Some("previous_response_not_found") && !retried_missing_continuation {
        return WebSocketFailureDecision::RetryContinuation;
    }
    if !started
        && error_code == Some("websocket_connection_limit_reached")
        && !retried_connection_limit
    {
        return WebSocketFailureDecision::RetryConnection;
    }
    if started {
        WebSocketFailureDecision::Propagate
    } else {
        WebSocketFailureDecision::FallbackToSse
    }
}

/// Build the provider-owned WebSocket frame while preserving the request
/// body. A continuation is only attached when the cached connection has a
/// response id, matching Pi's delta request shape.
pub fn response_create_continuation_frame(
    mut body: serde_json::Value,
    continuation: Option<&WebSocketContinuation>,
) -> Result<serde_json::Value, String> {
    let object = body
        .as_object_mut()
        .ok_or_else(|| "Codex response body must be a JSON object".to_owned())?;
    if let Some(response_id) = continuation.and_then(|state| state.last_response_id.as_ref()) {
        object.insert(
            "previous_response_id".to_owned(),
            serde_json::Value::String(response_id.clone()),
        );
    }
    response_create_envelope(body)
}

pub fn resolve_responses_url(base_url: Option<&str>) -> String {
    let raw = base_url
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .unwrap_or(DEFAULT_CODEX_BASE_URL)
        .trim_end_matches('/');
    if raw.ends_with("/codex/responses") {
        raw.to_owned()
    } else if raw.ends_with("/codex") {
        format!("{raw}/responses")
    } else {
        format!("{raw}/codex/responses")
    }
}

pub fn resolve_websocket_url(base_url: Option<&str>) -> Result<String, String> {
    let url = resolve_responses_url(base_url);
    if let Some(rest) = url.strip_prefix("https://") {
        return Ok(format!("wss://{rest}"));
    }
    if let Some(rest) = url.strip_prefix("http://") {
        return Ok(format!("ws://{rest}"));
    }
    if url.starts_with("ws://") || url.starts_with("wss://") {
        return Ok(url);
    }
    Err("unsupported Codex URL scheme".to_owned())
}

pub fn response_create_envelope(mut body: serde_json::Value) -> Result<serde_json::Value, String> {
    let object = body
        .as_object_mut()
        .ok_or_else(|| "Codex response body must be a JSON object".to_owned())?;
    object.insert("type".to_owned(), serde_json::json!("response.create"));
    Ok(body)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn resolves_codex_http_and_websocket_urls_like_pi() {
        assert_eq!(
            resolve_responses_url(Some("https://example.test/api/")),
            "https://example.test/api/codex/responses"
        );
        assert_eq!(
            resolve_responses_url(Some("https://example.test/codex")),
            "https://example.test/codex/responses"
        );
        assert_eq!(
            resolve_websocket_url(Some("https://example.test/codex/responses")).unwrap(),
            "wss://example.test/codex/responses"
        );
    }

    #[test]
    fn response_create_envelope_overwrites_wire_type_only() {
        let envelope = response_create_envelope(serde_json::json!({
            "model": "gpt-5",
            "type": "response.other"
        }))
        .unwrap();
        assert_eq!(envelope["type"], "response.create");
        assert_eq!(envelope["model"], "gpt-5");
        assert!(response_create_envelope(serde_json::json!(null)).is_err());
    }

    #[test]
    fn continuation_frame_and_failure_policy_match_codex_boundaries() {
        let frame = response_create_continuation_frame(
            serde_json::json!({"input": []}),
            Some(&WebSocketContinuation {
                last_response_id: Some("resp-1".into()),
            }),
        )
        .unwrap();
        assert_eq!(frame["type"], "response.create");
        assert_eq!(frame["previous_response_id"], "resp-1");
        assert_eq!(
            classify_websocket_failure(
                false,
                Some("websocket_connection_limit_reached"),
                false,
                false
            ),
            WebSocketFailureDecision::RetryConnection
        );
        assert_eq!(
            classify_websocket_failure(false, Some("previous_response_not_found"), false, false),
            WebSocketFailureDecision::RetryContinuation
        );
        assert_eq!(
            classify_websocket_failure(false, Some("transport_error"), true, true),
            WebSocketFailureDecision::FallbackToSse
        );
        assert_eq!(
            classify_websocket_failure(true, Some("transport_error"), true, true),
            WebSocketFailureDecision::Propagate
        );
    }

    #[test]
    #[allow(
        clippy::too_many_lines,
        reason = "the cache regression keeps account isolation and cleanup assertions together"
    )]
    fn session_cache_scopes_continuations_by_account_and_cleans_fallback_state() {
        let mut cache = WebSocketSessionCache::default();
        let first = WebSocketSessionKey {
            session_id: "session-1".into(),
            account_id: "account-a".into(),
        };
        let second = WebSocketSessionKey {
            session_id: "session-1".into(),
            account_id: "account-b".into(),
        };
        cache.store_continuation(
            first.clone(),
            WebSocketContinuation {
                last_response_id: Some("resp-a".into()),
            },
        );
        cache.store_continuation(
            second.clone(),
            WebSocketContinuation {
                last_response_id: Some("resp-b".into()),
            },
        );
        cache.mark_sse_fallback("session-1");
        assert_eq!(
            cache
                .continuation(&first)
                .unwrap()
                .last_response_id
                .as_deref(),
            Some("resp-a")
        );
        assert_eq!(
            cache
                .continuation(&second)
                .unwrap()
                .last_response_id
                .as_deref(),
            Some("resp-b")
        );
        assert!(cache.is_sse_fallback_active("session-1"));
        cache.clear_session("session-1");
        assert!(cache.continuation(&first).is_none());
        assert!(cache.continuation(&second).is_none());
        assert!(!cache.is_sse_fallback_active("session-1"));
    }

    struct FakeSocket {
        messages: std::collections::VecDeque<String>,
        sent: Arc<std::sync::Mutex<Vec<String>>>,
        closed: Arc<std::sync::atomic::AtomicBool>,
    }

    #[async_trait::async_trait]
    impl CodexWebSocket for FakeSocket {
        async fn send_text(&mut self, text: String) -> Result<(), StreamError> {
            self.sent.lock().unwrap().push(text);
            Ok(())
        }

        async fn next_text(&mut self) -> Result<Option<String>, StreamError> {
            Ok(self.messages.pop_front())
        }

        async fn close(&mut self) {
            self.closed
                .store(true, std::sync::atomic::Ordering::Release);
        }
    }

    struct FakeConnector {
        socket: std::sync::Mutex<Option<FakeSocket>>,
        url: std::sync::Mutex<Option<String>>,
        headers: std::sync::Mutex<Option<HashMap<String, String>>>,
    }

    #[async_trait::async_trait]
    impl CodexWebSocketConnector for FakeConnector {
        async fn connect(
            &self,
            url: String,
            headers: HashMap<String, String>,
            _timeout_ms: Option<u64>,
        ) -> Result<Box<dyn CodexWebSocket>, StreamError> {
            *self.url.lock().unwrap() = Some(url);
            *self.headers.lock().unwrap() = Some(headers);
            Ok(Box::new(self.socket.lock().unwrap().take().unwrap()))
        }
    }

    #[tokio::test]
    #[allow(
        clippy::too_many_lines,
        reason = "the adapter regression verifies URL, headers, frame, decoding, and cleanup together"
    )]
    async fn adapter_owns_codex_frame_receive_and_close_boundary() {
        use futures::StreamExt;

        let sent = Arc::new(std::sync::Mutex::new(Vec::new()));
        let closed = Arc::new(std::sync::atomic::AtomicBool::new(false));
        let connector = Arc::new(FakeConnector {
            socket: std::sync::Mutex::new(Some(FakeSocket {
                messages: [
                    r#"{"type":"response.created","response":{"id":"r1"}}"#.into(),
                    r#"{"type":"response.output_text.delta","delta":"hello"}"#.into(),
                    r#"{"type":"response.completed","response":{"status":"completed"}}"#.into(),
                ]
                .into_iter()
                .collect(),
                sent: sent.clone(),
                closed: closed.clone(),
            })),
            url: std::sync::Mutex::new(None),
            headers: std::sync::Mutex::new(None),
        });
        let adapter = CodexWebSocketAdapter::new(
            connector.clone(),
            Arc::new(|_, _, _| Ok(serde_json::json!({"model":"test-model"}))),
            Some("https://example.test/codex".into()),
            HashMap::new(),
            None,
        );
        let mut stream = adapter
            .stream_websocket(&Model::default(), &AgentContext::default(), None)
            .await
            .expect("Codex adapter stream");
        let events = stream.by_ref().collect::<Vec<_>>().await;
        assert!(events.iter().any(|event| matches!(
            event,
            crate::types::AssistantMessageEvent::TextDelta { delta, .. } if delta == "hello"
        )));
        assert_eq!(
            connector.url.lock().unwrap().as_deref(),
            Some("wss://example.test/codex/responses")
        );
        assert_eq!(
            connector.headers.lock().unwrap().as_ref().unwrap()["OpenAI-Beta"],
            CODEX_WEBSOCKET_BETA_HEADER
        );
        assert_eq!(sent.lock().unwrap().len(), 1);
        assert_eq!(
            serde_json::from_str::<serde_json::Value>(&sent.lock().unwrap()[0]).unwrap()["type"],
            "response.create"
        );
        assert!(closed.load(std::sync::atomic::Ordering::Acquire));
    }
}
