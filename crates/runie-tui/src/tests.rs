//! Chat rendering tests — ensure UI never breaks

#[cfg(test)]
mod tests {
    use runie_core::{AppState, Event};
    use runie_core::ui::format_messages;

    // === Empty State ===

    #[test]
    fn empty_state_renders_nothing() {
        let state = AppState::default();
        let lines = format_messages(&state);
        assert!(lines.is_empty(), "Empty state should have no lines");
    }

    // === User Messages ===

    #[test]
    fn user_message_has_prefix() {
        let mut state = AppState::default();
        state.update(Event::Input('H'));
        state.update(Event::Submit);
        let lines = format_messages(&state);
        let content: String = lines[0].spans.iter().map(|s| s.text.clone()).collect();
        assert!(content.starts_with("You:"), "User message should start with 'You:'");
    }

    #[test]
    fn user_message_contains_input() {
        let mut state = AppState::default();
        state.update(Event::Input('H'));
        state.update(Event::Input('i'));
        state.update(Event::Submit);
        let lines = format_messages(&state);
        let content: String = lines[0].spans.iter().map(|s| s.text.clone()).collect();
        assert!(content.contains("Hi"), "Should contain input text");
    }

    #[test]
    fn user_message_with_special_chars() {
        let mut state = AppState::default();
        for c in "/command".chars() {
            state.update(Event::Input(c));
        }
        state.update(Event::Submit);
        let lines = format_messages(&state);
        let content: String = lines[0].spans.iter().map(|s| s.text.clone()).collect();
        assert!(content.contains("/command"), "Should handle special chars");
    }

    #[test]
    fn user_message_clears_input() {
        let mut state = AppState::default();
        state.update(Event::Input('H'));
        state.update(Event::Input('i'));
        state.update(Event::Submit);
        assert!(state.input.is_empty(), "Input should be cleared after submit");
    }

    // === Agent Messages ===

    #[test]
    fn agent_message_has_prefix() {
        let mut state = AppState::default();
        state.update(Event::AgentThinking { id: "req.0".to_string() });
        state.update(Event::AgentThoughtDone { id: "req.0".to_string() });
        state.update(Event::AgentResponse { id: "req.0".to_string(), content: "Hello".to_string() });
        let lines = format_messages(&state);
        let content: String = lines.iter()
            .flat_map(|l| l.spans.iter().map(|s| s.text.clone()).collect::<Vec<_>>())
            .collect();
        assert!(content.contains("Agent:"), "Should have 'Agent:' prefix");
    }

    #[test]
    fn agent_message_contains_content() {
        let mut state = AppState::default();
        state.update(Event::AgentThinking { id: "req.0".to_string() });
        state.update(Event::AgentThoughtDone { id: "req.0".to_string() });
        state.update(Event::AgentResponse { id: "req.0".to_string(), content: "Test response".to_string() });
        let lines = format_messages(&state);
        let content: String = lines.iter()
            .flat_map(|l| l.spans.iter().map(|s| s.text.clone()).collect::<Vec<_>>())
            .collect();
        assert!(content.contains("Test response"), "Should contain response");
    }

    #[test]
    fn agent_message_streaming_chunks_merge() {
        let mut state = AppState::default();
        state.update(Event::AgentThinking { id: "req.0".to_string() });
        state.update(Event::AgentThoughtDone { id: "req.0".to_string() });
        state.update(Event::AgentResponse { id: "req.0".to_string(), content: "Hello ".to_string() });
        state.update(Event::AgentResponse { id: "req.0".to_string(), content: "World".to_string() });
        let lines = format_messages(&state);
        let content: String = lines.iter()
            .flat_map(|l| l.spans.iter().map(|s| s.text.clone()).collect::<Vec<_>>())
            .collect();
        assert!(content.contains("Hello World"), "Streaming chunks should merge");
    }

    // === Tool Execution ===

    #[test]
    fn tool_shows_running_state() {
        let mut state = AppState::default();
        state.update(Event::AgentToolStart { id: "req.0".to_string(), name: "list_files".to_string() });
        let lines = format_messages(&state);
        let content: String = lines.iter()
            .flat_map(|l| l.spans.iter().map(|s| s.text.clone()).collect::<Vec<_>>())
            .collect();
        assert!(content.contains("Running"), "Should show 'Running'");
        assert!(content.contains("list_files"), "Should show tool name");
    }

    #[test]
    fn tool_shows_ran_state() {
        let mut state = AppState::default();
        state.update(Event::AgentToolStart { id: "req.0".to_string(), name: "list_files".to_string() });
        state.update(Event::AgentToolEnd { duration_secs: 0.5 });
        let lines = format_messages(&state);
        let content: String = lines.iter()
            .flat_map(|l| l.spans.iter().map(|s| s.text.clone()).collect::<Vec<_>>())
            .collect();
        assert!(content.contains("Ran"), "Should show 'Ran'");
        assert!(content.contains("0.5s"), "Should show duration");
    }

    #[test]
    fn tool_duration_format() {
        let mut state = AppState::default();
        state.update(Event::AgentToolStart { id: "req.0".to_string(), name: "test".to_string() });
        state.update(Event::AgentToolEnd { duration_secs: 1.23 });
        let lines = format_messages(&state);
        let content: String = lines.iter()
            .flat_map(|l| l.spans.iter().map(|s| s.text.clone()).collect::<Vec<_>>())
            .collect();
        assert!(content.contains("1.2s") || content.contains("1.23s"), "Duration formatted");
    }

    // === Turn Complete ===

    #[test]
    fn turn_complete_shows_for_tool_flow() {
        let mut state = AppState::default();
        state.has_intermediate_steps = true;
        state.update(Event::AgentTurnComplete { id: "req.0".to_string(), duration_secs: 5.1 });
        let lines = format_messages(&state);
        let content: String = lines.iter()
            .flat_map(|l| l.spans.iter().map(|s| s.text.clone()).collect::<Vec<_>>())
            .collect();
        assert!(content.contains("Turn completed"), "Should show turn complete");
        assert!(content.contains("5.1s"), "Should show duration");
    }

    #[test]
    fn turn_complete_hidden_for_simple_flow() {
        let mut state = AppState::default();
        state.has_intermediate_steps = false;
        state.update(Event::AgentTurnComplete { id: "req.0".to_string(), duration_secs: 1.0 });
        let lines = format_messages(&state);
        let content: String = lines.iter()
            .flat_map(|l| l.spans.iter().map(|s| s.text.clone()).collect::<Vec<_>>())
            .collect();
        assert!(!content.contains("Turn completed"), "Simple flow should NOT show turn complete");
    }

    // === Thinking ===

    #[test]
    fn thinking_shows_spinner() {
        let mut state = AppState::default();
        state.streaming = true;
        state.thinking_started_at = Some(std::time::Instant::now());
        let lines = format_messages(&state);
        let content: String = lines.iter()
            .flat_map(|l| l.spans.iter().map(|s| s.text.clone()).collect::<Vec<_>>())
            .collect();
        assert!(content.contains("Though..."), "Should show thinking");
    }

    // === Thought Marker ===

    #[test]
    fn thought_marker_renders() {
        let mut state = AppState::default();
        state.update(Event::AgentThinking { id: "req.0".to_string() });
        state.update(Event::AgentThoughtDone { id: "req.0".to_string() });
        let lines = format_messages(&state);
        let content: String = lines.iter()
            .flat_map(|l| l.spans.iter().map(|s| s.text.clone()).collect::<Vec<_>>())
            .collect();
        assert!(content.contains("Though"), "Should render thought marker");
    }

    // === Multiple Messages ===

    #[test]
    fn multiple_user_messages() {
        let mut state = AppState::default();
        state.update(Event::Input('A'));
        state.update(Event::Submit);
        state.update(Event::Input('B'));
        state.update(Event::Submit);
        let lines = format_messages(&state);
        let you_count = lines.iter()
            .map(|l| l.spans.iter().map(|s| s.text.clone()).collect::<String>())
            .filter(|s| s.contains("You:"))
            .count();
        assert_eq!(you_count, 2, "Should have 2 user messages");
    }

    #[test]
    fn full_conversation_flow() {
        let mut state = AppState::default();
        state.update(Event::Input('H'));
        state.update(Event::Input('i'));
        state.update(Event::Submit);
        state.update(Event::AgentThinking { id: "req.0".to_string() });
        state.update(Event::AgentThoughtDone { id: "req.0".to_string() });
        state.update(Event::AgentResponse { id: "req.0".to_string(), content: "Hello!".to_string() });

        let lines = format_messages(&state);
        let content: String = lines.iter()
            .flat_map(|l| l.spans.iter().map(|s| s.text.clone()).collect::<Vec<_>>())
            .collect();

        assert!(content.contains("You:"), "User message");
        assert!(content.contains("Agent:"), "Agent message");
        assert!(content.contains("Hello!"), "Response");
        assert!(content.contains("Though"), "Thought");
    }

    // === Error ===

    #[test]
    fn error_message_renders() {
        let mut state = AppState::default();
        state.update(Event::AgentError { id: "req.0".to_string(), message: "Something went wrong".to_string() });
        let lines = format_messages(&state);
        let content: String = lines.iter()
            .flat_map(|l| l.spans.iter().map(|s| s.text.clone()).collect::<Vec<_>>())
            .collect();
        assert!(content.contains("Error:"), "Should contain 'Error:'");
        assert!(content.contains("Something went wrong"), "Should show message");
    }

    // === Reset ===

    #[test]
    fn reset_clears_all() {
        let mut state = AppState::default();
        state.update(Event::Input('T'));
        state.update(Event::Submit);
        state.update(Event::AgentThinking { id: "req.0".to_string() });
        state.update(Event::AgentThoughtDone { id: "req.0".to_string() });
        state.update(Event::AgentResponse { id: "req.0".to_string(), content: "Hi".to_string() });
        state.update(Event::Reset);
        assert!(state.messages.is_empty(), "Reset clears messages");
        assert!(state.input.is_empty(), "Reset clears input");
    }

    // === Scroll ===

    #[test]
    fn scroll_up_increments() {
        let mut state = AppState::default();
        state.update(Event::ScrollUp);
        assert_eq!(state.scroll, 1, "Scroll up increments");
    }

    #[test]
    fn scroll_down_decrements() {
        let mut state = AppState::default();
        state.scroll = 5;
        state.update(Event::ScrollDown);
        assert_eq!(state.scroll, 4, "Scroll down decrements");
    }

    #[test]
    fn scroll_down_saturates_at_zero() {
        let mut state = AppState::default();
        state.scroll = 0;
        state.update(Event::ScrollDown);
        assert_eq!(state.scroll, 0, "Scroll stays at 0");
    }

    // === Spacers ===

    #[test]
    fn spacers_between_messages() {
        let mut state = AppState::default();
        state.update(Event::Input('A'));
        state.update(Event::Submit);
        state.update(Event::Input('B'));
        state.update(Event::Submit);
        let lines = format_messages(&state);
        let empty_count = lines.iter().filter(|l| l.spans.is_empty() || l.spans.iter().all(|s| s.text.is_empty())).count();
        assert!(empty_count >= 1, "Should have spacer between messages");
    }
}
