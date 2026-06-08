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
            Element::UserMessage { content: "a".to_string(), timestamp: 0.0 },
            Element::Spacer { timestamp: 0.0 },
            Element::UserMessage { content: "b".to_string(), timestamp: 1.0 },
        ];
        let visible = LazyCache::visible(&cache, 0, 2);
        assert_eq!(visible.len(), 2);
    }

    #[test]
    fn test_visible_bounds_check() {
        let cache = vec![Element::UserMessage { content: "a".to_string(), timestamp: 0.0 }];
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

    #[test]
    fn test_every_message_has_spacer() {
        let mut state = AppState::default();
        state.messages.push(ChatMessage { role: Role::User, content: "Hello".into(), timestamp: 0.0, id: "req.0".into() });
        state.messages.push(ChatMessage { role: Role::Thought, content: "Thinking...".into(), timestamp: 0.0, id: "req.0".into() });
        state.messages.push(ChatMessage { role: Role::Assistant, content: "Hi".into(), timestamp: 0.0, id: "req.0".into() });
        state.messages.push(ChatMessage { role: Role::Tool, content: "Ran test 1.0s".into(), timestamp: 0.0, id: "req.0".into() });
        state.messages.push(ChatMessage { role: Role::TurnComplete, content: "Turn completed in 2.0s".into(), timestamp: 0.0, id: "req.0".into() });

        let elements = LazyCache::rebuild(&state);
        for (i, elem) in elements.iter().enumerate().step_by(2) {
            assert!(!matches!(elem, Element::Spacer { .. }), "idx {} should be message", i);
        }
        for (i, elem) in elements.iter().enumerate().skip(1).step_by(2) {
            assert!(matches!(elem, Element::Spacer { .. }), "idx {} should be Spacer", i);
        }
    }

    #[test]
    fn test_thinking_has_spacer() {
        let mut state = AppState::default();
        state.thinking_started_at = Some(std::time::Instant::now());
        let elements = LazyCache::rebuild(&state);
        assert!(matches!(elements[0], Element::Thinking { .. }));
        assert!(matches!(elements[1], Element::Spacer { .. }));
    }

    #[test]
    fn test_no_messages_no_elements() {
        let state = AppState::default();
        let elements = LazyCache::rebuild(&state);
        assert!(elements.is_empty());
    }

    #[test]
    fn test_elements_follow_timestamp_order() {
        let mut state = AppState::default();
        state.messages.push(ChatMessage { role: Role::User, content: "Q1".into(), timestamp: 1.0, id: "u1".into() });
        state.messages.push(ChatMessage { role: Role::Assistant, content: "A1".into(), timestamp: 2.0, id: "a1".into() });
        state.messages.push(ChatMessage { role: Role::User, content: "Q2".into(), timestamp: 3.0, id: "u2".into() });
        state.messages[1].timestamp = 4.0;
        let feed = LazyCache::feed(&state);
        let kinds: Vec<&str> = feed.elements.iter().map(|e| match e {
            Element::UserMessage { .. } => "U",
            Element::AgentMessage { .. } => "A",
            Element::Spacer { .. } => "S",
            _ => "?",
        }).collect();
        assert_eq!(kinds, vec!["U", "S", "U", "S", "A", "S"], "A1 updated to t=4 should float to bottom");
    }

    #[test]
    fn test_thought_ordered_by_timestamp() {
        let mut state = AppState::default();
        state.messages.push(ChatMessage { role: Role::Assistant, content: "Hello".into(), timestamp: 1.0, id: "same".into() });
        state.messages.push(ChatMessage { role: Role::Thought, content: "Thinking...".into(), timestamp: 2.0, id: "same".into() });
        let feed = LazyCache::feed(&state);
        let kinds: Vec<&str> = feed.elements.iter().map(|e| match e {
            Element::ThoughtMarker { .. } => "T",
            Element::AgentMessage { .. } => "A",
            Element::Spacer { .. } => "S",
            _ => "?",
        }).collect();
        assert_eq!(kinds, vec!["A", "S", "T", "S"], "Elements ordered by last update timestamp");
    }

    #[test]
    fn test_insertion_order_stable() {
        let mut state = AppState::default();
        state.messages.push(ChatMessage { role: Role::User, content: "First".into(), timestamp: 1.0, id: "u1".into() });
        state.messages.push(ChatMessage { role: Role::User, content: "Second".into(), timestamp: 1.0, id: "u2".into() });
        let feed = LazyCache::feed(&state);
        let texts: Vec<String> = feed.elements.iter().filter_map(|e| match e {
            Element::UserMessage { content, .. } => Some(content.clone()),
            _ => None,
        }).collect();
        assert_eq!(texts, vec!["First", "Second"]);
    }

    #[test]
    fn thinking_indicator_after_user_when_no_response_yet() {
        let mut state = AppState::default();
        state.messages.push(ChatMessage { role: Role::User, content: "Q1".into(), timestamp: 0.0, id: "t1".into() });
        state.messages.push(ChatMessage { role: Role::Assistant, content: "A1".into(), timestamp: 1.0, id: "t1".into() });
        state.messages.push(ChatMessage { role: Role::User, content: "Q2".into(), timestamp: 2.0, id: "t2".into() });
        state.thinking_started_at = Some(std::time::Instant::now());
        let feed = LazyCache::feed(&state);
        let kinds: Vec<&str> = feed.elements.iter().map(|e| match e {
            Element::UserMessage { .. } => "U",
            Element::AgentMessage { .. } => "A",
            Element::Thinking { .. } => "I",
            Element::Spacer { .. } => "S",
            _ => "?",
        }).collect();
        assert_eq!(kinds, vec!["U", "S", "A", "S", "U", "S", "I", "S"], "Thinking must be at bottom after current user msg, not before old assistant msg");
    }

    #[test]
    fn thinking_indicator_ordered_by_timestamp() {
        let mut state = AppState::default();
        state.messages.push(ChatMessage { role: Role::User, content: "Q1".into(), timestamp: 0.0, id: "t1".into() });
        state.messages.push(ChatMessage { role: Role::Assistant, content: "A1".into(), timestamp: 1.0, id: "t1".into() });
        state.messages.push(ChatMessage { role: Role::User, content: "Q2".into(), timestamp: 2.0, id: "t2".into() });
        state.messages.push(ChatMessage { role: Role::Assistant, content: "A2 partial".into(), timestamp: 3.0, id: "t2".into() });
        state.thinking_started_at = Some(std::time::Instant::now());
        state.current_request_id = Some("t2".into());
        let feed = LazyCache::feed(&state);
        let kinds: Vec<&str> = feed.elements.iter().map(|e| match e {
            Element::UserMessage { .. } => "U",
            Element::AgentMessage { .. } => "A",
            Element::Thinking { .. } => "I",
            Element::Spacer { .. } => "S",
            _ => "?",
        }).collect();
        // A2 is skipped during thinking (id=t2 matches current_request_id), Thinking at max_ts+1
        assert_eq!(kinds, vec!["U", "S", "A", "S", "U", "S", "I", "S"],
            "A2 hidden during thinking, Thinking indicator at bottom");
    }
}
