//! Replay provider for black-box testing.
//!
//! `ReplayProvider` cycles through a list of SSE fixtures, replaying each one
//! on successive calls. This enables deterministic black-box tests that do not
//! require live API keys or network access.
//!
//! The provider is selected when the `RUNIE_REPLAY_FIXTURES` environment variable
//! is set, or when explicitly constructed in tests.

use std::pin::Pin;
use std::sync::atomic::{AtomicUsize, Ordering};

use futures::Stream;
use runie_core::message::ChatMessage;
use runie_core::provider::Provider;
use runie_core::provider_event::{ModelError, ProviderEvent};

/// Protocol for replay fixtures.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Protocol {
    /// OpenAI-compatible SSE format.
    OpenAi,
    /// Anthropic-compatible SSE format.
    Anthropic,
}

/// A `Provider` that returns pre-recorded SSE fixtures in round-robin order.
///
/// Each call to `generate` returns the next fixture in the list, wrapping around
/// when the list is exhausted. This supports multi-turn conversation tests where
/// each turn uses a different recorded fixture.
pub struct ReplayProvider {
    fixtures: Vec<String>,
    protocol: Protocol,
    index: AtomicUsize,
}

impl ReplayProvider {
    /// Build a provider that cycles through `fixtures` (each a raw SSE string).
    pub fn new(fixtures: Vec<String>, protocol: Protocol) -> Self {
        Self {
            fixtures,
            protocol,
            index: AtomicUsize::new(0),
        }
    }

    /// Infer the protocol from fixture contents or path hints.
    ///
    /// Returns `Protocol::OpenAi` if any fixture contains OpenAI-specific markers,
    /// otherwise falls back to `Protocol::Anthropic`.
    pub fn infer_protocol(fixtures: &[String]) -> Protocol {
        for fixture in fixtures {
            // Anthropic SSE has distinct message_start/content_block_start markers.
            if fixture.contains("\"type\":\"message_start\"")
                || fixture.contains("\"type\":\"content_block_start\"")
                || fixture.contains("\"type\":\"message_delta\"")
            {
                return Protocol::Anthropic;
            }
            // OpenAI SSE uses chat.completion.chunk objects.
            if fixture.contains("\"object\":\"chat.completion.chunk\"")
                || fixture.contains("\"object\":\"chat.completion\"")
            {
                return Protocol::OpenAi;
            }
        }
        // Default to OpenAI for backwards compatibility.
        Protocol::OpenAi
    }

    /// Number of fixtures.
    pub fn fixture_count(&self) -> usize {
        self.fixtures.len()
    }
}

impl std::fmt::Debug for ReplayProvider {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ReplayProvider")
            .field("fixtures", &self.fixtures.len())
            .field("protocol", &self.protocol)
            .finish()
    }
}

impl Provider for ReplayProvider {
    fn generate(
        &self,
        _messages: Vec<ChatMessage>,
    ) -> Pin<Box<dyn Stream<Item = anyhow::Result<ProviderEvent>> + Send + '_>> {
        let idx = self.index.fetch_add(1, Ordering::SeqCst) % self.fixtures.len().max(1);
        let events = match self.fixtures.get(idx) {
            Some(fixture) => parse_fixture(fixture, self.protocol),
            None => Vec::new(),
        };
        Box::pin(futures::stream::iter(events.into_iter().map(Ok)))
    }
}

/// Parse a fixture string into `ProviderEvent`s.
///
/// If the fixture starts with `# HTTP <code>`, returns a single error event
/// for that HTTP status. Otherwise, parses as normal SSE content.
fn parse_fixture(content: &str, protocol: Protocol) -> Vec<ProviderEvent> {
    // Check for HTTP status prefix: "# HTTP 429"
    if let Some(first_line) = content.lines().next() {
        if first_line.starts_with("# HTTP ") {
            let code_str = first_line.trim_start_matches("# HTTP ").trim();
            let code: u16 = code_str.parse().unwrap_or(500);
            let message = content
                .lines()
                .nth(1)
                .map(|l| l.trim_start_matches('#').trim().to_string())
                .unwrap_or_else(|| format!("HTTP {}", code));

            let model_err = match code {
                401 | 403 => ModelError::Other(format!("HTTP {}: {}", code, message)),
                429 => ModelError::RateLimit { retry_after_secs: None },
                500 | 502 | 503 => ModelError::Other(format!("HTTP {}: {}", code, message)),
                _ => ModelError::Other(format!("HTTP {}: {}", code, message)),
            };
            return vec![ProviderEvent::Error(model_err)];
        }
    }
    // Otherwise parse as normal SSE
    match protocol {
        Protocol::OpenAi => crate::openai::stream::replay_sse(content),
        Protocol::Anthropic => crate::anthropic::replay_anthropic_sse(content),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn replay_provider_cycles_fixtures() {
        let fixtures = vec!["a".to_string(), "b".to_string()];
        let provider = ReplayProvider::new(fixtures, Protocol::OpenAi);
        let _ = provider;
    }

    #[test]
    fn infer_protocol_anthropic() {
        let fixtures = vec![r#"data: {"type":"message_start","message":{}}"#.to_string()];
        assert_eq!(ReplayProvider::infer_protocol(&fixtures), Protocol::Anthropic);
    }

    #[test]
    fn infer_protocol_openai() {
        let fixtures = vec![r#"data: {"object":"chat.completion.chunk"}"#.to_string()];
        assert_eq!(ReplayProvider::infer_protocol(&fixtures), Protocol::OpenAi);
    }

    #[test]
    fn infer_protocol_default_openai() {
        let fixtures = vec![r#"data: {"unknown":"format"}"#.to_string()];
        assert_eq!(ReplayProvider::infer_protocol(&fixtures), Protocol::OpenAi);
    }
}
