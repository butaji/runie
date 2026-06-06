//! Rendering tests

#[cfg(test)]
mod tests {
    use runie_core::{AppState, Event};
    use runie_core::ui::format_messages;

    #[test]
    fn renders_empty_state() {
        let state = AppState::default();
        let lines = format_messages(&state);
        assert!(lines.is_empty());
    }

    #[test]
    fn renders_user_message() {
        let mut state = AppState::default();
        state.update(Event::Input('H'));
        state.update(Event::Submit);
        let lines = format_messages(&state);

        assert_eq!(lines.len(), 2);
        let content: String = lines[0].spans.iter().map(|s| s.text.clone()).collect();
        assert!(content.contains("You:"));
        assert!(content.contains("H"));
    }

    #[test]
    fn renders_agent_response() {
        let mut state = AppState::default();
        state.streaming = true;

        state.update(Event::AgentThinking { id: "req.0".to_string() });
        state.update(Event::AgentThoughtDone { id: "req.0".to_string() });
        state.update(Event::AgentResponse { id: "req.0".to_string(), content: "Hello".to_string() });

        let lines = format_messages(&state);
        let content: String = lines.iter()
            .flat_map(|l| l.spans.iter().map(|s| s.text.clone()).collect::<Vec<_>>())
            .collect();

        assert!(content.contains("Agent:"));
        assert!(content.contains("Hello"));
        assert!(content.contains("◆ Though"));
    }

    #[test]
    fn renders_thinking_for_queued_request() {
        let mut state = AppState::default();
        state.streaming = true;
        state.thinking_started_at = Some(std::time::Instant::now());

        let lines = format_messages(&state);
        let content: String = lines.iter()
            .flat_map(|l| l.spans.iter().map(|s| s.text.clone()).collect::<Vec<_>>())
            .collect();

        assert!(content.contains("Though..."));
    }
}
