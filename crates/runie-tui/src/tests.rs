//! Rendering tests using TestBackend
use ratatui::{backend::TestBackend, Terminal};

use crate::ui::view;
use runie_core::{AppState, Event, update::update};

#[test]
fn renders_empty_state() {
    let backend = TestBackend::new(60, 20);
    let mut terminal = Terminal::new(backend).unwrap();
    
    let state = AppState::default();
    terminal.draw(|f| view(f, &state)).unwrap();
    
    let buf = terminal.backend().buffer();
    
    // Should have Chat panel
    let content: String = buf.content.iter()
        .map(|c| c.symbol())
        .collect();
    
    assert!(content.contains("Chat"), "Should contain Chat panel");
}

#[test]
fn renders_user_message() {
    let backend = TestBackend::new(60, 20);
    let mut terminal = Terminal::new(backend).unwrap();
    
    let state = AppState::default();
    let state = update(state, Event::Input('H'));
    let state = update(state, Event::Input('i'));
    let state = update(state, Event::Submit);
    
    terminal.draw(|f| view(f, &state)).unwrap();
    
    let buf = terminal.backend().buffer();
    let lines: Vec<String> = buf.content.chunks(60)
        .map(|row| row.iter().map(|c| c.symbol()).collect::<String>().trim_end().to_string())
        .collect();
    
    // Should show user message
    let has_user = lines.iter().any(|l| l.contains("You: Hi"));
    assert!(has_user, "Should show user message. Lines: {:?}", lines);
}

#[test]
fn renders_agent_response() {
    let backend = TestBackend::new(60, 20);
    let mut terminal = Terminal::new(backend).unwrap();
    
    let mut state = AppState::default();
    state.streaming = true;
    state.thinking_started_at = Some(std::time::Instant::now());
    let state = update(state, Event::AgentResponse {
        id: "req.0".to_string(),
        content: "Hello".to_string()
    });
    
    terminal.draw(|f| view(f, &state)).unwrap();
    
    let buf = terminal.backend().buffer();
    let lines: Vec<String> = buf.content.chunks(60)
        .map(|row| row.iter().map(|c| c.symbol()).collect::<String>().trim_end().to_string())
        .collect();
    
    // Should show agent response
    let has_agent = lines.iter().any(|l| l.contains("Agent: Hello"));
    let has_thought = lines.iter().any(|l| l.contains("◆ Though"));
    
    assert!(has_thought, "Should show thought marker. Lines: {:?}", lines);
    assert!(has_agent, "Should show agent message. Lines: {:?}", lines);
}

#[test]
fn renders_thinking_for_queued_request() {
    let backend = TestBackend::new(60, 20);
    let mut terminal = Terminal::new(backend).unwrap();
    
    // Simulate: first request done, second in queue, streaming active
    let mut state = AppState::default();
    state.streaming = true;
    state.request_queue.push(("Second question".to_string(), "req.1".to_string()));
    
    terminal.draw(|f| view(f, &state)).unwrap();
    
    let buf = terminal.backend().buffer();
    let lines: Vec<String> = buf.content.chunks(60)
        .map(|row| row.iter().map(|c| c.symbol()).collect::<String>().trim_end().to_string())
        .collect();
    
    // Should show thinking indicator
    let has_thinking = lines.iter().any(|l| l.contains("Thinking"));
    assert!(has_thinking, "Should show Thinking for queued request. Lines: {:?}", lines);
}
