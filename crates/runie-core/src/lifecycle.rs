//! Lifecycle state machine for LLM content blocks.
//!
//! Tracks open text/reasoning blocks and emits synthetic `TextStart`/`TextEnd`
//! and `ThinkingStart`/`ThinkingEnd` events so downstream consumers can build
//! proper `Vec<Part>` content during streaming.

use std::collections::HashSet;

use crate::provider_event::ProviderEvent;

/// Tracks open content blocks and emits lifecycle events.
#[derive(Debug, Default)]
pub struct LifecycleState {
    open_text_blocks: HashSet<String>,
    open_thinking_blocks: HashSet<String>,
}

impl LifecycleState {
    /// Create a new empty lifecycle state.
    pub fn new() -> Self {
        Self::default()
    }

    /// Process a text delta and emit lifecycle events.
    ///
    /// Returns `TextStart` on first delta for this `id`, then `TextDelta`.
    pub fn text_delta(&mut self, id: &str, delta: &str) -> Vec<ProviderEvent> {
        let is_new = self.open_text_blocks.insert(id.to_string());
        let mut events = Vec::new();
        if is_new {
            events.push(ProviderEvent::TextStart { id: id.to_string() });
        }
        events.push(ProviderEvent::TextDelta(delta.to_string()));
        events
    }

    /// Explicitly close a text block and emit `TextEnd`.
    pub fn text_end(&mut self, id: &str) -> Vec<ProviderEvent> {
        self.open_text_blocks.remove(id);
        vec![ProviderEvent::TextEnd { id: id.to_string() }]
    }

    /// Process a thinking delta and emit lifecycle events.
    ///
    /// Returns `ThinkingStart` on first delta for this `id`, then `ThinkingDelta`.
    pub fn thinking_delta(&mut self, id: &str, delta: &str) -> Vec<ProviderEvent> {
        let is_new = self.open_thinking_blocks.insert(id.to_string());
        let mut events = Vec::new();
        if is_new {
            events.push(ProviderEvent::ThinkingStart { id: id.to_string() });
        }
        events.push(ProviderEvent::ThinkingDelta(delta.to_string()));
        events
    }

    /// Explicitly close a thinking block and emit `ThinkingEnd`.
    pub fn thinking_end(&mut self, id: &str) -> Vec<ProviderEvent> {
        self.open_thinking_blocks.remove(id);
        vec![ProviderEvent::ThinkingEnd { id: id.to_string() }]
    }

    /// Close all open blocks and emit their end events, plus `Finish`.
    pub fn finish(&mut self, reason: crate::provider_event::StopReason) -> Vec<ProviderEvent> {
        let mut events = Vec::new();
        for id in self.open_text_blocks.drain() {
            events.push(ProviderEvent::TextEnd { id });
        }
        for id in self.open_thinking_blocks.drain() {
            events.push(ProviderEvent::ThinkingEnd { id });
        }
        events.push(ProviderEvent::Finish { reason });
        events
    }
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lifecycle_emits_start_on_first_delta() {
        let mut state = LifecycleState::new();
        let events = state.text_delta("b1", "hi");
        assert_eq!(
            events,
            vec![
                ProviderEvent::TextStart { id: "b1".into() },
                ProviderEvent::TextDelta("hi".into())
            ]
        );
    }

    #[test]
    fn lifecycle_skips_start_on_continuation() {
        let mut state = LifecycleState::new();
        state.text_delta("b1", "hi");
        let events = state.text_delta("b1", " world");
        assert_eq!(events, vec![ProviderEvent::TextDelta(" world".into())]);
    }

    #[test]
    fn lifecycle_finish_closes_all_open_blocks() {
        let mut state = LifecycleState::new();
        state.text_delta("t1", "hello");
        state.text_delta("t2", "world");
        state.thinking_delta("r1", "thinking");
        let events = state.finish(crate::provider_event::StopReason::Stop);
        assert!(events.contains(&ProviderEvent::TextEnd { id: "t1".into() }));
        assert!(events.contains(&ProviderEvent::TextEnd { id: "t2".into() }));
        assert!(events.contains(&ProviderEvent::ThinkingEnd { id: "r1".into() }));
        assert!(events.contains(&ProviderEvent::Finish {
            reason: crate::provider_event::StopReason::Stop
        }));
        assert_eq!(events.len(), 4); // 3 End + 1 Finish
    }

    #[test]
    fn lifecycle_text_end_removes_from_open_set() {
        let mut state = LifecycleState::new();
        state.text_delta("b1", "hi");
        state.text_end("b1");
        let events = state.text_delta("b1", "x");
        assert_eq!(
            events,
            vec![
                ProviderEvent::TextStart { id: "b1".into() },
                ProviderEvent::TextDelta("x".into())
            ]
        );
    }

    #[test]
    fn lifecycle_thinking_delta_emits_thinking_start() {
        let mut state = LifecycleState::new();
        let events = state.thinking_delta("r1", "reasoning");
        assert_eq!(
            events,
            vec![
                ProviderEvent::ThinkingStart { id: "r1".into() },
                ProviderEvent::ThinkingDelta("reasoning".into())
            ]
        );
    }

    #[test]
    fn lifecycle_multiple_text_blocks_independent() {
        let mut state = LifecycleState::new();
        let e1 = state.text_delta("a", "hello");
        let e2 = state.text_delta("b", "world");
        // Each delta should emit exactly 2 events (Start + Delta)
        assert_eq!(e1.len(), 2);
        assert_eq!(e2.len(), 2);
        // First event of each should be TextStart
        match &e1[0] {
            ProviderEvent::TextStart { id } => assert_eq!(id, "a"),
            _ => panic!("Expected TextStart"),
        }
        match &e2[0] {
            ProviderEvent::TextStart { id } => assert_eq!(id, "b"),
            _ => panic!("Expected TextStart"),
        }
    }
}
