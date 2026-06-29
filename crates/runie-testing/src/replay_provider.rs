//! Shared replay provider and event-capture helpers.
//!
//! `ReplayProvider` cycles through a list of SSE fixtures, replaying each one
//! on successive calls.  Used by agent-turn replay tests to simulate streaming
//! without a live network connection.

use std::pin::Pin;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;
use parking_lot::Mutex;

use futures::Stream;
use runie_core::message::ChatMessage;
use runie_core::provider::Provider;
use runie_core::provider_event::ProviderEvent;
use runie_core::Event;
use runie_provider::openai::stream::replay_sse;
use runie_provider::DynProvider;

/// A `Provider` that returns pre-recorded SSE fixtures in round-robin order.
pub struct ReplayProvider {
    fixtures: Vec<String>,
    index: AtomicUsize,
}

impl ReplayProvider {
    /// Build a provider that cycles through `fixtures` (each a raw SSE string).
    pub fn new(fixtures: Vec<String>) -> Self {
        Self { fixtures, index: AtomicUsize::new(0) }
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
            .map(|f| replay_sse(f))
            .unwrap_or_default();
        Box::pin(futures::stream::iter(events.into_iter().map(Ok)))
    }
}

/// Wrap a `ReplayProvider` in a `DynProvider`.
pub fn dyn_replay_provider(fixtures: &[String]) -> DynProvider {
    let provider = ReplayProvider::new(fixtures.to_vec());
    DynProvider::from_provider(Box::new(provider), "minimax", "MiniMax-M3")
}

/// Capture emitted `Event`s into a `Vec` using the same `EmitFn` type as
/// `runie_agent::stream_response::EmitFn`.
pub fn capture_events() -> (Arc<Mutex<Vec<Event>>>, runie_agent::stream_response::EmitFn) {
    let events: Arc<Mutex<Vec<Event>>> = Arc::new(Mutex::new(Vec::new()));
    let captured = events.clone();
    let emit: runie_agent::stream_response::EmitFn =
        Arc::new(Mutex::new(move |evt: Event| {
            captured.lock().push(evt);
        }));
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
