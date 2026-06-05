//! Unit tests for Event Stream
//! 
//! These tests verify that events correctly transform state.

#[cfg(test)]
mod tests {
    use crate::model::AppState;
    use crate::event::Event;
    use crate::update::update;
    
    /// Helper to create fresh state
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
    
    // === Submit Events Tests ===
    
    #[test]
    fn test_submit_adds_message_to_queue() {
        let state = fresh_state();
        let state = update(state, Event::Input('H'));
        let state = update(state, Event::Input('i'));
        let state = update(state, Event::Submit);
        
        // Input should be cleared
        assert_eq!(state.input, "");
        
        // Should have one message
        assert_eq!(state.messages.len(), 1);
        assert_eq!(state.messages[0].role, "user");
        assert_eq!(state.messages[0].content, "Hi");
        
        // Should be in queue
        assert_eq!(state.request_queue.len(), 1);
        // streaming is false until agent actually starts (AgentThinking fires)
        assert!(!state.streaming);
    }
    
    #[test]
    fn test_submit_empty_input() {
        let state = fresh_state();
        let state = update(state, Event::Submit);
        assert_eq!(state.messages.len(), 0);
        assert!(!state.streaming);
    }
    
    #[test]
    fn test_submit_reset_command() {
        let state = fresh_state();
        let state = update(state, Event::Input('/'));
        let state = update(state, Event::Input('r'));
        let state = update(state, Event::Input('e'));
        let state = update(state, Event::Input('s'));
        let state = update(state, Event::Input('e'));
        let state = update(state, Event::Input('t'));
        let state = update(state, Event::Submit);
        
        // State should be reset
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
        state.thinking_started_at = Some(std::time::Instant::now());
        
        let state = update(state, Event::AgentResponse { 
            id: "req.0".to_string(),
            content: "Hello".to_string() 
        });
        
        assert_eq!(state.messages.len(), 2); // thought + agent
        assert_eq!(state.messages[1].role, "assistant");
        assert_eq!(state.messages[1].content, "Hello");
    }
    
    #[test]
    fn test_agent_response_creates_multiple_messages() {
        let mut state = fresh_state();
        state.streaming = true;
        state.thinking_started_at = Some(std::time::Instant::now());
        
        let state = update(state, Event::AgentResponse { 
            id: "req.0".to_string(),
            content: "Hello ".to_string() 
        });
        let state = update(state, Event::AgentResponse { 
            id: "req.0".to_string(),
            content: "World".to_string() 
        });
        
        // Each response adds a separate message
        // Message order: thought, agent1, agent2
        assert_eq!(state.messages.len(), 3);
        assert_eq!(state.messages[0].role, "thought");
        assert_eq!(state.messages[1].role, "assistant");
        assert_eq!(state.messages[1].content, "Hello ");
        assert_eq!(state.messages[2].role, "assistant");
        assert_eq!(state.messages[2].content, "World");
    }
    
    #[test]
    fn test_dsl_combines_consecutive_agent_chunks() {
        use crate::ui::Dsl;
        use crate::ui::elements::Element;
        
        let mut state = fresh_state();
        state.streaming = true;
        state.thinking_started_at = Some(std::time::Instant::now());
        state.current_request_id = Some("req.0".to_string());
        
        let state = update(state, Event::AgentResponse { 
            id: "req.0".to_string(),
            content: "Hello ".to_string() 
        });
        let state = update(state, Event::AgentResponse { 
            id: "req.0".to_string(),
            content: "World!".to_string() 
        });
        
        let feed = Dsl::feed(&state);
        
        // Should have: ThoughtMarker, Spacer, AgentMessage, Spacer = 4 elements
        assert_eq!(feed.elements.len(), 4);
        
        // Check that AgentMessage is combined
        if let Element::AgentMessage { content, .. } = &feed.elements[2] {
            assert_eq!(content, "Hello World!", "Chunks should be combined");
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
        
        // First submit
        let state = update(state, Event::Input('A'));
        let state = update(state, Event::Submit);
        let first_id = state.messages[0].id.clone();
        
        // Second submit
        let state = update(state, Event::Input('B'));
        let state = update(state, Event::Submit);
        let second_id = state.messages[1].id.clone();
        
        assert_ne!(first_id, second_id);
    }
    
    // === Full Flow Tests ===
    
    #[test]
    fn test_complete_agent_flow() {
        // Simulate complete flow: submit -> agent starts -> response -> done
        let mut state = fresh_state();
        state = update(state, Event::Input('H'));
        state = update(state, Event::Submit);
        
        // State should have user message in queue
        assert_eq!(state.messages.len(), 1);
        assert_eq!(state.messages[0].role, "user");
        // streaming is false until agent actually starts
        assert!(!state.streaming);
        
        // Agent starts (main.rs pops queue and spawns)
        state.pop_queue();
        state.streaming = true;
        
        // Agent thinks
        let state = update(state, Event::AgentThinking { id: "req.0".to_string() });
        assert!(state.streaming);
        
        // Agent responds
        let state = update(state, Event::AgentResponse { 
            id: "req.0".to_string(),
            content: "Hello".to_string() 
        });
        
        // Should have thought + agent message
        assert_eq!(state.messages.len(), 3); // user, thought, agent
        assert_eq!(state.messages[1].role, "thought");
        assert_eq!(state.messages[2].role, "assistant");
        assert_eq!(state.messages[2].content, "Hello");
        
        // Agent done - queue is empty, so streaming becomes false
        let state = update(state, Event::AgentDone { id: "req.0".to_string() });
        assert!(!state.streaming);
    }
    
    #[test]
    fn test_queue_processing() {
        // Submit two messages
        let state = fresh_state();
        let state = update(state, Event::Input('A'));
        let state = update(state, Event::Submit);
        let state = update(state, Event::Input('B'));
        let state = update(state, Event::Submit);
        
        // Should have 2 user messages
        assert_eq!(state.messages.len(), 2);
        assert_eq!(state.messages[0].content, "A");
        assert_eq!(state.messages[1].content, "B");
        
        // Queue should have 2 items
        assert_eq!(state.request_queue.len(), 2);
        // streaming is false until agent actually starts
        assert!(!state.streaming);
    }
    
    #[test]
    fn test_multiple_thoughts_for_sequential_requests() {
        // Simulate sequential FIFO: A finishes, then B, then C
        let mut state = fresh_state();
        
        // === Request A ===
        state.streaming = true;
        state.thinking_started_at = Some(std::time::Instant::now());
        
        let state = update(state, Event::AgentThinking { id: "req.0".to_string() });
        let state = update(state, Event::AgentResponse { 
            id: "req.0".to_string(), 
            content: "A".to_string() 
        });
        let state = update(state, Event::AgentDone { id: "req.0".to_string() });
        
        // === Request B (immediately after A done) ===
        let state = update(state, Event::AgentThinking { id: "req.1".to_string() });
        let state = update(state, Event::AgentResponse { 
            id: "req.1".to_string(), 
            content: "B".to_string() 
        });
        let state = update(state, Event::AgentDone { id: "req.1".to_string() });
        
        // === Request C (immediately after B done) ===
        let state = update(state, Event::AgentThinking { id: "req.2".to_string() });
        let state = update(state, Event::AgentResponse { 
            id: "req.2".to_string(), 
            content: "C".to_string() 
        });
        let state = update(state, Event::AgentDone { id: "req.2".to_string() });
        
        // Should have 6 messages: 3 thoughts + 3 agents
        assert_eq!(state.messages.len(), 6);
        
        // Check all 3 thoughts exist with correct IDs
        let thoughts: Vec<_> = state.messages.iter().filter(|m| m.role == "thought").collect();
        assert_eq!(thoughts.len(), 3, "Should have 3 thoughts");
        assert_eq!(thoughts[0].id, "req.0");
        assert_eq!(thoughts[1].id, "req.1");
        assert_eq!(thoughts[2].id, "req.2");
        
        // Check order: thought, agent, thought, agent, thought, agent
        let roles: Vec<_> = state.messages.iter().map(|m| m.role.as_str()).collect();
        assert_eq!(roles, vec!["thought", "assistant", "thought", "assistant", "thought", "assistant"]);
    }
    
    #[test]
    fn test_thinking_indicator_shows_for_queued_request() {
        // Submit first message
        let mut state = fresh_state();
        state.streaming = true;
        
        // Submit second message while first is processing
        let state = update(state, Event::Input('B'));
        let state = update(state, Event::Submit);
        
        // Queue should have 1 item (first was popped for processing)
        assert_eq!(state.request_queue.len(), 1);
        assert!(state.streaming);
        
        // There should be no thought yet for the current request
        let has_thought = state.messages.iter().any(|m| m.role == "thought");
        assert!(!has_thought, "No thought should exist yet for current request");
    }
    
    #[test]
    fn test_dsl_shows_thinking_when_streaming() {
        use crate::ui::format_messages;
        
        // Simulate queued request: streaming=true, queue has items, no thought yet
        let mut state = fresh_state();
        state.streaming = true;
        state.request_queue.push(("B".to_string(), "req.1".to_string()));
        
        let lines = format_messages(&state);
        let content: String = lines.iter()
            .flat_map(|l| l.spans.iter().map(|s| s.text.clone()).collect::<Vec<_>>())
            .collect();
        
        assert!(content.contains("Thinking"), "Should show Thinking when streaming with no thought. Content: {}", content);
    }
    
    // === Tool Execution Tests ===
    
    #[test]
    fn test_tool_flow_creates_single_thought() {
        let mut state = fresh_state();
        state.streaming = true;
        state.thinking_started_at = Some(std::time::Instant::now());
        
        // First thinking phase
        state = update(state, Event::AgentThinking { id: "req.0".to_string() });
        
        // Tool execution
        state = update(state, Event::AgentToolStart { 
            id: "req.0".to_string(), 
            name: "list_files".to_string() 
        });
        state = update(state, Event::AgentToolEnd { 
            id: "req.0".to_string(), 
            name: "list_files".to_string(), 
            output: "file1.rs\nfile2.rs".to_string() 
        });
        
        // Second thinking phase (should NOT create another thought)
        state = update(state, Event::AgentThinking { id: "req.0".to_string() });
        
        // Response
        state = update(state, Event::AgentResponse { 
            id: "req.0".to_string(), 
            content: "Here are the files".to_string() 
        });
        
        // Should have exactly one thought marker
        let thought_count = state.messages.iter().filter(|m| m.role == "thought").count();
        assert_eq!(thought_count, 1, "Should have exactly one thought marker for tool flow");
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
    fn test_tool_events_added_to_messages() {
        let mut state = fresh_state();
        state.streaming = true;
        
        // Tool start
        state = update(state, Event::AgentToolStart { 
            id: "req.0".to_string(), 
            name: "list_files".to_string() 
        });
        
        // Tool end
        state = update(state, Event::AgentToolEnd { 
            id: "req.0".to_string(), 
            name: "list_files".to_string(), 
            output: "README.md\nCargo.toml".to_string() 
        });
        
        assert_eq!(state.messages.len(), 2);
        assert!(state.messages[0].content.contains("list_files"));
        assert!(state.messages[1].content.contains("README.md"));
    }
}
