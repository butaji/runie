//! Regression tests for "ghost agent message" bug.
//!
//! Agent responses must appear in the feed after the turn completes, regardless
//! of which completion path is used (Event::Done → finish_turn vs
//! Event::TurnCompleted → apply_turn_completed).

use crate::event::Event;
use crate::tests::fresh_state;
use crate::view::elements::Element;
use crate::view::LazyCache;

/// Verify the assistant message appears in the feed after `Event::TurnCompleted`
/// (the production path via TurnActor).
///
/// Before the fix, `apply_turn_completed` did NOT clear `thinking_started_at`,
/// so `should_skip_msg` kept the assistant message hidden even after the turn
/// completed — the "ghost agent message" bug. This test reproduces that scenario.
#[test]
fn no_ghost_agent_after_turn_completed() {
    let mut state = fresh_state();

    // Production-style sequence via TurnActor:
    // TurnStarted → set_thinking (thinking_started_at + current_request_id)
    //   → ResponseDelta → TurnCompleted → apply_turn_completed
    state.update(Event::TurnStarted {
        id: "req.0".into(),
        request_id: "req.0".into(),
        content: "hello".into(),
    });
    state.update(Event::Thinking { id: "req.0".into() });
    state.update(Event::TextStart { id: "req.0".into() });
    state.update(Event::ResponseDelta {
        id: "req.0".into(),
        content: "Hello, world!\n".into(),
    });

    // During streaming with thinking_started_at set: thinking marker is shown
    // (correct) and assistant IS hidden (correct — it's part of the thought).
    {
        let feed = LazyCache::feed(&state);
        let has_thinking = feed
            .elements
            .iter()
            .any(|e| matches!(e, Element::Thinking { .. }));
        assert!(
            has_thinking,
            "Thinking marker must be shown during streaming. Feed: {:?}",
            feed.elements
        );
        let has_agent = feed
            .elements
            .iter()
            .any(|e| matches!(e, Element::AgentMessage { .. }));
        assert!(
            !has_agent,
            "AgentMessage must be hidden during thinking phase. Feed: {:?}",
            feed.elements
        );
    }

    // Turn completes — this is the key step: TurnCompleted does NOT call finish_turn.
    state.update(Event::TurnCompleted);

    // After TurnCompleted, assistant MUST be visible (ghost agent bug is fixed).
    let feed = LazyCache::feed(&state);
    let has_agent = feed
        .elements
        .iter()
        .any(|e| matches!(e, Element::AgentMessage { .. }));
    let texts: Vec<_> = feed
        .elements
        .iter()
        .filter_map(|e| match e {
            Element::AgentMessage { content, .. } => Some(content.clone()),
            _ => None,
        })
        .collect();

    assert!(
        has_agent,
        "AgentMessage must appear in feed after TurnCompleted. \
         Feed elements: {:?}",
        feed.elements
    );
    assert!(
        texts.iter().any(|t| t.contains("Hello")),
        "Agent text 'Hello, world!' must be visible. Got: {:?}",
        texts
    );
}

/// Same as above but without TextStart — exercises the fallback path in
/// `on_response_delta` where the assistant message is created directly from
/// the streaming buffer (no Part::Text opened by TextStart).
#[test]
fn no_ghost_agent_after_turn_completed_no_text_start() {
    let mut state = fresh_state();

    state.update(Event::TurnStarted {
        id: "req.0".into(),
        request_id: "req.0".into(),
        content: "say hi".into(),
    });
    state.update(Event::Thinking { id: "req.0".into() });
    // No TextStart — ResponseDelta creates assistant directly from buffer
    state.update(Event::ResponseDelta {
        id: "req.0".into(),
        content: "Hi there!\n".into(),
    });

    // During streaming with thinking: assistant is hidden
    {
        let feed = LazyCache::feed(&state);
        let has_agent = feed
            .elements
            .iter()
            .any(|e| matches!(e, Element::AgentMessage { .. }));
        assert!(
            !has_agent,
            "AgentMessage must be hidden during thinking (no TextStart path). \
             Feed: {:?}",
            feed.elements
        );
    }

    state.update(Event::TurnCompleted);

    let feed = LazyCache::feed(&state);
    let has_agent = feed
        .elements
        .iter()
        .any(|e| matches!(e, Element::AgentMessage { .. }));
    let texts: Vec<_> = feed
        .elements
        .iter()
        .filter_map(|e| match e {
            Element::AgentMessage { content, .. } => Some(content.clone()),
            _ => None,
        })
        .collect();

    assert!(
        has_agent,
        "AgentMessage must appear after TurnCompleted (no TextStart path). \
         Feed: {:?}",
        feed.elements
    );
    assert!(
        texts.iter().any(|t| t.contains("Hi")),
        "Agent text must be visible. Got: {:?}",
        texts
    );
}
