//! DSL tests — Element building and feed construction

#[cfg(test)]
mod tests {
    use crate::model::AppState;
    use crate::event::Event;
    use crate::ui::LazyCache;
    use crate::ui::elements::Element;

    #[test]
    fn test_rebuild_creates_elements() {
        let mut state = AppState::default();
        state.messages.push(crate::model::ChatMessage {
            role: "user".to_string(),
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
}
