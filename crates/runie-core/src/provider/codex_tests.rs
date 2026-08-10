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
fn session_cache_scopes_continuations_by_account_and_cleans_fallback_state() {
    let (mut cache, first, second) = cache_fixture();
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

fn cache_fixture() -> (
    WebSocketSessionCache,
    WebSocketSessionKey,
    WebSocketSessionKey,
) {
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
    (cache, first, second)
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

    let (connector, sent, closed) = adapter_socket_fixture();
    let adapter = CodexWebSocketAdapter::new(
        connector.clone(),
        Arc::new(|_, _, _| Ok(serde_json::json!({"model":"test-model"}))),
        Some("https://example.test/codex".into()),
        HashMap::new(),
        None,
    );
    let options = adapter_options();
    let mut stream = adapter
        .stream_websocket(&Model::default(), &AgentContext::default(), options.clone())
        .await
        .expect("Codex adapter stream");
    let events = stream.by_ref().collect::<Vec<_>>().await;
    assert!(events.iter().any(|event| matches!(
        event,
        crate::types::AssistantMessageEvent::TextDelta { delta, .. } if delta == "hello"
    )));
    assert_adapter_connection(&connector, &sent);
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

fn assert_adapter_connection(connector: &FakeConnector, sent: &Arc<std::sync::Mutex<Vec<String>>>) {
    assert_eq!(
        connector.url.lock().unwrap().as_deref(),
        Some("wss://example.test/codex/responses")
    );
    assert_eq!(
        connector.headers.lock().unwrap().as_ref().unwrap()["OpenAI-Beta"],
        CODEX_WEBSOCKET_BETA_HEADER
    );
    assert_eq!(sent.lock().unwrap().len(), 1);
}

fn adapter_options() -> Option<SimpleStreamOptions> {
    Some(SimpleStreamOptions {
        session_id: Some("session-1".into()),
        api_key: Some("account-a".into()),
        transport: Some(crate::types::ProviderTransport::WebsocketCached),
        ..Default::default()
    })
}

fn adapter_socket_fixture() -> (
    Arc<FakeConnector>,
    Arc<std::sync::Mutex<Vec<String>>>,
    Arc<std::sync::atomic::AtomicBool>,
) {
    let sent = Arc::new(std::sync::Mutex::new(Vec::new()));
    let closed = Arc::new(std::sync::atomic::AtomicBool::new(false));
    let connector = Arc::new(FakeConnector {
        socket: std::sync::Mutex::new(adapter_sockets(sent.clone(), closed.clone())),
        url: std::sync::Mutex::new(None),
        headers: std::sync::Mutex::new(None),
    });
    (connector, sent, closed)
}

fn adapter_sockets(
    sent: Arc<std::sync::Mutex<Vec<String>>>,
    closed: Arc<std::sync::atomic::AtomicBool>,
) -> std::collections::VecDeque<FakeSocket> {
    [
        FakeSocket {
            messages: [
                r#"{"type":"response.created","response":{"id":"r1"}}"#.into(),
                r#"{"type":"response.output_text.delta","delta":"hello"}"#.into(),
                r#"{"type":"response.completed","response":{"status":"completed"}}"#.into(),
            ]
            .into_iter()
            .collect(),
            sent: sent.clone(),
            closed: closed.clone(),
        },
        FakeSocket {
            messages: [
                r#"{"type":"response.created","response":{"id":"r2"}}"#.into(),
                r#"{"type":"response.completed","response":{"status":"completed"}}"#.into(),
            ]
            .into_iter()
            .collect(),
            sent,
            closed,
        },
    ]
    .into_iter()
    .collect()
}

#[tokio::test]
async fn adapter_falls_back_for_initial_error_before_marking_stream_started() {
    use futures::StreamExt;

    let (connector, closed) = fallback_socket_fixture();
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
    let options = SimpleStreamOptions {
        session_id: Some("fallback-session".into()),
        api_key: Some("account-a".into()),
        ..Default::default()
    };
    let mut stream = adapter
        .stream_websocket(&Model::default(), &AgentContext::default(), Some(options))
        .await
        .expect("initial provider error falls back");
    assert!(stream.next().await.is_some());
    assert_fallback_cleanup(&adapter, &closed).await;
}

async fn assert_fallback_cleanup(
    adapter: &CodexWebSocketAdapter,
    closed: &Arc<std::sync::atomic::AtomicBool>,
) {
    assert!(closed.load(std::sync::atomic::Ordering::Acquire));
    assert!(adapter
        .cache
        .lock()
        .await
        .is_sse_fallback_active("fallback-session"));
    adapter.clear_session("fallback-session").await;
    assert!(!adapter
        .cache
        .lock()
        .await
        .is_sse_fallback_active("fallback-session"));
}

fn fallback_socket_fixture() -> (Arc<FakeConnector>, Arc<std::sync::atomic::AtomicBool>) {
    let closed = Arc::new(std::sync::atomic::AtomicBool::new(false));
    let sent = Arc::new(std::sync::Mutex::new(Vec::new()));
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
    (connector, closed)
}

#[tokio::test]
async fn adapter_retries_stale_cached_continuation_once_as_fresh_request() {
    use futures::StreamExt;

    let (connector, sent, closed) = stale_retry_socket_fixture();
    let adapter = connection_limit_adapter(connector);
    let options = seed_stale_continuation(&adapter).await;
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

async fn seed_stale_continuation(adapter: &CodexWebSocketAdapter) -> Option<SimpleStreamOptions> {
    adapter.cache.lock().await.store_continuation(
        WebSocketSessionKey {
            session_id: "session-1".into(),
            account_id: "account-a".into(),
        },
        WebSocketContinuation {
            last_response_id: Some("stale".into()),
        },
    );
    Some(SimpleStreamOptions {
        session_id: Some("session-1".into()),
        api_key: Some("account-a".into()),
        transport: Some(crate::types::ProviderTransport::WebsocketCached),
        ..Default::default()
    })
}

fn stale_retry_socket_fixture() -> (
    Arc<FakeConnector>,
    Arc<std::sync::Mutex<Vec<String>>>,
    Arc<std::sync::atomic::AtomicBool>,
) {
    let sent = Arc::new(std::sync::Mutex::new(Vec::new()));
    let closed = Arc::new(std::sync::atomic::AtomicBool::new(false));
    let connector = Arc::new(FakeConnector {
        socket: std::sync::Mutex::new(
            [
                FakeSocket {
                    messages: [
                        r#"{"type":"error","error":{"code":"previous_response_not_found"}}"#.into(),
                    ]
                    .into_iter()
                    .collect(),
                    sent: sent.clone(),
                    closed: closed.clone(),
                },
                FakeSocket {
                    messages: [
                        r#"{"type":"response.created","response":{"id":"fresh"}}"#.into(),
                        r#"{"type":"response.completed","response":{"status":"completed"}}"#.into(),
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
    (connector, sent, closed)
}

#[tokio::test]
async fn adapter_retries_connection_limit_once_before_fallback_or_failure() {
    use futures::StreamExt;

    let (connector, sent, closed) = connection_limit_socket_fixture();
    let adapter = CodexWebSocketAdapter::new(
        connector,
        Arc::new(|_, _, _| Ok(serde_json::json!({"model":"test-model"}))),
        Some("https://example.test/codex".into()),
        HashMap::new(),
        None,
    );
    let mut stream = adapter
        .stream_websocket(&Model::default(), &AgentContext::default(), None)
        .await
        .expect("connection-limit retry");
    let _ = stream.by_ref().collect::<Vec<_>>().await;
    assert_eq!(sent.lock().unwrap().len(), 2);
    assert!(closed.load(std::sync::atomic::Ordering::Acquire));
}

fn connection_limit_socket_fixture() -> (
    Arc<FakeConnector>,
    Arc<std::sync::Mutex<Vec<String>>>,
    Arc<std::sync::atomic::AtomicBool>,
) {
    let sent = Arc::new(std::sync::Mutex::new(Vec::new()));
    let closed = Arc::new(std::sync::atomic::AtomicBool::new(false));
    let connector = Arc::new(FakeConnector {
        socket: std::sync::Mutex::new(connection_limit_sockets(sent.clone(), closed.clone())),
        url: std::sync::Mutex::new(None),
        headers: std::sync::Mutex::new(None),
    });
    (connector, sent, closed)
}

fn connection_limit_sockets(
    sent: Arc<std::sync::Mutex<Vec<String>>>,
    closed: Arc<std::sync::atomic::AtomicBool>,
) -> std::collections::VecDeque<FakeSocket> {
    [
        FakeSocket {
            messages: [
                r#"{"type":"error","error":{"code":"websocket_connection_limit_reached"}}"#.into(),
            ]
            .into_iter()
            .collect(),
            sent: sent.clone(),
            closed: closed.clone(),
        },
        FakeSocket {
            messages: [
                r#"{"type":"response.created","response":{"id":"retry"}}"#.into(),
                r#"{"type":"response.completed","response":{"status":"completed"}}"#.into(),
            ]
            .into_iter()
            .collect(),
            sent,
            closed,
        },
    ]
    .into_iter()
    .collect()
}

fn connection_limit_adapter(connector: Arc<FakeConnector>) -> CodexWebSocketAdapter {
    CodexWebSocketAdapter::new(
        connector,
        Arc::new(|_, _, _| Ok(serde_json::json!({"model":"test-model"}))),
        Some("https://example.test/codex".into()),
        HashMap::new(),
        None,
    )
}
