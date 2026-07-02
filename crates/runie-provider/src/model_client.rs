//! `ModelClient` — long-lived HTTP/WebSocket client for LLM providers.
//!
//! A `ModelClient` owns a shared `reqwest::Client` with connection pooling and
//! the credentials for a provider+model pair. Multiple `TurnSession`s share one
//! `ModelClient` so HTTP/WS connections are reused across turns.

use std::sync::Arc;

/// HTTP transport configuration for a model client.
#[derive(Clone)]
pub struct ModelClientTransport {
    /// Base URL for the API endpoint (e.g. "https://api.openai.com/v1").
    pub base_url: String,
    /// Optional WebSocket URL for real-time endpoints.
    pub ws_url: Option<String>,
}

/// A long-lived model client holding auth credentials and a shared HTTP client.
///
/// The client is built once and reused across all turns, enabling HTTP connection
/// pooling (HTTP/2 multiplexing, keep-alive, and TCP connection reuse).
#[derive(Clone)]
pub struct ModelClient {
    /// The HTTP client with connection pooling.
    pub client: Arc<reqwest::Client>,
    /// The API key for authentication.
    pub api_key: String,
    /// Model name (e.g. "gpt-4o").
    pub model: String,
    /// Provider registry key (e.g. "openai").
    pub provider_key: String,
    /// Transport configuration (base URL, optional WS URL).
    pub transport: ModelClientTransport,
}

impl ModelClient {
    /// Create a new `ModelClient` with a pooled HTTP client.
    pub fn new(api_key: String, model: String, provider_key: String) -> Self {
        Self {
            client: crate::http::build_client(),
            api_key,
            model,
            provider_key,
            transport: ModelClientTransport {
                base_url: "https://api.openai.com/v1".to_owned(),
                ws_url: None,
            },
        }
    }

    /// Set the base URL.
    pub fn with_base_url(mut self, url: impl Into<String>) -> Self {
        self.transport.base_url = crate::http::normalize_base_url(&url.into());
        self
    }

    /// Set the WebSocket URL.
    pub fn with_ws_url(mut self, url: impl Into<String>) -> Self {
        self.transport.ws_url = Some(url.into());
        self
    }

    /// Returns the base URL.
    pub fn base_url(&self) -> &str {
        &self.transport.base_url
    }
}

/// A per-turn session holding turn-specific state.
///
/// Each turn creates a `TurnSession` that borrows the shared `ModelClient`'s
/// HTTP client for its requests. Turn sessions are cheap to create and hold
/// turn-local state like message queues and token counters.
#[derive(Clone)]
pub struct TurnSession {
    /// Reference to the shared model client (shareable across turns).
    pub client: ModelClient,
    /// Messages accumulated in this turn.
    pub messages: Vec<runie_core::proto::message::ChatMessage>,
    /// Token counters for this turn.
    pub tokens_in: usize,
    /// Tokens generated in this turn.
    pub tokens_out: usize,
    /// Whether the model is currently streaming a response.
    pub streaming: bool,
}

impl TurnSession {
    /// Create a new turn session from a model client.
    pub fn new(client: ModelClient) -> Self {
        Self {
            client,
            messages: Vec::new(),
            tokens_in: 0,
            tokens_out: 0,
            streaming: false,
        }
    }

    /// Start a streaming response.
    pub fn start_streaming(&mut self) {
        self.streaming = true;
    }

    /// Stop streaming.
    pub fn stop_streaming(&mut self) {
        self.streaming = false;
    }

    /// Add a message to the turn's message history.
    pub fn push_message(&mut self, msg: runie_core::proto::message::ChatMessage) {
        self.messages.push(msg);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn model_client_shares_client_across_clones() {
        let client1 = ModelClient::new("sk-test".into(), "gpt-4o".into(), "openai".into());
        let client2 = client1.clone();
        // Arc should be the same
        assert!(Arc::ptr_eq(&client1.client, &client2.client));
    }

    #[test]
    fn model_client_base_url_normalizes_trailing_slash() {
        let client =
            ModelClient::new("sk-test".into(), "gpt-4o".into(), "openai".into())
                .with_base_url("https://api.example.com/v1/");
        assert_eq!(client.base_url(), "https://api.example.com/v1");
    }

    #[test]
    fn turn_session_tracks_streaming_state() {
        let mc = ModelClient::new("sk-test".into(), "gpt-4o".into(), "openai".into());
        let mut session = TurnSession::new(mc);
        assert!(!session.streaming);
        session.start_streaming();
        assert!(session.streaming);
        session.stop_streaming();
        assert!(!session.streaming);
    }

    #[test]
    fn turn_session_collects_messages() {
        use runie_core::proto::message::ChatMessage;
        let mc = ModelClient::new("sk-test".into(), "gpt-4o".into(), "openai".into());
        let mut session = TurnSession::new(mc);
        session.push_message(ChatMessage::user(String::from("hello")));
        assert_eq!(session.messages.len(), 1);
    }
}
