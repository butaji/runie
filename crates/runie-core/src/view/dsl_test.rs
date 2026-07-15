//! DSL tests — Element building and feed construction

#[cfg(test)]
mod tests {
    use crate::message::Part;
    use crate::model::{AppState, ChatMessage, Role};
    use crate::view::elements::Element;
    use crate::view::LazyCache;

    fn msg(role: Role, content: &str, timestamp: f64, id: &str) -> ChatMessage {
        ChatMessage {
            role,
            parts: vec![Part::Text {
                content: content.into(),
            }],
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
            parts: vec![Part::Text {
                content: "Hello".to_string(),
            }],
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
        state.update(crate::Event::Response {
            id: "req.0".to_string(),
            content: "Hello ".to_string(),

            role: String::new(),
            timestamp: 0.0,
            provider: String::new(),
        });
        state.update(crate::Event::Response {
            id: "req.0".to_string(),
            content: "World".to_string(),

            role: String::new(),
            timestamp: 0.0,
            provider: String::new(),
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
            // Both thought forms (full marker and default one-line summary)
            // are thought posts for ordering purposes.
            Element::ThoughtMarker { .. } | Element::ThoughtSummary { .. } => "T",
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
            parts: vec![Part::Text {
                content: "First".into(),
            }],
            timestamp: 1.0,
            id: "u1".into(),
            ..Default::default()
        });
        state.session.messages.push(ChatMessage {
            role: Role::User,
            parts: vec![Part::Text {
                content: "Second".into(),
            }],
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

    // =========================================================================
    // Layer 1 Tests: Model → Element mapping (generate-ui-elements-from-model)
    // =========================================================================

    /// `element_from_user_chat_message` — ChatMessage with Role::User produces Element::UserMessage.
    #[test]
    fn element_from_user_chat_message() {
        let user_msg = ChatMessage {
            role: Role::User,
            parts: vec![Part::Text {
                content: "Hello, world!".into(),
            }],
            timestamp: 1.0,
            id: "u1".into(),
            ..Default::default()
        };

        let mut state = AppState::default();
        state.session.messages.push(user_msg);

        let elements = LazyCache::rebuild(&state);

        // Find the UserMessage element (skip spacers)
        let user_elements: Vec<_> = elements
            .iter()
            .filter(|e| matches!(e, Element::UserMessage { .. }))
            .collect();

        assert_eq!(
            user_elements.len(),
            1,
            "Expected exactly one UserMessage element"
        );
        if let Element::UserMessage { content, timestamp } = user_elements[0] {
            assert_eq!(content, "Hello, world!");
            assert_eq!(*timestamp, 1.0);
        } else {
            panic!("Expected UserMessage element");
        }
    }

    /// `element_from_tool_output` — Tool message with success produces Element::ToolDone.
    #[test]
    fn element_from_tool_output() {
        // Format expected by transform.rs: "Ran <tool_name> <duration>\n<output>"
        let tool_msg = ChatMessage {
            role: Role::Tool,
            parts: vec![Part::Text {
                content: "Ran bash 1.5s\nfile1.txt\nfile2.txt".into(),
            }],
            timestamp: 2.0,
            id: "tool.0".into(),
            tool_call_id: Some("call_abc".into()),
            ..Default::default()
        };

        let mut state = AppState::default();
        state.session.messages.push(tool_msg);

        let elements = LazyCache::rebuild(&state);

        // Find the ToolDone element
        let tool_elements: Vec<_> = elements
            .iter()
            .filter(|e| matches!(e, Element::ToolDone { .. }))
            .collect();

        assert_eq!(
            tool_elements.len(),
            1,
            "Expected exactly one ToolDone element"
        );
        if let Element::ToolDone {
            name,
            duration_secs,
            output,
            error,
            ..
        } = tool_elements[0]
        {
            assert_eq!(name, "bash", "Tool name should be 'bash'");
            assert!(
                (duration_secs - 1.5).abs() < 0.01,
                "Expected duration ~1.5s"
            );
            assert!(
                output.contains("file1.txt"),
                "Output should contain file1.txt"
            );
            assert!(!error, "ToolDone should not be an error");
        } else {
            panic!("Expected ToolDone element");
        }
    }

    /// `element_from_assistant_chat_message` — ChatMessage with Role::Assistant produces Element::AgentMessage.
    #[test]
    fn element_from_assistant_chat_message() {
        let assistant_msg = ChatMessage {
            role: Role::Assistant,
            parts: vec![Part::Text {
                content: "I'll help you with that.".into(),
            }],
            timestamp: 3.0,
            id: "a1".into(),
            provider: "openai".into(),
            ..Default::default()
        };

        let mut state = AppState::default();
        state.session.messages.push(assistant_msg);

        let elements = LazyCache::rebuild(&state);

        let agent_elements: Vec<_> = elements
            .iter()
            .filter(|e| matches!(e, Element::AgentMessage { .. }))
            .collect();

        assert_eq!(
            agent_elements.len(),
            1,
            "Expected exactly one AgentMessage element"
        );
        if let Element::AgentMessage {
            content,
            timestamp,
            provider,
        } = agent_elements[0]
        {
            assert_eq!(content, "I'll help you with that.");
            assert_eq!(*timestamp, 3.0);
            assert_eq!(provider, "openai");
        } else {
            panic!("Expected AgentMessage element");
        }
    }

    /// `element_from_thought_chat_message` — ChatMessage with Role::Thought produces a
    /// thought element: a one-line ThoughtSummary by default (grok parity),
    /// expanding to the full ThoughtMarker when individually expanded.
    #[test]
    fn element_from_thought_chat_message() {
        let thought_msg = ChatMessage {
            role: Role::Thought,
            parts: vec![Part::Text {
                content: "Let me think about this 2.0s".into(),
            }],
            timestamp: 4.0,
            id: "thought.0".into(),
            ..Default::default()
        };

        let mut state = AppState::default();
        state.session.messages.push(thought_msg);

        // Default: collapsed one-line summary carrying the first line and ts.
        let elements = LazyCache::rebuild(&state);
        let summaries: Vec<_> = elements
            .iter()
            .filter(|e| matches!(e, Element::ThoughtSummary { .. }))
            .collect();
        assert_eq!(summaries.len(), 1, "Expected exactly one ThoughtSummary");
        if let Element::ThoughtSummary {
            content, timestamp, ..
        } = summaries[0]
        {
            assert_eq!(content, "Let me think about this 2.0s");
            assert_eq!(*timestamp, 4.0);
        } else {
            panic!("Expected ThoughtSummary element");
        }

        // Individually expanded: full marker with the complete body.
        state.view_mut().expanded_posts.insert(0);
        let elements = LazyCache::rebuild(&state);
        let markers: Vec<_> = elements
            .iter()
            .filter(|e| matches!(e, Element::ThoughtMarker { .. }))
            .collect();
        assert_eq!(markers.len(), 1, "expanded thought must render full body");
    }

    #[test]
    fn swarm_worker_rows_render_before_assistant_response() {
        let mut state = AppState::default();
        // User message at t=1 starts the turn.
        state.session.messages.push(msg(Role::User, "hello", 1.0, "req.0"));
        // Worker row emitted by the pattern before the final Response.
        state.update(crate::Event::PatternWorkerSpawned {
            id: "worker-1".into(),
            description: "Analyze context".into(),
            model: "mock/echo".into(),
        });
        state.update(crate::Event::PatternWorkerFinished {
            id: "worker-1".into(),
            status: "completed".into(),
            duration_ms: 500,
            output: "done".into(),
        });
        // Final assistant response arrives after worker rows.
        state.update(crate::Event::Response {
            id: "req.0".into(),
            content: "Final answer".into(),
            role: String::new(),
            timestamp: crate::message::now(),
            provider: String::new(),
        });

        let elements = LazyCache::rebuild(&state);
        let positions: Vec<_> = elements
            .iter()
            .enumerate()
            .filter_map(|(i, e)| match e {
                Element::SubagentRow { id, .. } if id == "worker-1" => Some(("worker", i)),
                Element::AgentMessage { content, .. } if content == "Final answer" => {
                    Some(("response", i))
                }
                _ => None,
            })
            .collect();
        let worker_pos = positions.iter().find(|(k, _)| *k == "worker").map(|(_, i)| *i);
        let response_pos = positions.iter().find(|(k, _)| *k == "response").map(|(_, i)| *i);
        assert!(
            worker_pos.is_some() && response_pos.is_some(),
            "both worker row and response must be present: {positions:?}"
        );
        assert!(
            worker_pos.unwrap() < response_pos.unwrap(),
            "worker row must appear before final response"
        );
    }

}
