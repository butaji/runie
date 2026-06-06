//! DSL tests — Element building and feed construction

#[cfg(test)]
mod tests {
    use crate::model::{AppState, ChatMessage, Role};
    use crate::event::Event;
    use crate::ui::LazyCache;
    use crate::ui::elements::Element;

    #[test]
    fn test_rebuild_creates_elements() {
        let mut state = AppState::default();
        state.messages.push(ChatMessage {
            role: Role::User,
            content: "Hello".to_string(),
            timestamp: 0.0,
            id: "req.0".to_string(),
        });
        let elems = LazyCache::rebuild(&state);
        assert!(!elems.is_empty());
    }

    #[test]
    fn test_visible_slices_correctly() {
        let cache = vec![
            Element::UserMessage { content: "a".to_string() },
            Element::Spacer,
            Element::UserMessage { content: "b".to_string() },
        ];
        let visible = LazyCache::visible(&cache, 0, 2);
        assert_eq!(visible.len(), 2);
    }

    #[test]
    fn test_visible_bounds_check() {
        let cache = vec![Element::UserMessage { content: "a".to_string() }];
        let visible = LazyCache::visible(&cache, 10, 5);
        assert!(visible.is_empty());
    }

    #[test]
    fn test_feed_merges_consecutive_agent_messages() {
        let mut state = AppState::default();
        state.update(Event::AgentResponse { id: "req.0".to_string(), content: "Hello ".to_string() });
        state.update(Event::AgentResponse { id: "req.0".to_string(), content: "World".to_string() });

        let feed = LazyCache::feed(&state);
        let agent_msgs: Vec<_> = feed.elements.iter().filter(|e| matches!(e, Element::AgentMessage { .. })).collect();
        assert_eq!(agent_msgs.len(), 1);
        if let Element::AgentMessage { content, .. } = &feed.elements[0] {
            assert_eq!(content, "Hello World");
        }
    }

    // === poc additions ===

    #[test]
    fn test_every_message_has_spacer() {
        let mut state = AppState::default();
        state.messages.push(ChatMessage { role: Role::User, content: "Hello".into(), timestamp: 0.0, id: "req.0".into() });
        state.messages.push(ChatMessage { role: Role::Thought, content: "Thinking...".into(), timestamp: 0.0, id: "req.0".into() });
        state.messages.push(ChatMessage { role: Role::Assistant, content: "Hi".into(), timestamp: 0.0, id: "req.0".into() });
        state.messages.push(ChatMessage { role: Role::Tool, content: "◆ Ran test 1.0s".into(), timestamp: 0.0, id: "req.0".into() });
        state.messages.push(ChatMessage { role: Role::TurnComplete, content: "Turn completed in 2.0s".into(), timestamp: 0.0, id: "req.0".into() });

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
        let mut state = AppState::default();
        state.thinking_started_at = Some(std::time::Instant::now());
        let elements = LazyCache::rebuild(&state);
        assert!(matches!(elements[0], Element::Thinking { .. }));
        assert!(matches!(elements[1], Element::Spacer));
    }

    #[test]
    fn test_no_messages_no_elements() {
        let state = AppState::default();
        let elements = LazyCache::rebuild(&state);
        assert!(elements.is_empty());
    }
}
