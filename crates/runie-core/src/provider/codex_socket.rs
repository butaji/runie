use super::*;

pub(super) struct PreparedWebSocketAttempt {
    pub(super) url: String,
    pub(super) frame: String,
    pub(super) headers: HashMap<String, String>,
    pub(super) cache_key: Option<WebSocketSessionKey>,
    pub(super) timeout_ms: Option<u64>,
}

pub(super) fn websocket_cache_key(
    options: &Option<SimpleStreamOptions>,
) -> Option<WebSocketSessionKey> {
    options.as_ref().and_then(|value| {
        value
            .session_id
            .as_ref()
            .map(|session_id| WebSocketSessionKey {
                session_id: session_id.clone(),
                account_id: value.api_key.clone().unwrap_or_else(|| "default".into()),
            })
    })
}

pub(super) fn websocket_headers(
    base: &HashMap<String, String>,
    options: &Option<SimpleStreamOptions>,
) -> HashMap<String, String> {
    let mut headers = base.clone();
    headers.insert("OpenAI-Beta".into(), CODEX_WEBSOCKET_BETA_HEADER.into());
    if let Some(value) = options.as_ref() {
        if let Some(api_key) = &value.api_key {
            headers.insert("Authorization".into(), format!("Bearer {api_key}"));
        }
        if let Some(extra) = &value.headers {
            headers.extend(extra.clone());
        }
    }
    headers
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
        let request = codex_websocket_request(&url, headers)?;
        let connect = tokio_tungstenite::connect_async(request);
        let (socket, _) = await_codex_connection(connect, timeout_ms).await?;
        Ok(Box::new(TokioCodexWebSocket { socket }))
    }
}

fn codex_websocket_request(
    url: &str,
    headers: HashMap<String, String>,
) -> Result<tokio_tungstenite::tungstenite::http::Request<()>, StreamError> {
    let mut request = tokio_tungstenite::tungstenite::http::Request::builder()
        .uri(url)
        .body(())
        .map_err(|error| StreamError::Invalid(format!("invalid Codex WebSocket URL: {error}")))?;
    for (name, value) in headers {
        let header_name = tokio_tungstenite::tungstenite::http::header::HeaderName::try_from(name)
            .map_err(|error| StreamError::Invalid(format!("invalid Codex header name: {error}")))?;
        let header_value = tokio_tungstenite::tungstenite::http::header::HeaderValue::try_from(
            value,
        )
        .map_err(|error| StreamError::Invalid(format!("invalid Codex header value: {error}")))?;
        request.headers_mut().insert(header_name, header_value);
    }
    Ok(request)
}

type CodexConnection = (
    tokio_tungstenite::WebSocketStream<tokio_tungstenite::MaybeTlsStream<tokio::net::TcpStream>>,
    tokio_tungstenite::tungstenite::handshake::client::Response,
);

async fn await_codex_connection<F>(
    connect: F,
    timeout_ms: Option<u64>,
) -> Result<CodexConnection, StreamError>
where
    F: std::future::Future<Output = Result<CodexConnection, tokio_tungstenite::tungstenite::Error>>,
{
    if let Some(timeout_ms) = timeout_ms {
        Ok(
            tokio::time::timeout(std::time::Duration::from_millis(timeout_ms), connect)
                .await
                .map_err(|_| {
                    StreamError::Network(format!(
                        "Codex WebSocket connect timed out after {timeout_ms}ms"
                    ))
                })?
                .map_err(|error| {
                    StreamError::Network(format!("Codex WebSocket connect failed: {error}"))
                })?,
        )
    } else {
        Ok(connect.await.map_err(|error| {
            StreamError::Network(format!("Codex WebSocket connect failed: {error}"))
        })?)
    }
}

pub type CodexRequestBuilder = Arc<
    dyn Fn(&Model, &AgentContext, Option<&SimpleStreamOptions>) -> Result<serde_json::Value, String>
        + Send
        + Sync,
>;
