//! DSL tests — Element building and feed construction

#[cfg(test)]
mod tests {
    use crate::event::AgentEvent;
    use crate::message::Part;
    use crate::model::{AppState, ChatMessage, Role};
    use crate::view::elements::Element;
    use crate::view::LazyCache;

    fn msg(role: Role, content: &str, timestamp: f64, id: &str) -> ChatMessage {
        ChatMessage {
            role,
            parts: vec![Part::Text { content: content.into() }],
            timestamp,
            id: id.into(),
            ..Default::default()
        }
    }

    #[test]
    fn test_rebuild_creates_elements() {
        let mut state = AppState::default();
        state.session.messages.push(ChatMessage {
            role: Role::User,
            parts: vec![Part::Text { content: "Hello".to_string() }],
            timestamp: 0.0,
            id: "req.0".to_string(),
            ..Default::default()
        });
        let elems = LazyCache::rebuild(&state);
        assert!(!elems.is_empty());
    }

    #[test]
    fn test_visible_slices_correctly() {
        let cache = vec![
            Element::UserMessage {
                content: "a".to_string(),
                timestamp: 0.0,
            },
            Element::Spacer { timestamp: 0.0 },
            Element::UserMessage {
                content: "b".to_string(),
                timestamp: 1.0,
            },
        ];
        let visible = LazyCache::visible(&cache, 0, 2);
        assert_eq!(visible.len(), 2);
    }

    #[test]
    fn test_visible_bounds_check() {
        let cache = vec![Element::UserMessage {
            content: "a".to_string(),
            timestamp: 0.0,
        }];
        let visible = LazyCache::visible(&cache, 10, 5);
        assert!(visible.is_empty());
    }

    #[test]
    fn test_feed_merges_consecutive_agent_messages() {
        let mut state = AppState::default();
        state.update(AgentEvent::Response {
            id: "req.0".to_string(),
            content: "Hello ".to_string(),
        });
        state.update(AgentEvent::Response {
            id: "req.0".to_string(),
            content: "World".to_string(),
        });

        let feed = LazyCache::feed(&state);
        let agent_msgs: Vec<_> = feed
            .elements
            .iter()
            .filter(|e| matches!(e, Element::AgentMessage { .. }))
            .collect();
        assert_eq!(agent_msgs.len(), 1);
        if let Element::AgentMessage { content, .. } = &feed.elements[0] {
            assert_eq!(content, "Hello World");
        }
    }

    #[test]
    fn test_every_message_has_spacer() {
        let mut state = AppState::default();
        state.session.messages.extend([
            msg(Role::User, "Hello", 0.0, "req.0"),
            msg(Role::Thought, "Thinking...", 0.0, "req.0"),
            msg(Role::Assistant, "Hi", 0.0, "req.0"),
            msg(Role::Tool, "Ran test 1.0s", 0.0, "req.0"),
            msg(Role::TurnComplete, "Turn completed in 2.0s", 0.0, "req.0"),
        ]);

        let elements = LazyCache::rebuild(&state);
        for (i, elem) in elements.iter().enumerate().step_by(2) {
            assert!(
                !matches!(elem, Element::Spacer { .. }),
                "idx {} should be message",
                i
            );
        }
        for (i, elem) in elements.iter().enumerate().skip(1).step_by(2) {
            assert!(
                matches!(elem, Element::Spacer { .. }),
                "idx {} should be Spacer",
                i
            );
        }
    }

    #[test]
    fn test_thinking_has_spacer() {
        let mut state = AppState::default();
        state.agent.thinking_started_at = Some(std::time::Instant::now());
        let elements = LazyCache::rebuild(&state);
        assert!(matches!(elements[0], Element::Spacer { .. }));
        assert!(matches!(elements[1], Element::Thinking { .. }));
        assert!(matches!(elements[2], Element::Spacer { .. }));
    }

    #[test]
    fn test_no_messages_no_elements() {
        let state = AppState::default();
        let elements = LazyCache::rebuild(&state);
        assert!(elements.is_empty());
    }

    fn kind(e: &Element) -> &'static str {
        match e {
            Element::UserMessage { .. } => "U",
            Element::AgentMessage { .. } => "A",
            Element::ThoughtMarker { .. } => "T",
            Element::Thinking { .. } => "I",
            Element::Spacer { .. } => "S",
            _ => "?",
        }
    }

    #[test]
    fn test_elements_follow_timestamp_order() {
        let mut state = AppState::default();
        state.session.messages.extend([
            msg(Role::User, "Q1", 1.0, "u1"),
            msg(Role::Assistant, "A1", 2.0, "a1"),
            msg(Role::User, "Q2", 3.0, "u2"),
        ]);
        state.session.messages[1].timestamp = 4.0;
        let kinds: Vec<&str> = LazyCache::feed(&state).elements.iter().map(kind).collect();
        assert_eq!(
            kinds,
            vec!["U", "S", "U", "S", "A", "S"],
            "A1 updated to t=4 should float to bottom"
        );
    }

    #[test]
    fn test_thought_ordered_by_timestamp() {
        let mut state = AppState::default();
        state.session.messages.extend([
            msg(Role::Assistant, "Hello", 1.0, "same"),
            msg(Role::Thought, "Thinking...", 2.0, "same"),
        ]);
        let kinds: Vec<&str> = LazyCache::feed(&state).elements.iter().map(kind).collect();
        assert_eq!(
            kinds,
            vec!["S", "A", "S", "T", "S"],
            "Elements ordered by last update timestamp"
        );
    }

    #[test]
    fn test_insertion_order_stable() {
        let mut state = AppState::default();
        state.session.messages.push(ChatMessage {
            role: Role::User,
            parts: vec![Part::Text { content: "First".into() }],
            timestamp: 1.0,
            id: "u1".into(),
            ..Default::default()
        });
        state.session.messages.push(ChatMessage {
            role: Role::User,
            parts: vec![Part::Text { content: "Second".into() }],
            timestamp: 1.0,
            id: "u2".into(),
            ..Default::default()
        });
        let feed = LazyCache::feed(&state);
        let texts: Vec<String> = feed
            .elements
            .iter()
            .filter_map(|e| match e {
                Element::UserMessage { content, .. } => Some(content.clone()),
                _ => None,
            })
            .collect();
        assert_eq!(texts, vec!["First", "Second"]);
    }

    #[test]
    fn thinking_indicator_after_user_when_no_response_yet() {
        let mut state = AppState::default();
        state.session.messages.extend([
            msg(Role::User, "Q1", 0.0, "t1"),
            msg(Role::Assistant, "A1", 1.0, "t1"),
            msg(Role::User, "Q2", 2.0, "t2"),
        ]);
        state.agent.thinking_started_at = Some(std::time::Instant::now());
        let kinds: Vec<&str> = LazyCache::feed(&state).elements.iter().map(kind).collect();
        assert_eq!(
            kinds,
            vec!["U", "S", "A", "S", "U", "S", "I", "S"],
            "Thinking must be at bottom after current user msg, not before old assistant msg"
        );
    }

    #[test]
    fn thinking_indicator_ordered_by_timestamp() {
        let mut state = AppState::default();
        state.session.messages.extend([
            msg(Role::User, "Q1", 0.0, "t1"),
            msg(Role::Assistant, "A1", 1.0, "t1"),
            msg(Role::User, "Q2", 2.0, "t2"),
            msg(Role::Assistant, "A2 partial", 3.0, "t2"),
        ]);
        state.agent.thinking_started_at = Some(std::time::Instant::now());
        state.agent.current_request_id = Some("t2".into());
        let kinds: Vec<&str> = LazyCache::feed(&state).elements.iter().map(kind).collect();
        assert_eq!(
            kinds,
            vec!["U", "S", "A", "S", "U", "S", "I", "S"],
            "A2 hidden during thinking, Thinking indicator at bottom"
        );
    }
}
