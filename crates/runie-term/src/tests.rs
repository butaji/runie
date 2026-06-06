//! End-to-end tests for the terminal application
use runie_core::{AppState, Event, Role, ChatMessage};
use runie_tui::ui::view;
use ratatui::{backend::TestBackend, Terminal};

/// Helper: simulate full tool flow
fn simulate_list_files_flow(state: &mut AppState) {
    state.update(Event::AgentThinking { id: "req.0".to_string() });
    state.update(Event::AgentThoughtDone { id: "req.0".to_string() });
    state.update(Event::AgentToolStart { id: "req.0".to_string(), name: "list_files".to_string() });
    state.update(Event::AgentToolEnd { duration_secs: 1.0 });
    state.update(Event::AgentThinking { id: "req.0".to_string() });
    state.update(Event::AgentThoughtDone { id: "req.0".to_string() });
    state.update(Event::AgentResponse { id: "req.0".to_string(), content: "src/main.rs\nlib.rs".to_string() });
    state.update(Event::AgentTurnComplete { id: "req.0".to_string(), duration_secs: 3.0 });
    state.update(Event::AgentDone { id: "req.0".to_string() });
}

// === REGRESSION: view() must call ensure_fresh() internally ===

#[test]
fn test_view_renders_user_message_without_manual_ensure_fresh() {
    let backend = TestBackend::new(60, 20);
    let mut terminal = Terminal::new(backend).expect("terminal");
    let mut state = AppState::default();

    state.update(Event::Input('H'));
    state.update(Event::Input('i'));
    state.update(Event::Submit);

    // DO NOT call state.ensure_fresh() here!
    terminal.draw(|f| view(f, &mut state)).expect("draw");

    let buf = terminal.backend().buffer();
    let content: String = buf.content.iter().map(|c| c.symbol()).collect();
    assert!(content.contains("You:"), "Chat must render 'You:'. Got: {}", content);
    assert!(content.contains("Hi"), "Chat must render content. Got: {}", content);
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
    assert!(content.contains("Agent:"), "Must render 'Agent:'. Got: {}", content);
    assert!(content.contains("Hello"), "Must render response. Got: {}", content);
}

#[test]
fn test_view_renders_multiple_messages_without_manual_ensure_fresh() {
    let backend = TestBackend::new(60, 30);
    let mut terminal = Terminal::new(backend).expect("terminal");
    let mut state = AppState::default();

    state.update(Event::Input('A'));
    state.update(Event::Submit);
    state.update(Event::AgentThinking { id: "req.0".to_string() });
    state.update(Event::AgentThoughtDone { id: "req.0".to_string() });
    state.update(Event::AgentResponse { id: "req.0".to_string(), content: "Response 1".to_string() });
    state.update(Event::AgentDone { id: "req.0".to_string() });
    state.update(Event::Input('B'));
    state.update(Event::Submit);

    // DO NOT call state.ensure_fresh() here!
    terminal.draw(|f| view(f, &mut state)).expect("draw");

    let buf = terminal.backend().buffer();
    let content: String = buf.content.iter().map(|c| c.symbol()).collect();
    assert!(content.contains("You:"), "Must show user prefix");
    assert!(content.contains("Agent:"), "Must show agent prefix");
}

// === Basic Flow ===

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

// === Render Tests ===

#[test]
fn test_render_user_message() {
    let backend = TestBackend::new(60, 20);
    let mut terminal = Terminal::new(backend).unwrap();
    let mut state = AppState::default();
    state.update(Event::Input('H'));
    state.update(Event::Submit);
    terminal.draw(|f| view(f, &mut state)).unwrap();
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
    terminal.draw(|f| view(f, &mut state)).unwrap();
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
        state.messages.push(ChatMessage {
            role: Role::User,
            content: format!("Message {} from user with some content here", i),
            timestamp: 0.0,
            id: format!("req.{}", i),
        });
        state.messages.push(ChatMessage {
            role: Role::Assistant,
            content: format!("Response {} from agent with detailed explanation", i),
            timestamp: 0.0,
            id: format!("resp.{}", i),
        });
    }
    let start = Instant::now();
    for _ in 0..100 {
        terminal.draw(|f| view(f, &mut state)).unwrap();
    }
    let elapsed = start.elapsed();
    println!("100 renders with {} messages: {:?}", state.messages.len(), elapsed);
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

// === Integration: Full Tool Flow ===

#[test]
fn test_full_list_files_integration() {
    let mut state = AppState::default();
    state.streaming = true;
    simulate_list_files_flow(&mut state);

    assert!(state.messages.iter().any(|m| m.role == Role::Thought));
    assert!(state.messages.iter().any(|m| m.role == Role::Tool));
    assert!(!state.streaming, "Streaming should stop after Done");
}

#[test]
fn test_list_files_command_flow() {
    let mut state = AppState::default();

    for c in "list files".chars() {
        state.update(Event::Input(c));
    }
    assert_eq!(state.input, "list files");

    state.update(Event::Submit);
    assert!(state.input.is_empty(), "Input cleared after submit");

    let (content, id) = state.peek_queue().expect("queued request");
    assert_eq!(content, "list files");
    assert!(id.starts_with("req."), "valid id");

    let (content, _id) = state.pop_queue().expect("pop");
    assert_eq!(content, "list files");
}

#[test]
fn test_list_files_message_content() {
    let mut state = AppState::default();
    state.streaming = true;
    state.update(Event::AgentThinking { id: "req.0".to_string() });
    state.update(Event::AgentThoughtDone { id: "req.0".to_string() });
    state.update(Event::AgentToolStart { id: "req.0".to_string(), name: "list_files".to_string() });
    state.update(Event::AgentToolEnd { duration_secs: 1.0 });
    state.update(Event::AgentThinking { id: "req.0".to_string() });
    state.update(Event::AgentThoughtDone { id: "req.0".to_string() });
    state.update(Event::AgentResponse { id: "req.0".to_string(), content: "\nsrc/main.rs".to_string() });

    let assistant = state.messages.iter().find(|m| m.role == Role::Assistant).expect("assistant msg");
    assert!(assistant.content.contains("src/main.rs"), "Should contain file list");
}

#[test]
fn test_list_files_full_sequence() {
    let mut state = AppState::default();
    state.streaming = true;
    simulate_list_files_flow(&mut state);

    let msg = state.messages.iter().find(|m| m.role == Role::Assistant).expect("assistant msg");
    assert!(msg.content.contains("main.rs"));
    assert_eq!(state.messages.len(), 5, "expected 5 messages");
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
            terminal.draw(|f| view(f, &mut state)).expect("draw");
        }
    }
    terminal.draw(|f| view(f, &mut state)).expect("draw");
    let buf = terminal.backend().buffer();
    let content: String = buf.content.iter().map(|c| c.symbol()).collect();
    assert!(content.contains("Files for turn"));
    assert!(state.messages.len() >= 100, "many messages, got {}", state.messages.len());
}

#[test]
fn test_render_thinking_indicator() {
    let backend = TestBackend::new(60, 20);
    let mut terminal = Terminal::new(backend).expect("terminal");
    let mut state = AppState::default();
    state.update(Event::Submit);
    state.update(Event::AgentThinking { id: "req.0".to_string() });
    terminal.draw(|f| view(f, &mut state)).expect("draw");
    let buf = terminal.backend().buffer();
    let content: String = buf.content.iter().map(|c| c.symbol()).collect();
    assert!(content.contains("Though"));
}

// === Slash command rendering ===

#[test]
fn test_render_sessions_list_on_separate_lines() {
    use runie_core::session::Store;
    use std::sync::Mutex;
    static LOCK: Mutex<()> = Mutex::new(());
    let _guard = LOCK.lock().unwrap_or_else(|e| e.into_inner());

    let store = Store::new(std::env::temp_dir().join("runie_render_sessions_test"));
    let _ = std::fs::remove_dir_all(&store.dir);
    std::env::set_var("RUNIE_SESSIONS_DIR", store.dir.clone());

    store.save("alpha", &runie_core::Session {
        name: "alpha".to_string(), created_at: 1.0, updated_at: 1.0,
        messages: vec![], provider: "mock".into(), model: "echo".into(),
    }).unwrap();
    store.save("beta", &runie_core::Session {
        name: "beta".to_string(), created_at: 1.0, updated_at: 1.0,
        messages: vec![], provider: "mock".into(), model: "echo".into(),
    }).unwrap();

    let backend = TestBackend::new(60, 20);
    let mut terminal = Terminal::new(backend).expect("terminal");
    let mut state = AppState::default();

    for c in "/sessions".chars() {
        state.update(Event::Input(c));
    }
    state.update(Event::Submit);
    terminal.draw(|f| view(f, &mut state)).expect("draw");

    let buf = terminal.backend().buffer();
    let lines: Vec<String> = (0..buf.area().height)
        .map(|y| (0..buf.area().width).map(|x| buf.get(x, y).symbol()).collect::<String>())
        .collect();

    let session_line_count = lines.iter().filter(|l| l.contains("alpha") || l.contains("beta")).count();
    assert_eq!(session_line_count, 2, "Sessions should render on 2 separate lines, got: {:?}", lines);

    std::env::remove_var("RUNIE_SESSIONS_DIR");
}

#[test]
fn test_render_model_no_args_shows_usage_not_echoed() {
    let backend = TestBackend::new(60, 20);
    let mut terminal = Terminal::new(backend).expect("terminal");
    let mut state = AppState::default();

    for c in "/model".chars() {
        state.update(Event::Input(c));
    }
    state.update(Event::Submit);
    terminal.draw(|f| view(f, &mut state)).expect("draw");

    let buf = terminal.backend().buffer();
    let content: String = buf.content.iter().map(|c| c.symbol()).collect();
    assert!(content.contains("Usage:"), "Should show usage: {}", content);
    assert!(!content.contains("You: /model"), "Should NOT echo /model as user message: {}", content);
}

#[test]
fn test_render_save_no_args_shows_usage() {
    let backend = TestBackend::new(60, 20);
    let mut terminal = Terminal::new(backend).expect("terminal");
    let mut state = AppState::default();

    for c in "/save".chars() {
        state.update(Event::Input(c));
    }
    state.update(Event::Submit);
    terminal.draw(|f| view(f, &mut state)).expect("draw");

    let buf = terminal.backend().buffer();
    let content: String = buf.content.iter().map(|c| c.symbol()).collect();
    assert!(content.contains("Usage:"), "Should show usage: {}", content);
    assert!(!content.contains("You: /save"), "Should NOT echo /save as user message: {}", content);
}

#[test]
fn test_render_load_no_args_shows_usage() {
    let backend = TestBackend::new(60, 20);
    let mut terminal = Terminal::new(backend).expect("terminal");
    let mut state = AppState::default();

    for c in "/load".chars() {
        state.update(Event::Input(c));
    }
    state.update(Event::Submit);
    terminal.draw(|f| view(f, &mut state)).expect("draw");

    let buf = terminal.backend().buffer();
    let content: String = buf.content.iter().map(|c| c.symbol()).collect();
    assert!(content.contains("Usage:"), "Should show usage: {}", content);
    assert!(!content.contains("You: /load"), "Should NOT echo /load as user message: {}", content);
}

#[test]
fn test_render_delete_no_args_shows_usage() {
    let backend = TestBackend::new(60, 20);
    let mut terminal = Terminal::new(backend).expect("terminal");
    let mut state = AppState::default();

    for c in "/delete".chars() {
        state.update(Event::Input(c));
    }
    state.update(Event::Submit);
    terminal.draw(|f| view(f, &mut state)).expect("draw");

    let buf = terminal.backend().buffer();
    let content: String = buf.content.iter().map(|c| c.symbol()).collect();
    assert!(content.contains("Usage:"), "Should show usage: {}", content);
    assert!(!content.contains("You: /delete"), "Should NOT echo /delete as user message: {}", content);
}
