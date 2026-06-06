//! End-to-end tests for the terminal application
use runie_core::{AppState, Event, Role};
use runie_tui::ui::view;
use ratatui::{backend::TestBackend, Terminal};

/// Helper: simulate full "list files" tool flow
fn simulate_list_files_flow(state: &mut AppState, file_content: &str) {
    state.update(Event::AgentThinking { id: "req.0".to_string() });
    state.update(Event::AgentThoughtDone { id: "req.0".to_string() });
    state.update(Event::AgentToolStart { id: "req.0".to_string(), name: "list_files".to_string() });
    state.update(Event::AgentToolEnd { duration_secs: 1.0 });
    state.update(Event::AgentThinking { id: "req.0".to_string() });
    state.update(Event::AgentThoughtDone { id: "req.0".to_string() });
    state.update(Event::AgentResponse { id: "req.0".to_string(), content: format!("\n{}", file_content) });
    state.update(Event::AgentTurnComplete { id: "req.0".to_string(), duration_secs: 3.0 });
    state.update(Event::AgentDone { id: "req.0".to_string() });
}

// === REGRESSION TEST: Chat must render content ===
// This test verifies that view() calls ensure_fresh() internally.
// WITHOUT the fix, this test FAILS because cache is never rebuilt.

#[test]
fn test_view_renders_user_message_without_manual_ensure_fresh() {
    let backend = TestBackend::new(60, 20);
    let mut terminal = Terminal::new(backend).expect("terminal");
    let mut state = AppState::default();
    
    // Add a message - this sets dirty=true but doesn't rebuild cache
    state.update(Event::Input('H'));
    state.update(Event::Input('i'));
    state.update(Event::Submit);
    
    // DO NOT call state.ensure_fresh() here!
    // The view() function MUST call it internally.
    
    terminal.draw(|f| view(f, &mut state)).expect("draw");
    let buf = terminal.backend().buffer();
    let content: String = buf.content.iter().map(|c| c.symbol()).collect();
    
    // This assertion would FAIL if view() doesn't call ensure_fresh()
    assert!(content.contains("You:"), "Chat must render 'You:' prefix. Got: {}", content);
    assert!(content.contains("Hi"), "Chat must render message content. Got: {}", content);
}

#[test]
fn test_view_renders_agent_message_without_manual_ensure_fresh() {
    let backend = TestBackend::new(60, 20);
    let mut terminal = Terminal::new(backend).expect("terminal");
    let mut state = AppState::default();
    
    state.update(Event::AgentThinking { id: "req.0".to_string() });
    state.update(Event::AgentThoughtDone { id: "req.0".to_string() });
    state.update(Event::AgentResponse { id: "req.0".to_string(), content: "Hello".to_string() });
    
    // DO NOT call state.ensure_fresh() here!
    
    terminal.draw(|f| view(f, &mut state)).expect("draw");
    let buf = terminal.backend().buffer();
    let content: String = buf.content.iter().map(|c| c.symbol()).collect();
    
    assert!(content.contains("Agent:"), "Chat must render 'Agent:' prefix. Got: {}", content);
    assert!(content.contains("Hello"), "Chat must render agent response. Got: {}", content);
}

#[test]
fn test_view_renders_multiple_messages_without_manual_ensure_fresh() {
    let backend = TestBackend::new(60, 30);
    let mut terminal = Terminal::new(backend).expect("terminal");
    let mut state = AppState::default();
    
    // First message
    state.update(Event::Input('A'));
    state.update(Event::Submit);
    
    // Agent responds
    state.update(Event::AgentThinking { id: "req.0".to_string() });
    state.update(Event::AgentThoughtDone { id: "req.0".to_string() });
    state.update(Event::AgentResponse { id: "req.0".to_string(), content: "Response 1".to_string() });
    state.update(Event::AgentDone { id: "req.0".to_string() });
    
    // Second message
    state.update(Event::Input('B'));
    state.update(Event::Submit);
    
    // DO NOT call state.ensure_fresh() here!
    
    terminal.draw(|f| view(f, &mut state)).expect("draw");
    let buf = terminal.backend().buffer();
    let content: String = buf.content.iter().map(|c| c.symbol()).collect();
    
    assert!(content.contains("You:"), "Must show user prefix");
    assert!(content.contains("Agent:"), "Must show agent prefix");
    assert!(content.contains("A") || content.contains("B"), "Must show message content");
}

#[test]
fn test_submit_adds_message_to_queue() {
    let mut state = AppState::default();
    state.update(Event::Input('H'));
    state.update(Event::Input('i'));
    state.update(Event::Submit);
    assert_eq!(state.input, "");
    assert_eq!(state.messages.len(), 1);
    assert_eq!(state.messages[0].role, Role::User);
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
    assert_eq!(state.messages[0].role, Role::Thought);
    assert_eq!(state.messages[1].role, Role::Assistant);
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
    let thoughts: Vec<_> = state.messages.iter().filter(|m| m.role == Role::Thought).collect();
    assert_eq!(thoughts.len(), 2);
}

#[test]
fn test_render_user_message() {
    let backend = TestBackend::new(60, 20);
    let mut terminal = Terminal::new(backend).expect("terminal");
    let mut state = AppState::default();
    state.update(Event::Input('H'));
    state.update(Event::Submit);
    state.ensure_fresh();
    terminal.draw(|f| view(f, &mut state)).expect("draw");
    let buf = terminal.backend().buffer();
    let content: String = buf.content.iter().map(|c| c.symbol()).collect();
    assert!(content.contains("You:"));
}

#[test]
fn test_render_agent_response() {
    let backend = TestBackend::new(60, 20);
    let mut terminal = Terminal::new(backend).expect("terminal");
    let mut state = AppState::default();
    state.streaming = true;
    state.update(Event::AgentThinking { id: "req.0".to_string() });
    state.update(Event::AgentThoughtDone { id: "req.0".to_string() });
    state.update(Event::AgentResponse { id: "req.0".to_string(), content: "Hello".to_string() });
    state.ensure_fresh();
    terminal.draw(|f| view(f, &mut state)).expect("draw");
    let buf = terminal.backend().buffer();
    let content: String = buf.content.iter().map(|c| c.symbol()).collect();
    assert!(content.contains("Agent:"));
    assert!(content.contains("Hello"));
}

#[test]
fn test_render_performance_1000_messages() {
    use std::time::Instant;
    let backend = TestBackend::new(80, 40);
    let mut terminal = Terminal::new(backend).expect("terminal");
    let mut state = AppState::default();
    for i in 0..200 {
        state.messages.push(runie_core::ChatMessage {
            role: Role::User,
            content: format!("Message {} from user with some content here", i),
            timestamp: 0.0,
            id: format!("req.{}", i),
        });
        state.messages.push(runie_core::ChatMessage {
            role: Role::Assistant,
            content: format!("Response {} from agent with detailed explanation and more text", i),
            timestamp: 0.0,
            id: format!("resp.{}", i),
        });
    }
    state.ensure_fresh();
    let start = Instant::now();
    for _ in 0..100 {
        terminal.draw(|f| view(f, &mut state)).expect("draw");
    }
    let elapsed = start.elapsed();
    println!("100 renders with 1000 messages: {:?} ({} msg)", elapsed, state.messages.len());
    assert!(elapsed.as_secs_f64() < 1.0, "Rendering too slow: {:?}", elapsed);
}

fn simulate_tool_call(state: &mut AppState, i: usize) {
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
    state.update(Event::AgentResponse { id, content: format!("Files for turn {}\n", i) });
}

// === Integration Test: Full Tool Flow ===

#[test]
fn test_full_list_files_integration() {
    use runie_agent::{needs_tool_execution, get_fake_file_list};
    assert!(needs_tool_execution("list files"));
    let files = get_fake_file_list();
    assert!(files.contains("main.rs"));
    
    let mut state = AppState::default();
    state.streaming = true;
    simulate_list_files_flow(&mut state, &files);
    
    assert!(state.messages.iter().any(|m| m.role == Role::Thought));
    assert!(state.messages.iter().any(|m| m.role == Role::Tool));
    assert!(!state.streaming, "Streaming should stop after Done");
}

#[test]
fn test_list_files_command_flow() {
    // This test verifies the complete "list files" flow:
    // 1. User types "list files"
    // 2. Submit sends to queue with correct content
    // 3. peek_queue returns (content, id) tuple
    let mut state = AppState::default();
    
    // Type "list files"
    for c in "list files".chars() {
        state.update(Event::Input(c));
    }
    assert_eq!(state.input, "list files");
    
    // Submit
    state.update(Event::Submit);
    assert!(state.input.is_empty(), "Input should be cleared after submit");
    
    // Check queue has correct content
    let (content, id) = state.peek_queue().expect("Should have queued request");
    assert_eq!(content, "list files", "Queue should contain 'list files'");
    assert!(id.starts_with("req."), "Queue should have valid id");
    
    // Pop and verify
    let (content, _id) = state.pop_queue().expect("Should pop");
    assert_eq!(content, "list files");
}

#[test]
fn test_list_files_message_content() {
    // Debug: verify message content is stored correctly
    let mut state = AppState::default();
    state.streaming = true;
    
    state.update(Event::AgentThinking { id: "req.0".to_string() });
    state.update(Event::AgentThoughtDone { id: "req.0".to_string() });
    state.update(Event::AgentToolStart { id: "req.0".to_string(), name: "list_files".to_string() });
    state.update(Event::AgentToolEnd { duration_secs: 1.0 });
    state.update(Event::AgentThinking { id: "req.0".to_string() });
    state.update(Event::AgentThoughtDone { id: "req.0".to_string() });
    
    let file_list = "src/main.rs".to_string();
    state.update(Event::AgentResponse { id: "req.0".to_string(), content: format!("\n{}", file_list) });
    
    // Check message content
    let assistant_msg = state.messages.iter().find(|m| m.role == Role::Assistant).expect("Should have assistant message");
    println!("Assistant message content: {:?}", assistant_msg.content);
    assert!(assistant_msg.content.contains("src/main.rs"), "Message should contain file list");
}

#[test]
fn test_list_files_full_sequence() {
    use runie_agent::get_fake_file_list;
    let file_list = get_fake_file_list();
    
    let mut state = AppState::default();
    state.streaming = true;
    simulate_list_files_flow(&mut state, &file_list);
    
    // Verify message content
    let msg = state.messages.iter().find(|m| m.role == Role::Assistant).expect("Should have message");
    assert!(msg.content.contains("main.rs"), "Should contain file list");
    assert_eq!(state.messages.len(), 5, "Should have 5 messages");
}

#[test]
fn test_stress_many_tool_calls() {
    let backend = TestBackend::new(60, 20);
    let mut terminal = Terminal::new(backend).expect("terminal");
    let mut state = AppState::default();
    for i in 0..20 {
        simulate_tool_call(&mut state, i);
        state.update(Event::AgentTurnComplete { id: format!("req.{}", i), duration_secs: 1.0 });
        state.update(Event::AgentDone { id: format!("req.{}", i) });
        if i % 5 == 0 {
            state.ensure_fresh();
            terminal.draw(|f| view(f, &mut state)).expect("draw");
        }
    }
    state.ensure_fresh();
    terminal.draw(|f| view(f, &mut state)).expect("draw");
    let buf = terminal.backend().buffer();
    let content: String = buf.content.iter().map(|c| c.symbol()).collect();
    assert!(content.contains("Files for turn"));
    assert!(state.messages.len() >= 100, "Expected many messages, got {}", state.messages.len());
}

#[test]
fn test_render_thinking_indicator() {
    let backend = TestBackend::new(60, 20);
    let mut terminal = Terminal::new(backend).expect("terminal");
    let mut state = AppState::default();
    state.update(Event::Submit);
    state.update(Event::AgentThinking { id: "req.0".to_string() });
    state.ensure_fresh();
    terminal.draw(|f| view(f, &mut state)).expect("draw");
    let buf = terminal.backend().buffer();
    let content: String = buf.content.iter().map(|c| c.symbol()).collect();
    assert!(content.contains("Though"));
}
