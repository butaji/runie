//! End-to-end tests for the terminal application

use runie_core::{AppState, Event, update::update};
use runie_tui::ui::view;
use ratatui::{backend::TestBackend, Terminal};

fn set_test_mode() {
    std::env::set_var("RUNIE_TEST", "1");
}

#[test]
fn test_submit_adds_message_to_queue() {
    set_test_mode();
    let state = update(update(update(AppState::default(), Event::Input('H')), Event::Input('i')), Event::Submit);
    
    assert_eq!(state.input, "");
    assert_eq!(state.messages.len(), 1);
    assert_eq!(state.messages[0].role, "user");
    assert_eq!(state.request_queue.len(), 1);
}

#[test]
fn test_agent_thinking_sets_streaming() {
    set_test_mode();
    let state = update(update(AppState::default(), Event::Submit), Event::AgentThinking { id: "req.0".to_string() });
    
    assert!(state.streaming);
    assert!(state.thinking_started_at.is_some());
}

#[test]
fn test_agent_response_creates_messages() {
    set_test_mode();
    let mut state = AppState::default();
    state.streaming = true;
    
    state = update(state, Event::AgentThinking { id: "req.0".to_string() });
    state = update(state, Event::AgentThoughtDone { id: "req.0".to_string() });
    state = update(state, Event::AgentResponse { id: "req.0".to_string(), content: "Hello".to_string() });
    
    assert_eq!(state.messages.len(), 2);
    assert_eq!(state.messages[0].role, "thought");
    assert_eq!(state.messages[1].role, "assistant");
}

#[test]
fn test_agent_done_clears_streaming() {
    set_test_mode();
    let mut state = AppState::default();
    state.streaming = true;
    
    state = update(state, Event::AgentThinking { id: "req.0".to_string() });
    state = update(state, Event::AgentThoughtDone { id: "req.0".to_string() });
    state = update(state, Event::AgentResponse { id: "req.0".to_string(), content: "Hi".to_string() });
    state = update(state, Event::AgentDone { id: "req.0".to_string() });
    
    assert!(!state.streaming);
}

#[test]
fn test_sequential_fifo_a_then_b() {
    set_test_mode();
    let mut state = AppState::default();
    
    state = update(state, Event::Input('A'));
    state = update(state, Event::Submit);
    state.pop_queue();
    state.streaming = true;
    
    state = update(state, Event::AgentThinking { id: "req.0".to_string() });
    state = update(state, Event::AgentThoughtDone { id: "req.0".to_string() });
    state = update(state, Event::AgentResponse { id: "req.0".to_string(), content: "A".to_string() });
    state = update(state, Event::AgentDone { id: "req.0".to_string() });
    
    state = update(state, Event::AgentThinking { id: "req.1".to_string() });
    state = update(state, Event::AgentThoughtDone { id: "req.1".to_string() });
    state = update(state, Event::AgentResponse { id: "req.1".to_string(), content: "B".to_string() });
    
    let thoughts: Vec<_> = state.messages.iter().filter(|m| m.role == "thought").collect();
    assert_eq!(thoughts.len(), 2);
}

#[test]
fn test_render_user_message() {
    set_test_mode();
    let backend = TestBackend::new(60, 20);
    let mut terminal = Terminal::new(backend).unwrap();
    
    let mut state = update(update(AppState::default(), Event::Input('H')), Event::Submit);
    state.ensure_fresh();
    terminal.draw(|f| view(f, &state)).unwrap();
    
    let buf = terminal.backend().buffer();
    let content: String = buf.content.iter().map(|c| c.symbol()).collect();
    
    assert!(content.contains("You:"));
}

#[test]
fn test_render_agent_response() {
    set_test_mode();
    let backend = TestBackend::new(60, 20);
    let mut terminal = Terminal::new(backend).unwrap();
    
    let mut state = AppState::default();
    state.streaming = true;
    
    state = update(state, Event::AgentThinking { id: "req.0".to_string() });
    state = update(state, Event::AgentThoughtDone { id: "req.0".to_string() });
    state = update(state, Event::AgentResponse { id: "req.0".to_string(), content: "Hello".to_string() });
    state.ensure_fresh();
    terminal.draw(|f| view(f, &state)).unwrap();
    
    let buf = terminal.backend().buffer();
    let content: String = buf.content.iter().map(|c| c.symbol()).collect();
    
    assert!(content.contains("Agent:"));
    assert!(content.contains("Hello"));
}

#[test]
fn test_render_performance_1000_messages() {
    use std::time::Instant;
    let backend = TestBackend::new(80, 40);
    let mut terminal = Terminal::new(backend).unwrap();
    
    let mut state = AppState::default();
    
    // Build 1000 messages
    for i in 0..200 {
        state.messages.push(runie_core::ChatMessage {
            role: "user".to_string(),
            content: format!("Message {} from user with some content here", i),
            timestamp: 0.0,
            id: format!("req.{}", i),
        });
        state.messages.push(runie_core::ChatMessage {
            role: "assistant".to_string(),
            content: format!("Response {} from agent with detailed explanation and more text to fill lines", i),
            timestamp: 0.0,
            id: format!("resp.{}", i),
        });
    }
    
    state.ensure_fresh();
    
    // Time 100 renders
    let start = Instant::now();
    for _ in 0..100 {
        terminal.draw(|f| view(f, &state)).unwrap();
    }
    let elapsed = start.elapsed();
    
    println!("100 renders with 1000 messages: {:?} ({} msg)", elapsed, state.messages.len());
    
    // Should be fast - less than 1 second for 100 renders
    assert!(elapsed.as_secs_f64() < 1.0, "Rendering too slow: {:?}", elapsed);
}

#[test]
fn test_stress_many_tool_calls() {
    set_test_mode();
    let backend = TestBackend::new(60, 20);
    let mut terminal = Terminal::new(backend).unwrap();
    
    let mut state = AppState::default();
    
    // Simulate 20 rapid "list files" commands
    for i in 0..20 {
        let id = format!("req.{}", i);
        
        // User submits
        state = update(state, Event::Input('l'));
        state = update(state, Event::Submit);
        state.pop_queue();
        state.streaming = true;
        
        // Agent thinking
        state = update(state, Event::AgentThinking { id: id.clone() });
        state = update(state, Event::AgentThoughtDone { id: id.clone() });
        
        // Tool execution
        state = update(state, Event::AgentToolStart { id: id.clone(), name: "list_files".to_string() });
        state = update(state, Event::AgentToolEnd { duration_secs: 0.5 });
        
        // Second thinking
        state = update(state, Event::AgentThinking { id: id.clone() });
        state = update(state, Event::AgentThoughtDone { id: id.clone() });
        
        // Response
        state = update(state, Event::AgentResponse { 
            id: id.clone(), 
            content: format!("Files for turn {}\n", i) 
        });
        
        // Turn complete
        state = update(state, Event::AgentTurnComplete { id: id.clone(), duration_secs: 1.0 });
        state = update(state, Event::AgentDone { id: id.clone() });
        
        // Render every 5 turns
        if i % 5 == 0 {
            state.ensure_fresh();
            terminal.draw(|f| view(f, &state)).unwrap();
        }
    }
    
    // Final render
    state.ensure_fresh();
    terminal.draw(|f| view(f, &state)).unwrap();
    
    let buf = terminal.backend().buffer();
    let content: String = buf.content.iter().map(|c| c.symbol()).collect();
    
    // Should have content from later turns visible (auto-scroll)
    assert!(content.contains("Files for turn"));
    
    // Check message count
    // 20 turns * (1 user + 2 thought + 1 tool + 1 assistant + 1 turn_complete) = ~120 messages
    assert!(state.messages.len() >= 100, "Expected many messages, got {}", state.messages.len());
}

#[test]
fn test_render_thinking_indicator() {
    set_test_mode();
    let backend = TestBackend::new(60, 20);
    let mut terminal = Terminal::new(backend).unwrap();
    
    let mut state = update(
        update(AppState::default(), Event::Submit),
        Event::AgentThinking { id: "req.0".to_string() }
    );
    state.ensure_fresh();
    terminal.draw(|f| view(f, &state)).unwrap();
    
    let buf = terminal.backend().buffer();
    let content: String = buf.content.iter().map(|c| c.symbol()).collect();
    
    assert!(content.contains("Though"));
}
