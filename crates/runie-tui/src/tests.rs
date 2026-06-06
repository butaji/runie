//! Chat rendering tests - ensure UI never breaks
#[cfg(test)]
mod tests {
    use runie_core::{AppState, Event};
    use runie_core::ui::format_messages;

    // === Empty State Tests ===

    #[test]
    fn empty_state_renders_nothing() {
        let state = AppState::default();
        let lines = format_messages(&state);
        assert!(lines.is_empty(), "Empty state should have no lines");
    }

    // === User Message Tests ===

    #[test]
    fn user_message_has_prefix() {
        let mut state = AppState::default();
        state.update(Event::Input('H'));
        state.update(Event::Submit);
        let lines = format_messages(&state);
        let content: String = lines[0].spans.iter().map(|s| s.text.clone()).collect();
        assert!(content.starts_with("You:"), "User message should start with 'You:' prefix");
    }

    #[test]
    fn user_message_contains_input() {
        let mut state = AppState::default();
        state.update(Event::Input('H'));
        state.update(Event::Input('i'));
        state.update(Event::Submit);
        let lines = format_messages(&state);
        let content: String = lines[0].spans.iter().map(|s| s.text.clone()).collect();
        assert!(content.contains("Hi"), "User message should contain input 'Hi'");
    }

    #[test]
    fn user_message_with_special_chars() {
        let mut state = AppState::default();
        state.update(Event::Input('/'));
        state.update(Event::Input('c'));
        state.update(Event::Input('o'));
        state.update(Event::Input('m'));
        state.update(Event::Input('m'));
        state.update(Event::Input('a'));
        state.update(Event::Input('n'));
        state.update(Event::Input('d'));
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

    // === Agent Message Tests ===

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
        assert!(content.contains("Agent:"), "Agent message should have 'Agent:' prefix");
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
        assert!(content.contains("Test response"), "Agent message should contain response");
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

    // === Tool Execution Tests ===

    #[test]
    fn tool_shows_running_state() {
        let mut state = AppState::default();
        state.update(Event::AgentToolStart { id: "req.0".to_string(), name: "list_files".to_string() });
        let lines = format_messages(&state);
        let content: String = lines.iter()
            .flat_map(|l| l.spans.iter().map(|s| s.text.clone()).collect::<Vec<_>>())
            .collect();
        assert!(content.contains("Running"), "Should show 'Running' state");
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
        assert!(content.contains("Ran"), "Should show 'Ran' state after completion");
        assert!(content.contains("list_files"), "Should show tool name");
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
        assert!(content.contains("1.2s") || content.contains("1.23s"), "Duration should be formatted");
    }

    // === Turn Complete Tests ===

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
        state.has_intermediate_steps = false; // No tool was run
        state.update(Event::AgentTurnComplete { id: "req.0".to_string(), duration_secs: 1.0 });
        let lines = format_messages(&state);
        let content: String = lines.iter()
            .flat_map(|l| l.spans.iter().map(|s| s.text.clone()).collect::<Vec<_>>())
            .collect();
        assert!(!content.contains("Turn completed"), "Simple flow should NOT show turn complete");
    }

    // === Thinking Indicator Tests ===

    #[test]
    fn thinking_shows_spinner() {
        let mut state = AppState::default();
        state.streaming = true;
        state.thinking_started_at = Some(std::time::Instant::now());
        let lines = format_messages(&state);
        let content: String = lines.iter()
            .flat_map(|l| l.spans.iter().map(|s| s.text.clone()).collect::<Vec<_>>())
            .collect();
        assert!(content.contains("Though..."), "Should show thinking indicator");
    }

    #[test]
    fn thinking_shows_elapsed_time() {
        let mut state = AppState::default();
        state.streaming = true;
        // Manually set to test rendering
        state.thinking_started_at = Some(std::time::Instant::now());
        let lines = format_messages(&state);
        let content: String = lines.iter()
            .flat_map(|l| l.spans.iter().map(|s| s.text.clone()).collect::<Vec<_>>())
            .collect();
        // Should contain "s" for seconds
        assert!(content.contains("s"), "Should show elapsed seconds");
    }

    // === Thought Marker Tests ===

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

    // === Multiple Messages Tests ===

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
        // User message
        state.update(Event::Input('H'));
        state.update(Event::Input('i'));
        state.update(Event::Submit);
        // Agent thinking
        state.update(Event::AgentThinking { id: "req.0".to_string() });
        state.update(Event::AgentThoughtDone { id: "req.0".to_string() });
        // Agent response
        state.update(Event::AgentResponse { id: "req.0".to_string(), content: "Hello!".to_string() });

        let lines = format_messages(&state);
        let content: String = lines.iter()
            .flat_map(|l| l.spans.iter().map(|s| s.text.clone()).collect::<Vec<_>>())
            .collect();

        assert!(content.contains("You:"), "Should have user message");
        assert!(content.contains("Agent:"), "Should have agent message");
        assert!(content.contains("Hello!"), "Should have response");
        assert!(content.contains("Though"), "Should have thought");
    }

    // === Error Handling Tests ===

    #[test]
    fn error_message_renders() {
        let mut state = AppState::default();
        state.update(Event::AgentError { id: "req.0".to_string(), message: "Something went wrong".to_string() });
        let lines = format_messages(&state);
        let content: String = lines.iter()
            .flat_map(|l| l.spans.iter().map(|s| s.text.clone()).collect::<Vec<_>>())
            .collect();
        assert!(content.contains("Error:"), "Error message should contain 'Error:'");
        assert!(content.contains("Something went wrong"), "Should show error message");
    }

    // === Reset Tests ===

    #[test]
    fn reset_clears_all() {
        let mut state = AppState::default();
        state.update(Event::Input('T'));
        state.update(Event::Submit);
        state.update(Event::AgentThinking { id: "req.0".to_string() });
        state.update(Event::AgentThoughtDone { id: "req.0".to_string() });
        state.update(Event::AgentResponse { id: "req.0".to_string(), content: "Hi".to_string() });
        state.update(Event::Reset);
        assert!(state.messages.is_empty(), "Reset should clear messages");
        assert!(state.input.is_empty(), "Reset should clear input");
    }

    // === Scroll Tests ===

    #[test]
    fn scroll_up_increments() {
        let mut state = AppState::default();
        state.update(Event::ScrollUp);
        assert_eq!(state.scroll, 1, "Scroll up should increment");
    }

    #[test]
    fn scroll_down_decrements() {
        let mut state = AppState::default();
        state.scroll = 5;
        state.update(Event::ScrollDown);
        assert_eq!(state.scroll, 4, "Scroll down should decrement");
    }

    #[test]
    fn scroll_down_saturates_at_zero() {
        let mut state = AppState::default();
        state.scroll = 0;
        state.update(Event::ScrollDown);
        assert_eq!(state.scroll, 0, "Scroll should not go below 0");
    }

    // === Spacer Tests ===

    #[test]
    fn spacers_between_messages() {
        let mut state = AppState::default();
        state.update(Event::Input('A'));
        state.update(Event::Submit);
        state.update(Event::Input('B'));
        state.update(Event::Submit);
        // Each message should be followed by a spacer (empty line)
        let lines = format_messages(&state);
        let empty_count = lines.iter().filter(|l| l.spans.is_empty() || l.spans.iter().all(|s| s.text.is_empty())).count();
        assert!(empty_count >= 1, "Should have spacer between messages");
    }
}
