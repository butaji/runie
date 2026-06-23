//! Lifecycle state machine for LLM content blocks.
//!
//! Tracks open text/reasoning blocks and emits synthetic `TextStart`/`TextEnd`
//! and `ThinkingStart`/`ThinkingEnd` events so downstream consumers can build
//! proper `Vec<Part>` content during streaming.

use std::collections::HashSet;

use crate::llm_event::LLMEvent;

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
    pub fn text_delta(&mut self, id: &str, delta: &str) -> Vec<LLMEvent> {
        let is_new = self.open_text_blocks.insert(id.to_string());
        let mut events = Vec::new();
        if is_new {
            events.push(LLMEvent::TextStart { id: id.to_string() });
        }
        events.push(LLMEvent::TextDelta(delta.to_string()));
        events
    }

    /// Explicitly close a text block and emit `TextEnd`.
    pub fn text_end(&mut self, id: &str) -> Vec<LLMEvent> {
        self.open_text_blocks.remove(id);
        vec![LLMEvent::TextEnd { id: id.to_string() }]
    }

    /// Process a thinking delta and emit lifecycle events.
    ///
    /// Returns `ThinkingStart` on first delta for this `id`, then `ThinkingDelta`.
    pub fn thinking_delta(&mut self, id: &str, delta: &str) -> Vec<LLMEvent> {
        let is_new = self.open_thinking_blocks.insert(id.to_string());
        let mut events = Vec::new();
        if is_new {
            events.push(LLMEvent::ThinkingStart { id: id.to_string() });
        }
        events.push(LLMEvent::ThinkingDelta(delta.to_string()));
        events
    }

    /// Explicitly close a thinking block and emit `ThinkingEnd`.
    pub fn thinking_end(&mut self, id: &str) -> Vec<LLMEvent> {
        self.open_thinking_blocks.remove(id);
        vec![LLMEvent::ThinkingEnd { id: id.to_string() }]
    }

    /// Close all open blocks and emit their end events, plus `Finish`.
    pub fn finish(&mut self, reason: crate::llm_event::StopReason) -> Vec<LLMEvent> {
        let mut events = Vec::new();
        for id in self.open_text_blocks.drain() {
            events.push(LLMEvent::TextEnd { id });
        }
        for id in self.open_thinking_blocks.drain() {
            events.push(LLMEvent::ThinkingEnd { id });
        }
        events.push(LLMEvent::Finish { reason });
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
                LLMEvent::TextStart { id: "b1".into() },
                LLMEvent::TextDelta("hi".into())
            ]
        );
    }

    #[test]
    fn lifecycle_skips_start_on_continuation() {
        let mut state = LifecycleState::new();
        state.text_delta("b1", "hi");
        let events = state.text_delta("b1", " world");
        assert_eq!(events, vec![LLMEvent::TextDelta(" world".into())]);
    }

    #[test]
    fn lifecycle_finish_closes_all_open_blocks() {
        let mut state = LifecycleState::new();
        state.text_delta("t1", "hello");
        state.text_delta("t2", "world");
        state.thinking_delta("r1", "thinking");
        let events = state.finish(crate::llm_event::StopReason::Stop);
        assert!(events.contains(&LLMEvent::TextEnd { id: "t1".into() }));
        assert!(events.contains(&LLMEvent::TextEnd { id: "t2".into() }));
        assert!(events.contains(&LLMEvent::ThinkingEnd { id: "r1".into() }));
        assert!(events.contains(&LLMEvent::Finish {
            reason: crate::llm_event::StopReason::Stop
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
                LLMEvent::TextStart { id: "b1".into() },
                LLMEvent::TextDelta("x".into())
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
                LLMEvent::ThinkingStart { id: "r1".into() },
                LLMEvent::ThinkingDelta("reasoning".into())
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
            LLMEvent::TextStart { id } => assert_eq!(id, "a"),
            _ => panic!("Expected TextStart"),
        }
        match &e2[0] {
            LLMEvent::TextStart { id } => assert_eq!(id, "b"),
            _ => panic!("Expected TextStart"),
        }
    }
}
