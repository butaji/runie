//! Unit tests for Event Stream

#[cfg(test)]
mod tests {
    use crate::model::AppState;
    use crate::event::Event;
    use crate::update::update;
    
    fn fresh_state() -> AppState {
        AppState::default()
    }
    
    // === Input Events Tests ===
    
    #[test]
    fn test_input_adds_character() {
        let state = fresh_state();
        let state = update(state, Event::Input('H'));
        let state = update(state, Event::Input('i'));
        assert_eq!(state.input, "Hi");
    }
    
    #[test]
    fn test_backspace_removes_character() {
        let state = fresh_state();
        let state = update(state, Event::Input('H'));
        let state = update(state, Event::Input('i'));
        let state = update(state, Event::Backspace);
        assert_eq!(state.input, "H");
    }
    
    #[test]
    fn test_backspace_empty_input() {
        let state = fresh_state();
        let state = update(state, Event::Backspace);
        assert_eq!(state.input, "");
    }
    
    #[test]
    fn test_submit_empty_input() {
        let state = fresh_state();
        let state = update(state, Event::Submit);
        assert_eq!(state.input, "");
    }
    
    #[test]
    fn test_submit_reset_command() {
        let state = update(update(fresh_state(), Event::Input('/')), Event::Input('r'));
        let state = update(state, Event::Input('e'));
        let state = update(state, Event::Input('s'));
        let state = update(state, Event::Input('e'));
        let state = update(state, Event::Input('t'));
        let state = update(state, Event::Submit);
        
        assert_eq!(state.messages.len(), 0);
        assert_eq!(state.input, "");
    }
    
    // === Agent Events Tests ===
    
    #[test]
    fn test_agent_thinking_sets_streaming() {
        let mut state = fresh_state();
        state.streaming = true;
        let state = update(state, Event::AgentThinking { id: "req.0".to_string() });
        
        assert!(state.streaming);
        assert!(state.thinking_started_at.is_some());
    }
    
    #[test]
    fn test_agent_response_creates_message() {
        let mut state = fresh_state();
        state.streaming = true;
        
        let state = update(state, Event::AgentThinking { id: "req.0".to_string() });
        let state = update(state, Event::AgentThoughtDone { id: "req.0".to_string() });
        let state = update(state, Event::AgentResponse { 
            id: "req.0".to_string(),
            content: "Hello".to_string() 
        });
        
        assert_eq!(state.messages.len(), 2);
        assert_eq!(state.messages[1].role, "assistant");
        assert_eq!(state.messages[1].content, "Hello");
    }
    
    #[test]
    fn test_agent_response_creates_multiple_messages() {
        let mut state = fresh_state();
        state.streaming = true;
        
        let state = update(state, Event::AgentThinking { id: "req.0".to_string() });
        let state = update(state, Event::AgentThoughtDone { id: "req.0".to_string() });
        let state = update(state, Event::AgentResponse { 
            id: "req.0".to_string(),
            content: "Hello ".to_string() 
        });
        let state = update(state, Event::AgentResponse { 
            id: "req.0".to_string(),
            content: "World".to_string() 
        });
        
        assert_eq!(state.messages.len(), 3);
        assert_eq!(state.messages[0].role, "thought");
        assert_eq!(state.messages[1].role, "assistant");
        assert_eq!(state.messages[2].role, "assistant");
    }
    
    #[test]
    fn test_dsl_combines_consecutive_agent_chunks() {
        use crate::ui::Dsl;
        use crate::ui::elements::Element;
        
        let mut state = fresh_state();
        state.streaming = true;
        
        let state = update(state, Event::AgentThinking { id: "req.0".to_string() });
        let state = update(state, Event::AgentThoughtDone { id: "req.0".to_string() });
        let state = update(state, Event::AgentResponse { 
            id: "req.0".to_string(),
            content: "Hello ".to_string() 
        });
        let state = update(state, Event::AgentResponse { 
            id: "req.0".to_string(),
            content: "World!".to_string() 
        });
        
        let feed = Dsl::feed(&state);
        
        assert_eq!(feed.elements.len(), 4);
        
        if let Element::AgentMessage { content, .. } = &feed.elements[2] {
            assert_eq!(content, "Hello World!");
        } else {
            panic!("Expected AgentMessage at index 2");
        }
    }
    
    #[test]
    fn test_agent_done_clears_streaming() {
        let mut state = fresh_state();
        state.streaming = true;
        state.thinking_started_at = Some(std::time::Instant::now());
        
        let state = update(state, Event::AgentDone { id: "req.0".to_string() });
        
        assert!(!state.streaming);
        assert!(state.thinking_started_at.is_none());
    }
    
    #[test]
    fn test_agent_error_creates_error_message() {
        let mut state = fresh_state();
        state.streaming = true;
        
        let state = update(state, Event::AgentError { 
            id: "req.0".to_string(),
            message: "Something went wrong".to_string() 
        });
        
        assert!(!state.streaming);
        assert_eq!(state.messages.len(), 1);
        assert_eq!(state.messages[0].role, "assistant");
        assert!(state.messages[0].content.contains("Error"));
    }
    
    // === Reset Event Tests ===
    
    #[test]
    fn test_reset_clears_state() {
        let mut state = fresh_state();
        state.input = "test".to_string();
        state.streaming = true;
        
        let state = update(state, Event::Reset);
        
        assert_eq!(state.input, "");
        assert!(!state.streaming);
        assert_eq!(state.messages.len(), 0);
    }
    
    // === Scroll Events Tests ===
    
    #[test]
    fn test_scroll_up() {
        let state = fresh_state();
        let state = update(state, Event::ScrollUp);
        assert_eq!(state.scroll, 1);
    }
    
    #[test]
    fn test_scroll_down() {
        let mut state = fresh_state();
        state.scroll = 5;
        let state = update(state, Event::ScrollDown);
        assert_eq!(state.scroll, 4);
    }
    
    #[test]
    fn test_scroll_down_saturates() {
        let mut state = fresh_state();
        state.scroll = 0;
        let state = update(state, Event::ScrollDown);
        assert_eq!(state.scroll, 0);
    }
    
    // === Correlation ID Tests ===
    
    #[test]
    fn test_messages_have_correlation_id() {
        let state = fresh_state();
        let state = update(state, Event::Input('H'));
        let state = update(state, Event::Submit);
        
        assert_eq!(state.messages.len(), 1);
        assert!(state.messages[0].id.starts_with("req."));
    }
    
    #[test]
    fn test_multiple_submits_increment_id() {
        let state = fresh_state();
        
        let state = update(state, Event::Input('A'));
        let state = update(state, Event::Submit);
        let first_id = state.messages[0].id.clone();
        
        let state = update(state, Event::Input('B'));
        let state = update(state, Event::Submit);
        let second_id = state.messages[1].id.clone();
        
        assert_ne!(first_id, second_id);
    }
    
    // === Full Flow Tests ===
    
    #[test]
    fn test_complete_agent_flow() {
        let mut state = fresh_state();
        state = update(state, Event::Input('H'));
        state = update(state, Event::Submit);
        
        assert_eq!(state.messages.len(), 1);
        assert_eq!(state.messages[0].role, "user");
        assert!(!state.streaming);
        
        state.pop_queue();
        state.streaming = true;
        
        let state = update(state, Event::AgentThinking { id: "req.0".to_string() });
        assert!(state.streaming);
        
        let state = update(state, Event::AgentThoughtDone { id: "req.0".to_string() });
        let state = update(state, Event::AgentResponse { 
            id: "req.0".to_string(),
            content: "Hello".to_string() 
        });
        
        assert_eq!(state.messages.len(), 3);
        assert_eq!(state.messages[1].role, "thought");
        assert_eq!(state.messages[2].role, "assistant");
        
        let state = update(state, Event::AgentDone { id: "req.0".to_string() });
        assert!(!state.streaming);
    }
    
    #[test]
    fn test_queue_processing() {
        let state = fresh_state();
        let state = update(state, Event::Input('A'));
        let state = update(state, Event::Submit);
        let state = update(state, Event::Input('B'));
        let state = update(state, Event::Submit);
        
        assert_eq!(state.messages.len(), 2);
        assert_eq!(state.request_queue.len(), 2);
        assert!(!state.streaming);
    }
    
    #[test]
    fn test_submit_adds_message_to_queue() {
        let state = update(update(fresh_state(), Event::Input('H')), Event::Submit);
        
        assert_eq!(state.messages.len(), 1);
        assert_eq!(state.messages[0].role, "user");
        assert_eq!(state.request_queue.len(), 1);
    }
    
    // === DSL Tests ===
    
    #[test]
    fn test_thinking_indicator_shows_for_queued_request() {
        let mut state = fresh_state();
        state.streaming = true;
        state.request_queue.push(("B".to_string(), "req.1".to_string()));
        state.thinking_started_at = Some(std::time::Instant::now());
        
        let has_thought = state.messages.iter().any(|m| m.role == "thought");
        assert!(!has_thought);
    }
    
    #[test]
    fn test_dsl_shows_thinking_when_streaming() {
        use crate::ui::format_messages;
        
        let mut state = fresh_state();
        state.streaming = true;
        state.request_queue.push(("B".to_string(), "req.1".to_string()));
        state.thinking_started_at = Some(std::time::Instant::now());
        
        let lines = format_messages(&state);
        let content: String = lines.iter()
            .flat_map(|l| l.spans.iter().map(|s| s.text.clone()).collect::<Vec<_>>())
            .collect();
        
        assert!(content.contains("Though"));
    }
    
    #[test]
    fn test_multiple_thoughts_for_sequential_requests() {
        let mut state = fresh_state();
        
        // Request A
        state.streaming = true;
        state = update(state, Event::AgentThinking { id: "req.0".to_string() });
        state = update(state, Event::AgentThoughtDone { id: "req.0".to_string() });
        state = update(state, Event::AgentResponse { id: "req.0".to_string(), content: "A".to_string() });
        state = update(state, Event::AgentDone { id: "req.0".to_string() });
        
        // Request B
        state = update(state, Event::AgentThinking { id: "req.1".to_string() });
        state = update(state, Event::AgentThoughtDone { id: "req.1".to_string() });
        state = update(state, Event::AgentResponse { id: "req.1".to_string(), content: "B".to_string() });
        
        let thoughts: Vec<_> = state.messages.iter().filter(|m| m.role == "thought").collect();
        assert_eq!(thoughts.len(), 2);
    }
    
    // === Tool Execution Tests ===
    
    #[test]
    fn test_tool_flow_creates_two_thoughts() {
        let mut state = fresh_state();
        state.streaming = true;
        
        // First thought
        state = update(state, Event::AgentThinking { id: "req.0".to_string() });
        state = update(state, Event::AgentThoughtDone { id: "req.0".to_string() });
        
        // Tool execution
        state = update(state, Event::AgentToolStart { 
            id: "req.0".to_string(), 
            name: "list_files".to_string(),
        });
        state = update(state, Event::AgentToolEnd { 
            duration_secs: 0.5,
        });
        
        // Second thought
        state = update(state, Event::AgentThinking { id: "req.0".to_string() });
        state = update(state, Event::AgentThoughtDone { id: "req.0".to_string() });
        
        // Response
        state = update(state, Event::AgentResponse { 
            id: "req.0".to_string(), 
            content: "Here are the files".to_string() 
        });
        
        let thought_count = state.messages.iter().filter(|m| m.role == "thought").count();
        assert_eq!(thought_count, 2);
    }
    
    #[test]
    fn test_turn_complete_event() {
        let state = update(fresh_state(), Event::AgentTurnComplete { 
            id: "req.0".to_string(), 
            duration_secs: 5.1 
        });
        
        assert_eq!(state.messages.len(), 1);
        let msg = &state.messages[0];
        assert_eq!(msg.role, "turn_complete");
        assert!(msg.content.contains("5.1s"));
    }
    
    #[test]
    fn test_tool_done_event() {
        let state = update(update(fresh_state(), Event::AgentToolStart { 
            id: "req.0".to_string(), 
            name: "list_files".to_string(),
        }), Event::AgentToolEnd { 
            duration_secs: 0.3,
        });
        
        assert_eq!(state.messages.len(), 1);
        let msg = &state.messages[0];
        assert_eq!(msg.role, "tool");
        assert!(msg.content.contains("list_files"));
        assert!(msg.content.contains("0.3s"));
    }
    
    #[test]
    fn test_formatted_labels_short_names() {
        use crate::ui::format_messages;
        
        let mut state = fresh_state();
        state.streaming = true;
        
        // Tool execution (after end, shows Ran not Running)
        state = update(state, Event::AgentToolStart { 
            id: "req.0".to_string(), 
            name: "list_files".to_string(),
        });
        state = update(state, Event::AgentToolEnd { 
            duration_secs: 0.3,
        });
        
        // Debug: print message content
        for msg in &state.messages {
            eprintln!("MSG role={} content={}", msg.role, msg.content);
        }
        
        // Turn complete
        state = update(state, Event::AgentTurnComplete { 
            id: "req.0".to_string(), 
            duration_secs: 5.1 
        });
        
        let lines = format_messages(&state);
        let content: String = lines.iter()
            .flat_map(|l| l.spans.iter().map(|s| s.text.clone()).collect::<Vec<_>>())
            .collect();
        
        // After tool end, shows "Ran" not "Running"
        let has_ran = content.contains("Ran");
        let has_duration = content.contains("0.3s");
        let has_turn = content.contains("Turn completed");
        assert!(has_ran, "Missing 'Ran' in: {}", content);
        assert!(has_duration, "Missing '0.3s' in: {}", content);
        assert!(has_turn, "Missing 'Turn completed' in: {}", content);
    }
    
    #[test]
    fn test_list_files_full_tool_flow_sequence() {
        use crate::ui::format_messages;
        
        let mut state = fresh_state();
        state.streaming = true;
        
        // 1. Though (first thinking done)
        state = update(state, Event::AgentThinking { id: "req.0".to_string() });
        state = update(state, Event::AgentThoughtDone { id: "req.0".to_string() });
        
        // 2. Running -> Ran
        state = update(state, Event::AgentToolStart { 
            id: "req.0".to_string(), 
            name: "list_files".to_string(),
        });
        state = update(state, Event::AgentToolEnd { 
            duration_secs: 0.5,
        });
        
        // 3. Though (second thinking done)
        state = update(state, Event::AgentThinking { id: "req.0".to_string() });
        state = update(state, Event::AgentThoughtDone { id: "req.0".to_string() });
        
        // 4. Response
        state = update(state, Event::AgentResponse { 
            id: "req.0".to_string(), 
            content: "Here are the files:".to_string() 
        });
        
        // 5. Turn complete
        state = update(state, Event::AgentTurnComplete { 
            id: "req.0".to_string(), 
            duration_secs: 5.1 
        });
        
        // Verify: thought1, tool, thought2, assistant, turn_complete = 5
        assert_eq!(state.messages.len(), 5);
        assert_eq!(state.messages[0].role, "thought");
        assert_eq!(state.messages[1].role, "tool");
        assert_eq!(state.messages[2].role, "thought");
        assert_eq!(state.messages[3].role, "assistant");
        assert_eq!(state.messages[4].role, "turn_complete");
        
        let lines = format_messages(&state);
        let content: String = lines.iter()
            .flat_map(|l| l.spans.iter().map(|s| s.text.clone()).collect::<Vec<_>>())
            .collect();
        
        // Check final content (Ran not Running after tool end)
        assert!(content.contains("Though"));
        assert!(content.contains("Ran"));
        assert!(content.contains("list_files"));
        assert!(content.contains("Agent:"));
        assert!(content.contains("Turn completed in 5.1s"));
    }
}
