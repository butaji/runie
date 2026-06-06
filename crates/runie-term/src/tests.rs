//! End-to-end tests for the terminal application

use runie_core::{AppState, Event};
use runie_tui::ui::view;
use ratatui::{backend::TestBackend, Terminal};

#[test]
fn test_submit_adds_message_to_queue() {
    let mut state = AppState::default();
    state.update(Event::Input('H'));
    state.update(Event::Input('i'));
    state.update(Event::Submit);

    assert_eq!(state.input, "");
    assert_eq!(state.messages.len(), 1);
    assert_eq!(state.messages[0].role, "user");
    assert_eq!(state.request_queue.len(), 1);
}

#[test]
fn test_agent_thinking_sets_streaming() {
    let mut state = AppState::default();
    state.update(Event::Submit);
    state.update(Event::AgentThinking { id: "req.0".to_string() });

    assert!(state.streaming);
    assert!(state.thinking_started_at.is_some());
}

#[test]
fn test_agent_response_creates_messages() {
    let mut state = AppState::default();
    state.streaming = true;

    state.update(Event::AgentThinking { id: "req.0".to_string() });
    state.update(Event::AgentThoughtDone { id: "req.0".to_string() });
    state.update(Event::AgentResponse { id: "req.0".to_string(), content: "Hello".to_string() });

    assert_eq!(state.messages.len(), 2);
    assert_eq!(state.messages[0].role, "thought");
    assert_eq!(state.messages[1].role, "assistant");
}

#[test]
fn test_agent_done_clears_streaming() {
    let mut state = AppState::default();
    state.streaming = true;

    state.update(Event::AgentThinking { id: "req.0".to_string() });
    state.update(Event::AgentThoughtDone { id: "req.0".to_string() });
    state.update(Event::AgentResponse { id: "req.0".to_string(), content: "Hi".to_string() });
    state.update(Event::AgentDone { id: "req.0".to_string() });

    assert!(!state.streaming);
}

#[test]
fn test_sequential_fifo_a_then_b() {
    let mut state = AppState::default();

    state.update(Event::Input('A'));
    state.update(Event::Submit);
    state.pop_queue();
    state.streaming = true;

    state.update(Event::AgentThinking { id: "req.0".to_string() });
    state.update(Event::AgentThoughtDone { id: "req.0".to_string() });
    state.update(Event::AgentResponse { id: "req.0".to_string(), content: "A".to_string() });
    state.update(Event::AgentDone { id: "req.0".to_string() });

    state.update(Event::AgentThinking { id: "req.1".to_string() });
    state.update(Event::AgentThoughtDone { id: "req.1".to_string() });
    state.update(Event::AgentResponse { id: "req.1".to_string(), content: "B".to_string() });

    let thoughts: Vec<_> = state.messages.iter().filter(|m| m.role == "thought").collect();
    assert_eq!(thoughts.len(), 2);
}

#[test]
fn test_render_user_message() {
    let backend = TestBackend::new(60, 20);
    let mut terminal = Terminal::new(backend).unwrap();

    let mut state = AppState::default();
    state.update(Event::Input('H'));
    state.update(Event::Submit);
    state.ensure_fresh();
    terminal.draw(|f| view(f, &state)).unwrap();

    let buf = terminal.backend().buffer();
    let content: String = buf.content.iter().map(|c| c.symbol()).collect();

    assert!(content.contains("You:"));
}

#[test]
fn test_render_agent_response() {
    let backend = TestBackend::new(60, 20);
    let mut terminal = Terminal::new(backend).unwrap();

    let mut state = AppState::default();
    state.streaming = true;

    state.update(Event::AgentThinking { id: "req.0".to_string() });
    state.update(Event::AgentThoughtDone { id: "req.0".to_string() });
    state.update(Event::AgentResponse { id: "req.0".to_string(), content: "Hello".to_string() });
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

    let start = Instant::now();
    for _ in 0..100 {
        terminal.draw(|f| view(f, &state)).unwrap();
    }
    let elapsed = start.elapsed();

    println!("100 renders with 1000 messages: {:?} ({} msg)", elapsed, state.messages.len());
    assert!(elapsed.as_secs_f64() < 1.0, "Rendering too slow: {:?}", elapsed);
}

#[test]
fn test_stress_many_tool_calls() {
    let backend = TestBackend::new(60, 20);
    let mut terminal = Terminal::new(backend).unwrap();

    let mut state = AppState::default();

    for i in 0..20 {
        let id = format!("req.{}", i);

        state.update(Event::Input('l'));
        state.update(Event::Submit);
        state.pop_queue();
        state.streaming = true;

        state.update(Event::AgentThinking { id: id.clone() });
        state.update(Event::AgentThoughtDone { id: id.clone() });
        state.update(Event::AgentToolStart { id: id.clone(), name: "list_files".to_string() });
        state.update(Event::AgentToolEnd { duration_secs: 0.5 });
        state.update(Event::AgentThinking { id: id.clone() });
        state.update(Event::AgentThoughtDone { id: id.clone() });
        state.update(Event::AgentResponse {
            id: id.clone(),
            content: format!("Files for turn {}\n", i)
        });
        state.update(Event::AgentTurnComplete { id: id.clone(), duration_secs: 1.0 });
        state.update(Event::AgentDone { id: id.clone() });

        if i % 5 == 0 {
            state.ensure_fresh();
            terminal.draw(|f| view(f, &state)).unwrap();
        }
    }

    state.ensure_fresh();
    terminal.draw(|f| view(f, &state)).unwrap();

    let buf = terminal.backend().buffer();
    let content: String = buf.content.iter().map(|c| c.symbol()).collect();

    assert!(content.contains("Files for turn"));
    assert!(state.messages.len() >= 100, "Expected many messages, got {}", state.messages.len());
}

#[test]
fn test_render_thinking_indicator() {
    let backend = TestBackend::new(60, 20);
    let mut terminal = Terminal::new(backend).unwrap();

    let mut state = AppState::default();
    state.update(Event::Submit);
    state.update(Event::AgentThinking { id: "req.0".to_string() });
    state.ensure_fresh();
    terminal.draw(|f| view(f, &state)).unwrap();

    let buf = terminal.backend().buffer();
    let content: String = buf.content.iter().map(|c| c.symbol()).collect();

    assert!(content.contains("Though"));
}
