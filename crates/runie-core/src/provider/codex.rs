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

pub struct TokioCodexWebSocketConnector;

struct TokioCodexWebSocket {
    socket: tokio_tungstenite::WebSocketStream<
        tokio_tungstenite::MaybeTlsStream<tokio::net::TcpStream>,
    >,
}

#[async_trait::async_trait]
impl CodexWebSocket for TokioCodexWebSocket {
    async fn send_text(&mut self, text: String) -> Result<(), StreamError> {
        use futures::SinkExt;
        self.socket
            .send(tokio_tungstenite::tungstenite::Message::Text(text))
            .await
            .map_err(|error| StreamError::Network(format!("Codex WebSocket send failed: {error}")))
    }

    async fn next_text(&mut self) -> Result<Option<String>, StreamError> {
        use futures::{SinkExt, StreamExt};
        while let Some(message) = self.socket.next().await {
            let message = message.map_err(|error| {
                StreamError::Network(format!("Codex WebSocket receive failed: {error}"))
            })?;
            match message {
                tokio_tungstenite::tungstenite::Message::Text(text) => return Ok(Some(text)),
                tokio_tungstenite::tungstenite::Message::Binary(_) => {
                    return Err(StreamError::Invalid(
                        "Codex WebSocket message must be text".into(),
                    ));
                }
                tokio_tungstenite::tungstenite::Message::Ping(payload) => {
                    self.socket
                        .send(tokio_tungstenite::tungstenite::Message::Pong(payload))
                        .await
                        .map_err(|error| {
                            StreamError::Network(format!("Codex WebSocket pong failed: {error}"))
                        })?;
                }
                tokio_tungstenite::tungstenite::Message::Pong(_) => {}
                tokio_tungstenite::tungstenite::Message::Close(_) => return Ok(None),
                tokio_tungstenite::tungstenite::Message::Frame(_) => {}
            }
        }
        Ok(None)
    }

    async fn close(&mut self) {
        let _ = self.socket.close(None).await;
    }
}

#[async_trait::async_trait]
impl CodexWebSocketConnector for TokioCodexWebSocketConnector {
    async fn connect(
        &self,
        url: String,
        headers: HashMap<String, String>,
        timeout_ms: Option<u64>,
    ) -> Result<Box<dyn CodexWebSocket>, StreamError> {
        let mut request = tokio_tungstenite::tungstenite::http::Request::builder()
            .uri(&url)
            .body(())
            .map_err(|error| {
                StreamError::Invalid(format!("invalid Codex WebSocket URL: {error}"))
            })?;
        for (name, value) in headers {
            let header_name = tokio_tungstenite::tungstenite::http::header::HeaderName::try_from(
                name,
            )
            .map_err(|error| StreamError::Invalid(format!("invalid Codex header name: {error}")))?;
            let header_value =
                tokio_tungstenite::tungstenite::http::header::HeaderValue::try_from(value)
                    .map_err(|error| {
                        StreamError::Invalid(format!("invalid Codex header value: {error}"))
                    })?;
            request.headers_mut().insert(header_name, header_value);
        }
        let connect = tokio_tungstenite::connect_async(request);
        let (socket, _) = if let Some(timeout_ms) = timeout_ms {
            tokio::time::timeout(std::time::Duration::from_millis(timeout_ms), connect)
                .await
                .map_err(|_| {
                    StreamError::Network(format!(
                        "Codex WebSocket connect timed out after {timeout_ms}ms"
                    ))
                })?
                .map_err(|error| {
                    StreamError::Network(format!("Codex WebSocket connect failed: {error}"))
                })?
        } else {
            connect.await.map_err(|error| {
                StreamError::Network(format!("Codex WebSocket connect failed: {error}"))
            })?
        };
        Ok(Box::new(TokioCodexWebSocket { socket }))
    }
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
    cache: Arc<tokio::sync::Mutex<WebSocketSessionCache>>,
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
            cache: Arc::new(tokio::sync::Mutex::new(WebSocketSessionCache::default())),
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
        let cache_key = options.as_ref().and_then(|value| {
            value
                .session_id
                .as_ref()
                .map(|session_id| WebSocketSessionKey {
                    session_id: session_id.clone(),
                    account_id: value.api_key.clone().unwrap_or_else(|| "default".into()),
                })
        });
        let continuation = if options
            .as_ref()
            .and_then(|value| value.transport)
            .is_some_and(|transport| {
                matches!(
                    transport,
                    crate::types::ProviderTransport::WebsocketCached
                        | crate::types::ProviderTransport::Auto
                )
            }) {
            let cache = self.cache.lock().await;
            cache_key
                .as_ref()
                .and_then(|key| cache.continuation(key).cloned())
        } else {
            None
        };
        let frame = response_create_continuation_frame(body, continuation.as_ref())
            .map_err(StreamError::Invalid)?;
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
                    if !started {
                        if let Some(fallback) = &self.fallback {
                            return fallback.stream(model, context, options).await;
                        }
                    }
                    return Err(error);
                }
                Ok(Some(message)) => {
                    let value = match serde_json::from_str::<serde_json::Value>(&message) {
                        Ok(value) => value,
                        Err(error) => {
                            socket.close().await;
                            if !started {
                                if let Some(fallback) = &self.fallback {
                                    return fallback.stream(model, context, options).await;
                                }
                            }
                            return Err(StreamError::Invalid(format!(
                                "invalid Codex WebSocket JSON: {error}"
                            )));
                        }
                    };
                    if !value.is_object() {
                        socket.close().await;
                        if !started {
                            if let Some(fallback) = &self.fallback {
                                return fallback.stream(model, context, options).await;
                            }
                        }
                        return Err(StreamError::Invalid(
                            "Codex WebSocket message must be a JSON object".into(),
                        ));
                    }
                    let message_type = value.get("type").and_then(|value| value.as_str());
                    if message_type == Some("error")
                        && continuation.is_some()
                        && value
                            .pointer("/error/code")
                            .and_then(|value| value.as_str())
                            == Some("previous_response_not_found")
                    {
                        // One bounded continuation retry: removing the
                        // stale response ID makes the recursive attempt a
                        // fresh request, so a repeated provider error cannot
                        // recurse indefinitely.
                        socket.close().await;
                        if let Some(key) = &cache_key {
                            self.cache.lock().await.continuations.remove(key);
                        }
                        return self.stream_websocket(model, context, options.clone()).await;
                    }
                    // Pi treats an initial provider `error` envelope as a
                    // transport/setup failure. It must remain pre-stream so
                    // the adapter can apply its explicit fallback policy;
                    // marking it started would turn the same wire fact into
                    // an unrecoverable post-stream error.
                    started |= message_type != Some("error");
                    let terminal = matches!(
                        message_type,
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
            if let Some(fallback) = &self.fallback {
                return fallback.stream(model, context, options).await;
            }
            return Err(StreamError::Network(
                "Codex WebSocket closed before streaming".into(),
            ));
        }
        if let Some(key) = cache_key {
            if let Some(response_id) = response_id_from_messages(&messages) {
                self.cache.lock().await.store_continuation(
                    key,
                    WebSocketContinuation {
                        last_response_id: Some(response_id),
                    },
                );
            }
        }
        let provider = crate::provider::replay::ReplayProvider::from_websocket_messages(messages)?;
        provider.stream(model, context, options).await
    }
}

fn response_id_from_messages(messages: &[String]) -> Option<String> {
    messages.iter().rev().find_map(|message| {
        let value = serde_json::from_str::<serde_json::Value>(message).ok()?;
        value
            .pointer("/response/id")
            .and_then(|value| value.as_str())
            .map(str::to_owned)
    })
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
        socket: std::sync::Mutex<std::collections::VecDeque<FakeSocket>>,
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
            Ok(Box::new(self.socket.lock().unwrap().pop_front().unwrap()))
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
            socket: std::sync::Mutex::new(
                [
                    FakeSocket {
                        messages: [
                            r#"{"type":"response.created","response":{"id":"r1"}}"#.into(),
                            r#"{"type":"response.output_text.delta","delta":"hello"}"#.into(),
                            r#"{"type":"response.completed","response":{"status":"completed"}}"#
                                .into(),
                        ]
                        .into_iter()
                        .collect(),
                        sent: sent.clone(),
                        closed: closed.clone(),
                    },
                    FakeSocket {
                        messages: [
                            r#"{"type":"response.created","response":{"id":"r2"}}"#.into(),
                            r#"{"type":"response.completed","response":{"status":"completed"}}"#
                                .into(),
                        ]
                        .into_iter()
                        .collect(),
                        sent: sent.clone(),
                        closed: closed.clone(),
                    },
                ]
                .into_iter()
                .collect(),
            ),
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
        let options = Some(SimpleStreamOptions {
            session_id: Some("session-1".into()),
            api_key: Some("account-a".into()),
            transport: Some(crate::types::ProviderTransport::WebsocketCached),
            ..Default::default()
        });
        let mut stream = adapter
            .stream_websocket(&Model::default(), &AgentContext::default(), options.clone())
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
        let mut cached_stream = adapter
            .stream_websocket(&Model::default(), &AgentContext::default(), options)
            .await
            .expect("cached Codex adapter stream");
        let _ = cached_stream.by_ref().collect::<Vec<_>>().await;
        assert_eq!(sent.lock().unwrap().len(), 2);
        assert_eq!(
            serde_json::from_str::<serde_json::Value>(&sent.lock().unwrap()[1]).unwrap()
                ["previous_response_id"],
            "r1"
        );
        assert!(closed.load(std::sync::atomic::Ordering::Acquire));
    }

    #[tokio::test]
    async fn adapter_falls_back_for_initial_error_before_marking_stream_started() {
        use futures::StreamExt;

        let sent = Arc::new(std::sync::Mutex::new(Vec::new()));
        let closed = Arc::new(std::sync::atomic::AtomicBool::new(false));
        let connector = Arc::new(FakeConnector {
            socket: std::sync::Mutex::new(
                [FakeSocket {
                    messages: [
                        r#"{"type":"error","error":{"code":"rate_limit","message":"busy"}}"#.into(),
                    ]
                    .into_iter()
                    .collect(),
                    sent: sent.clone(),
                    closed: closed.clone(),
                }]
                .into_iter()
                .collect(),
            ),
            url: std::sync::Mutex::new(None),
            headers: std::sync::Mutex::new(None),
        });
        let fallback = Arc::new(
            crate::provider::replay::ReplayProvider::from_websocket_messages([
                r#"{"type":"response.created","response":{"id":"fallback"}}"#,
                r#"{"type":"response.completed","response":{"status":"completed"}}"#,
            ])
            .expect("fallback replay provider"),
        );
        let adapter = CodexWebSocketAdapter::new(
            connector,
            Arc::new(|_, _, _| Ok(serde_json::json!({"model":"test-model"}))),
            Some("https://example.test/codex".into()),
            HashMap::new(),
            Some(fallback),
        );

        let mut stream = adapter
            .stream_websocket(&Model::default(), &AgentContext::default(), None)
            .await
            .expect("initial provider error falls back");
        assert!(stream.next().await.is_some());
        assert!(closed.load(std::sync::atomic::Ordering::Acquire));
    }

    #[tokio::test]
    #[allow(
        clippy::too_many_lines,
        reason = "the retry regression keeps both owned sockets and envelope assertions together"
    )]
    async fn adapter_retries_stale_cached_continuation_once_as_fresh_request() {
        use futures::StreamExt;

        let sent = Arc::new(std::sync::Mutex::new(Vec::new()));
        let closed = Arc::new(std::sync::atomic::AtomicBool::new(false));
        let connector = Arc::new(FakeConnector {
            socket: std::sync::Mutex::new(
                [
                    FakeSocket {
                        messages: [
                            r#"{"type":"error","error":{"code":"previous_response_not_found"}}"#
                                .into(),
                        ]
                        .into_iter()
                        .collect(),
                        sent: sent.clone(),
                        closed: closed.clone(),
                    },
                    FakeSocket {
                        messages: [
                            r#"{"type":"response.created","response":{"id":"fresh"}}"#.into(),
                            r#"{"type":"response.completed","response":{"status":"completed"}}"#
                                .into(),
                        ]
                        .into_iter()
                        .collect(),
                        sent: sent.clone(),
                        closed: closed.clone(),
                    },
                ]
                .into_iter()
                .collect(),
            ),
            url: std::sync::Mutex::new(None),
            headers: std::sync::Mutex::new(None),
        });
        let adapter = CodexWebSocketAdapter::new(
            connector,
            Arc::new(|_, _, _| Ok(serde_json::json!({"model":"test-model"}))),
            Some("https://example.test/codex".into()),
            HashMap::new(),
            None,
        );
        let key = WebSocketSessionKey {
            session_id: "session-1".into(),
            account_id: "account-a".into(),
        };
        adapter.cache.lock().await.store_continuation(
            key,
            WebSocketContinuation {
                last_response_id: Some("stale".into()),
            },
        );
        let options = Some(SimpleStreamOptions {
            session_id: Some("session-1".into()),
            api_key: Some("account-a".into()),
            transport: Some(crate::types::ProviderTransport::WebsocketCached),
            ..Default::default()
        });
        let mut stream = adapter
            .stream_websocket(&Model::default(), &AgentContext::default(), options)
            .await
            .expect("fresh continuation retry");
        let _ = stream.by_ref().collect::<Vec<_>>().await;
        let sent = sent.lock().unwrap();
        assert_eq!(sent.len(), 2);
        assert_eq!(
            serde_json::from_str::<serde_json::Value>(&sent[0]).unwrap()["previous_response_id"],
            "stale"
        );
        assert!(serde_json::from_str::<serde_json::Value>(&sent[1])
            .unwrap()
            .get("previous_response_id")
            .is_none());
        assert!(closed.load(std::sync::atomic::Ordering::Acquire));
    }
}
