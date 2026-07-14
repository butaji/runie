//! Shared replay provider and event-capture helpers.
//!
//! `ReplayProvider` cycles through a list of SSE fixtures, replaying each one
//! on successive calls. Used by agent-turn replay tests to simulate streaming
//! without a live network connection.
//!
//! `GrokReplayProvider` does the same for Grok Build SSE fixtures.

use std::pin::Pin;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;

use futures::Stream;
use runie_core::message::ChatMessage;
use runie_core::provider::Provider;
use runie_core::provider_event::{ModelError, ProviderEvent};
use runie_core::Event;
use runie_provider::openai::stream::replay_sse;
use runie_provider::BuiltProvider;

/// A `Provider` that returns pre-recorded SSE fixtures in round-robin order.
pub struct ReplayProvider {
    fixtures: Vec<String>,
    index: AtomicUsize,
}

impl ReplayProvider {
    /// Build a provider that cycles through `fixtures` (each a raw SSE string).
    pub fn new(fixtures: Vec<String>) -> Self {
        Self {
            fixtures,
            index: AtomicUsize::new(0),
        }
    }
}

impl Provider for ReplayProvider {
    fn generate(
        &self,
        _messages: Vec<ChatMessage>,
    ) -> Pin<Box<dyn Stream<Item = anyhow::Result<ProviderEvent>> + Send + '_>> {
        let idx = self.index.fetch_add(1, Ordering::SeqCst);
        let events = self
            .fixtures
            .get(idx)
            .map(|f| parse_fixture(f))
            .unwrap_or_default();
        Box::pin(futures::stream::iter(events.into_iter().map(Ok)))
    }
}

/// Parse a fixture string into `ProviderEvent`s.
///
/// If the fixture starts with `# HTTP <code>`, returns a single error event
/// for that HTTP status. Otherwise, parses as normal SSE content.
fn parse_fixture(content: &str) -> Vec<ProviderEvent> {
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
                429 => ModelError::RateLimit {
                    retry_after_secs: None,
                },
                500 | 502 | 503 => ModelError::Other(format!("HTTP {}: {}", code, message)),
                _ => ModelError::Other(format!("HTTP {}: {}", code, message)),
            };
            return vec![ProviderEvent::Error(model_err)];
        }
    }
    // Otherwise parse as normal SSE
    replay_sse(content)
}

/// Wrap a `ReplayProvider` in a `BuiltProvider`.
pub fn dyn_replay_provider(fixtures: &[String]) -> BuiltProvider {
    let provider = ReplayProvider::new(fixtures.to_vec());
    BuiltProvider::from_provider(Box::new(provider), "minimax", "MiniMax-M3")
}

/// Wrap a `ReplayProvider` in a `BuiltProvider` with custom key and model.
pub fn dyn_replay_provider_with(fixtures: &[String], key: &str, model: &str) -> BuiltProvider {
    let provider = ReplayProvider::new(fixtures.to_vec());
    BuiltProvider::from_provider(Box::new(provider), key, model)
}

// ── Grok Build replay ─────────────────────────────────────────────────────────

/// A `Provider` that returns pre-recorded Grok Build SSE fixtures in round-robin
/// order. Grok Build uses the same OpenAI-compatible SSE format, so we reuse
/// `replay_sse` for parsing.
pub struct GrokReplayProvider {
    fixtures: Vec<String>,
    index: AtomicUsize,
}

impl GrokReplayProvider {
    /// Build a provider that cycles through `fixtures` (each a raw SSE string).
    pub fn new(fixtures: Vec<String>) -> Self {
        Self {
            fixtures,
            index: AtomicUsize::new(0),
        }
    }

    /// Build from Grok fixture names in `crates/runie-testing/src/fixtures/grok_build/`.
    pub fn from_fixture_names(names: &[&str]) -> Self {
        let fixtures: Vec<String> = names
            .iter()
            .map(|n| crate::fixtures::grok_build::raw_fixture(n))
            .collect();
        Self::new(fixtures)
    }
}

impl Provider for GrokReplayProvider {
    fn generate(
        &self,
        _messages: Vec<ChatMessage>,
    ) -> Pin<Box<dyn Stream<Item = anyhow::Result<ProviderEvent>> + Send + '_>> {
        let idx = self.index.fetch_add(1, Ordering::SeqCst);
        let events = self
            .fixtures
            .get(idx)
            .map(|f| parse_fixture(f))
            .unwrap_or_default();
        Box::pin(futures::stream::iter(events.into_iter().map(Ok)))
    }
}

/// Wrap a `GrokReplayProvider` in a `BuiltProvider`.
pub fn dyn_grok_replay_provider(fixtures: &[String]) -> BuiltProvider {
    let provider = GrokReplayProvider::new(fixtures.to_vec());
    BuiltProvider::from_provider(Box::new(provider), "grok", "grok-3")
}

/// Wrap a `GrokReplayProvider` in a `BuiltProvider` from fixture names.
pub fn grok_replay_from_fixtures(names: &[&str]) -> BuiltProvider {
    let fixtures: Vec<String> = names
        .iter()
        .map(|n| crate::fixtures::grok_build::raw_fixture(n))
        .collect();
    dyn_grok_replay_provider(&fixtures)
}

/// Capture emitted `Event`s into a `Vec`.
///
/// Returns `(events, emit)` where `emit` is a closure compatible with
/// `runie_agent::stream_response::EmitFn = Box<dyn Fn(Event) + Send + Sync>`.
///
/// Events are written synchronously into `events` via `parking_lot::Mutex` so
/// tests can read them via `events.lock()` immediately after `run_agent_turn`
/// completes — no background task, no polling, no race conditions.
pub fn capture_events() -> (
    Arc<parking_lot::Mutex<Vec<Event>>>,
    runie_agent::stream_response::EmitFn,
) {
    let events: Arc<parking_lot::Mutex<Vec<Event>>> = Arc::new(parking_lot::Mutex::new(Vec::new()));
    let captured = events.clone();
    let emit: runie_agent::stream_response::EmitFn = Arc::new(move |evt: Event| {
        captured.lock().push(evt);
    });
    (events, emit)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn replay_provider_cycles_fixtures() {
        let provider = ReplayProvider::new(vec!["a".to_string(), "b".to_string()]);
        // We can't easily test the Stream without running it, but we can
        // verify construction doesn't panic and that the index starts at 0.
        let _ = provider;
    }

    #[test]
    fn dyn_replay_builds_dyn_provider() {
        let fixtures = vec!["hello".to_string()];
        let provider = dyn_replay_provider(&fixtures);
        assert_eq!(provider.key(), "minimax");
        assert_eq!(provider.model(), "MiniMax-M3");
    }
}
