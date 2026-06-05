//! End-to-end tests for the terminal application
//! 
//! Tests the complete flow: user input -> state update -> UI rendering

use runie_core::{AppState, Event, update::update};
use runie_tui::ui::view;
use ratatui::{backend::TestBackend, Terminal};

// Skip delays in tests
fn set_test_mode() {
    std::env::set_var("RUNIE_TEST", "1");
}

/// Test: User submits input -> message added to queue
#[test]
fn test_submit_adds_message_to_queue() {
    set_test_mode();
    let state = update(update(update(AppState::default(), Event::Input('H')), Event::Input('i')), Event::Submit);
    
    assert_eq!(state.input, "");
    assert_eq!(state.messages.len(), 1);
    assert_eq!(state.messages[0].role, "user");
    assert_eq!(state.messages[0].content, "Hi");
    assert_eq!(state.request_queue.len(), 1);
    assert!(!state.streaming); // streaming false until agent starts
}

/// Test: Agent thinking sets streaming state
#[test]
fn test_agent_thinking_sets_streaming() {
    set_test_mode();
    let state = update(update(AppState::default(), Event::Submit), Event::AgentThinking { id: "req.0".to_string() });
    
    assert!(state.streaming);
    assert!(state.thinking_started_at.is_some());
}

/// Test: Agent response creates messages
#[test]
fn test_agent_response_creates_messages() {
    set_test_mode();
    let mut state = AppState::default();
    state.streaming = true;
    state.thinking_started_at = Some(std::time::Instant::now());
    
    let state = update(state, Event::AgentThinking { id: "req.0".to_string() });
    let state = update(state, Event::AgentResponse { id: "req.0".to_string(), content: "Hello".to_string() });
    
    // Should have thought + agent message
    assert_eq!(state.messages.len(), 2);
    assert_eq!(state.messages[0].role, "thought");
    assert_eq!(state.messages[1].role, "assistant");
    assert_eq!(state.messages[1].content, "Hello");
}

/// Test: Agent done clears streaming when queue empty
#[test]
fn test_agent_done_clears_streaming() {
    set_test_mode();
    let mut state = AppState::default();
    state.streaming = true;
    state.thinking_started_at = Some(std::time::Instant::now());
    
    let state = update(state, Event::AgentThinking { id: "req.0".to_string() });
    let state = update(state, Event::AgentResponse { id: "req.0".to_string(), content: "Hi".to_string() });
    let state = update(state, Event::AgentDone { id: "req.0".to_string() });
    
    assert!(!state.streaming);
    assert!(state.thinking_started_at.is_none());
}

/// Test: Sequential FIFO - A finishes, then B
#[test]
fn test_sequential_fifo_a_then_b() {
    set_test_mode();
    let mut state = AppState::default();
    
    // Submit A
    state = update(state, Event::Input('A'));
    state = update(state, Event::Submit);
    
    // Pop queue and start A
    state.pop_queue();
    state.streaming = true;
    state = update(state, Event::AgentThinking { id: "req.0".to_string() });
    state = update(state, Event::AgentResponse { id: "req.0".to_string(), content: "Response A".to_string() });
    state = update(state, Event::AgentDone { id: "req.0".to_string() });
    
    // Now B starts
    state = update(state, Event::AgentThinking { id: "req.1".to_string() });
    state = update(state, Event::AgentResponse { id: "req.1".to_string(), content: "Response B".to_string() });
    state = update(state, Event::AgentDone { id: "req.1".to_string() });
    
    // Should have 2 thoughts and 2 responses
    let thoughts: Vec<_> = state.messages.iter().filter(|m| m.role == "thought").collect();
    let agents: Vec<_> = state.messages.iter().filter(|m| m.role == "assistant").collect();
    
    assert_eq!(thoughts.len(), 2);
    assert_eq!(agents.len(), 2);
    assert_eq!(thoughts[0].id, "req.0");
    assert_eq!(thoughts[1].id, "req.1");
}

/// Test: Render user message in UI
#[test]
fn test_render_user_message() {
    set_test_mode();
    let backend = TestBackend::new(60, 20);
    let mut terminal = Terminal::new(backend).unwrap();
    
    let state = update(update(update(AppState::default(), Event::Input('H')), Event::Input('i')), Event::Submit);
    terminal.draw(|f| view(f, &state)).unwrap();
    
    let buf = terminal.backend().buffer();
    let content: String = buf.content.iter().map(|c| c.symbol()).collect();
    
    assert!(content.contains("You: Hi"), "Should show user message");
}

/// Test: Render agent response in UI
#[test]
fn test_render_agent_response() {
    set_test_mode();
    let backend = TestBackend::new(60, 20);
    let mut terminal = Terminal::new(backend).unwrap();
    
    let mut state = AppState::default();
    state.streaming = true;
    state.thinking_started_at = Some(std::time::Instant::now());
    
    state = update(state, Event::AgentThinking { id: "req.0".to_string() });
    state = update(state, Event::AgentResponse { id: "req.0".to_string(), content: "Hello".to_string() });
    
    terminal.draw(|f| view(f, &state)).unwrap();
    
    let buf = terminal.backend().buffer();
    let content: String = buf.content.iter().map(|c| c.symbol()).collect();
    
    assert!(content.contains("Agent: Hello"), "Should show agent message");
    assert!(content.contains("Though"), "Should show though");
}

/// Test: Render thinking indicator when agent starts but no response yet
#[test]
fn test_render_thinking_indicator() {
    set_test_mode();
    let backend = TestBackend::new(60, 20);
    let mut terminal = Terminal::new(backend).unwrap();
    
    // Agent started but no response yet
    let state = update(
        update(AppState::default(), Event::Submit),
        Event::AgentThinking { id: "req.0".to_string() }
    );
    
    // streaming=true, current_request_id=Some(req.0), but no thought yet
    assert!(state.streaming);
    
    terminal.draw(|f| view(f, &state)).unwrap();
    
    let buf = terminal.backend().buffer();
    let content: String = buf.content.iter().map(|c| c.symbol()).collect();
    
    assert!(content.contains("Thinking"), "Should show thinking indicator when agent starts");
}

/// Test: Thinking indicator hides after first response (thought created)
#[test]
fn test_thinking_indicator_hides_after_response() {
    set_test_mode();
    let backend = TestBackend::new(60, 20);
    let mut terminal = Terminal::new(backend).unwrap();
    
    let mut state = AppState::default();
    state.streaming = true;
    state.thinking_started_at = Some(std::time::Instant::now());
    
    // First response creates thought
    state = update(state, Event::AgentResponse { 
        id: "req.0".to_string(), 
        content: "Hello".to_string() 
    });
    
    terminal.draw(|f| view(f, &state)).unwrap();
    
    let buf = terminal.backend().buffer();
    let content: String = buf.content.iter().map(|c| c.symbol()).collect();
    
    // Should NOT show thinking indicator (thought exists)
    assert!(!content.contains("Thinking"), "Should not show thinking after thought created");
    // Should show the thought marker
    assert!(content.contains("Though"), "Should show though marker");
}

/// Test: Multiple requests - both show thought
#[test]
fn test_render_multiple_thoughts() {
    set_test_mode();
    let backend = TestBackend::new(60, 40);
    let mut terminal = Terminal::new(backend).unwrap();
    
    let mut state = AppState::default();
    
    // Request A done
    state.streaming = true;
    state.thinking_started_at = Some(std::time::Instant::now());
    state = update(state, Event::AgentThinking { id: "req.0".to_string() });
    state = update(state, Event::AgentResponse { id: "req.0".to_string(), content: "A".to_string() });
    state = update(state, Event::AgentDone { id: "req.0".to_string() });
    
    // Request B starts and gets first response (creates B's thought)
    state = update(state, Event::AgentThinking { id: "req.1".to_string() });
    state = update(state, Event::AgentResponse { id: "req.1".to_string(), content: "B".to_string() });
    
    terminal.draw(|f| view(f, &state)).unwrap();
    
    let buf = terminal.backend().buffer();
    let content: String = buf.content.iter().map(|c| c.symbol()).collect();
    
    // Count "Though" occurrences - should have A's and B's
    let though_count = content.matches("Though").count();
    assert_eq!(though_count, 2, "Should show 2 thoughs: one for A, one for B");
}

/// Test: Three sequential requests each get their own thought
#[test]
fn test_three_sequential_requests_three_thoughts() {
    set_test_mode();
    let backend = TestBackend::new(60, 50);
    let mut terminal = Terminal::new(backend).unwrap();
    
    let mut state = AppState::default();
    
    // === Request 0 ===
    state.streaming = true;
    state.thinking_started_at = Some(std::time::Instant::now());
    state = update(state, Event::AgentThinking { id: "req.0".to_string() });
    state = update(state, Event::AgentResponse { id: "req.0".to_string(), content: "R0".to_string() });
    state = update(state, Event::AgentDone { id: "req.0".to_string() });
    
    // === Request 1 ===
    state = update(state, Event::AgentThinking { id: "req.1".to_string() });
    state = update(state, Event::AgentResponse { id: "req.1".to_string(), content: "R1".to_string() });
    state = update(state, Event::AgentDone { id: "req.1".to_string() });
    
    // === Request 2 (in progress) ===
    state = update(state, Event::AgentThinking { id: "req.2".to_string() });
    state = update(state, Event::AgentResponse { id: "req.2".to_string(), content: "R2".to_string() });
    
    terminal.draw(|f| view(f, &state)).unwrap();
    
    let buf = terminal.backend().buffer();
    let content: String = buf.content.iter().map(|c| c.symbol()).collect();
    
    // Should have 3 thoughts: one for each request
    let though_count = content.matches("Though").count();
    assert_eq!(though_count, 3, "Should show 3 thoughs for 3 sequential requests");
    
    // Should have 3 responses
    let agent_count = content.matches("Agent:").count();
    assert_eq!(agent_count, 3, "Should show 3 agent responses");
}

/// Test: Though spinner shows before first response, Though marker after
#[test]
fn test_thinking_vs_thought_marker() {
    set_test_mode();
    
    // === Before first response: should show "Thinking" spinner ===
    {
        let state = update(
            update(AppState::default(), Event::Submit),
            Event::AgentThinking { id: "req.0".to_string() }
        );
        
        use runie_core::ui::format_messages;
        let lines = format_messages(&state);
        let content: String = lines.iter()
            .flat_map(|l| l.spans.iter().map(|s| s.text.clone()).collect::<Vec<_>>())
            .collect();
        
        assert!(content.contains("Thinking") && !content.contains("◆"),
            "Before response: should show 'Though', not '◆ Though' marker");
    }
    
    // === After first response: should show "◆ Though" marker ===
    {
        let mut state = AppState::default();
        state.streaming = true;
        state.thinking_started_at = Some(std::time::Instant::now());
        state = update(state, Event::AgentResponse { id: "req.0".to_string(), content: "Hi".to_string() });
        
        use runie_core::ui::format_messages;
        let lines = format_messages(&state);
        let content: String = lines.iter()
            .flat_map(|l| l.spans.iter().map(|s| s.text.clone()).collect::<Vec<_>>())
            .collect();
        
        assert!(content.contains("◆ Though"),
            "After response: should show '◆ Though' marker");
    }
}
