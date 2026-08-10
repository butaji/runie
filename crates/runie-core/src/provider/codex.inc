// Pure wire helpers for Pi's OpenAI Codex Responses provider.

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

#[path = "codex_socket.rs"]
mod codex_socket;
use codex_socket::{websocket_cache_key, websocket_headers, PreparedWebSocketAttempt};
pub use codex_socket::{CodexRequestBuilder, TokioCodexWebSocketConnector};

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
    async fn prepare_websocket_attempt(
        &self,
        model: &Model,
        context: &AgentContext,
        options: &Option<SimpleStreamOptions>,
    ) -> Result<PreparedWebSocketAttempt, StreamError> {
        let url = resolve_websocket_url(self.base_url.as_deref()).map_err(StreamError::Invalid)?;
        let body = (self.request_builder)(model, context, options.as_ref())
            .map_err(StreamError::Invalid)?;
        let cache_key = websocket_cache_key(options);
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
        let headers = websocket_headers(&self.headers, options);
        Ok(PreparedWebSocketAttempt {
            url,
            frame: frame.to_string(),
            headers,
            cache_key,
            timeout_ms: options
                .as_ref()
                .and_then(|value| value.websocket_connect_timeout_ms),
        })
    }

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

    /// Construct the live Codex adapter. Transport remains provider-owned;
    /// callers may supply an ordinary stream fallback for pre-stream failures.
    pub fn production(fallback: Option<Arc<dyn StreamFn>>) -> Self {
        Self::new(
            Arc::new(TokioCodexWebSocketConnector),
            Arc::new(|model, context, _options| {
                Ok(serde_json::json!({
                    "model": model.id,
                    "input": serde_json::to_value(&context.messages)
                        .map_err(|error| format!("serialize Codex input: {error}"))?,
                    "instructions": context.system_prompt,
                }))
            }),
            None,
            HashMap::new(),
            fallback,
        )
    }

    async fn mark_fallback(&self, cache_key: Option<&WebSocketSessionKey>) {
        if let Some(key) = cache_key {
            self.cache.lock().await.mark_sse_fallback(&key.session_id);
        }
    }

    /// Remove continuation and fallback state for one provider session.
    pub async fn clear_session(&self, session_id: &str) {
        self.cache.lock().await.clear_session(session_id);
    }

    /// Clear all provider-owned continuation and fallback state.
    pub async fn clear(&self) {
        self.cache.lock().await.clear();
    }
}

#[allow(
    clippy::too_many_lines,
    clippy::cognitive_complexity,
    reason = "the provider adapter keeps socket lifecycle and decoder settlement in one boundary"
)]
impl CodexWebSocketAdapter {
    async fn stream_websocket_attempt(
        &self,
        model: &Model,
        context: &AgentContext,
        options: Option<SimpleStreamOptions>,
        retried_connection_limit: bool,
        retried_missing_continuation: bool,
    ) -> Result<AssistantMessageEventStream, StreamError> {
        let prepared = self
            .prepare_websocket_attempt(model, context, &options)
            .await?;
        let cache_key = prepared.cache_key.clone();
        let fallback_active = if let Some(key) = cache_key.as_ref() {
            self.cache
                .lock()
                .await
                .is_sse_fallback_active(&key.session_id)
        } else {
            false
        };
        if fallback_active {
            if let Some(fallback) = &self.fallback {
                return fallback.stream(model, context, options).await;
            }
        }
        let mut socket = match self
            .connector
            .connect(prepared.url, prepared.headers, prepared.timeout_ms)
            .await
        {
            Ok(socket) => socket,
            Err(error) => {
                if let Some(fallback) = &self.fallback {
                    self.mark_fallback(cache_key.as_ref()).await;
                    return fallback.stream(model, context, options).await;
                }
                return Err(error);
            }
        };
        let send_result = socket.send_text(prepared.frame).await;
        if let Err(error) = send_result {
            socket.close().await;
            if let Some(fallback) = &self.fallback {
                self.mark_fallback(cache_key.as_ref()).await;
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
                            self.mark_fallback(cache_key.as_ref()).await;
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
                                    self.mark_fallback(cache_key.as_ref()).await;
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
                                self.mark_fallback(cache_key.as_ref()).await;
                                return fallback.stream(model, context, options).await;
                            }
                        }
                        return Err(StreamError::Invalid(
                            "Codex WebSocket message must be a JSON object".into(),
                        ));
                    }
                    let message_type = value.get("type").and_then(|value| value.as_str());
                    if message_type == Some("error")
                        && !started
                        && !retried_connection_limit
                        && value
                            .pointer("/error/code")
                            .and_then(|value| value.as_str())
                            == Some("websocket_connection_limit_reached")
                    {
                        socket.close().await;
                        return Box::pin(self.stream_websocket_attempt(
                            model,
                            context,
                            options.clone(),
                            true,
                            retried_missing_continuation,
                        ))
                        .await;
                    }
                    if message_type == Some("error")
                        && !retried_missing_continuation
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
                        return Box::pin(self.stream_websocket_attempt(
                            model,
                            context,
                            options.clone(),
                            retried_connection_limit,
                            true,
                        ))
                        .await;
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
                self.mark_fallback(cache_key.as_ref()).await;
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

#[async_trait::async_trait]
impl WebSocketAdapter for CodexWebSocketAdapter {
    async fn stream_websocket(
        &self,
        model: &Model,
        context: &AgentContext,
        options: Option<SimpleStreamOptions>,
    ) -> Result<AssistantMessageEventStream, StreamError> {
        self.stream_websocket_attempt(model, context, options, false, false)
            .await
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
#[path = "codex_tests.rs"]
mod tests;
#[cfg(test)]
#[path = "codex_tests_extra.rs"]
mod tests_extra;
