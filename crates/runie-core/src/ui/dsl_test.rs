#[cfg(test)]
use crate::model::ChatMessage;
#[cfg(test)]
use crate::ui::transform::LazyCache;
#[cfg(test)]
use crate::ui::elements::Element;

#[test]
fn test_every_message_has_spacer() {
    let mut state = crate::model::AppState::default();
    state.messages.push(ChatMessage { role: "user".into(), content: "Hello".into(), timestamp: 0.0, id: "req.0".into() });
    state.messages.push(ChatMessage { role: "thought".into(), content: "Thinking...".into(), timestamp: 0.0, id: "req.0".into() });
    state.messages.push(ChatMessage { role: "assistant".into(), content: "Hi".into(), timestamp: 0.0, id: "req.0".into() });
    state.messages.push(ChatMessage { role: "tool".into(), content: "◆ Ran test 1.0s".into(), timestamp: 0.0, id: "req.0".into() });
    state.messages.push(ChatMessage { role: "turn_complete".into(), content: "Turn completed in 2.0s".into(), timestamp: 0.0, id: "req.0".into() });

    let elements = LazyCache::rebuild(&state);
    for (i, elem) in elements.iter().enumerate().step_by(2) {
        assert!(!matches!(elem, Element::Spacer), "idx {} should be message", i);
    }
    for (i, elem) in elements.iter().enumerate().skip(1).step_by(2) {
        assert!(matches!(elem, Element::Spacer), "idx {} should be Spacer", i);
    }
}

#[test]
fn test_thinking_has_spacer() {
    let mut state = crate::model::AppState::default();
    state.thinking_started_at = Some(std::time::Instant::now());
    let elements = LazyCache::rebuild(&state);
    assert!(matches!(elements[0], Element::Thinking { .. }));
    assert!(matches!(elements[1], Element::Spacer));
}

#[test]
fn test_no_messages_no_elements() {
    let state = crate::model::AppState::default();
    assert_eq!(LazyCache::rebuild(&state).len(), 0);
}

#[test]
fn test_visible_returns_correct_slice() {
    let mut state = crate::model::AppState::default();
    for i in 0..5 {
        state.messages.push(ChatMessage { role: "user".into(), content: format!("msg{}", i), timestamp: 0.0, id: format!("req.{}", i) });
    }
    state.ensure_fresh();
    assert_eq!(state.element_count(), 10);

    let visible = LazyCache::visible(state.elements_cache(), 4, 4);
    assert_eq!(visible.len(), 4);
    assert!(matches!(visible[0], Element::UserMessage { .. }));
    assert!(matches!(visible[1], Element::Spacer));
    assert!(matches!(visible[2], Element::UserMessage { .. }));
    assert!(matches!(visible[3], Element::Spacer));
}

#[test]
fn test_count_matches_cache_len() {
    let mut state = crate::model::AppState::default();
    state.messages.push(ChatMessage { role: "user".into(), content: "test".into(), timestamp: 0.0, id: "req.0".into() });
    state.ensure_fresh();
    assert_eq!(state.element_count(), state.elements_cache().len());
}
