use super::*;
use std::sync::Mutex;
use std::time::Instant;

static ENV_LOCK: Mutex<()> = Mutex::new(());



#[test]
fn test_view_renders_user_message_without_manual_ensure_fresh() {
    let backend = TestBackend::new(60, 20);
    let mut terminal = Terminal::new(backend).expect("terminal");
    let mut state = AppState::default();

    state.update(Event::Input('H'));
    state.update(Event::Input('i'));
    state.update(Event::Submit);


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


    terminal.draw(|f| view(f, &mut state)).expect("draw");

    let buf = terminal.backend().buffer();
    let content: String = buf.content.iter().map(|c| c.symbol()).collect();
    assert!(content.contains("You:"), "Must show user prefix");
    assert!(content.contains("Agent:"), "Must show agent prefix");
}



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
    assert!(content.contains("Thinking"));
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



fn render_slash(input: &str) -> String {
    let backend = TestBackend::new(60, 20);
    let mut terminal = Terminal::new(backend).expect("terminal");
    let mut state = AppState::default();
    for c in input.chars() {
        state.update(Event::Input(c));
    }
    state.update(Event::Submit);
    terminal.draw(|f| view(f, &mut state)).expect("draw");
    let buf = terminal.backend().buffer();
    buf.content.iter().map(|c| c.symbol()).collect()
}

#[test]
fn test_render_sessions_list_on_separate_lines() {
    use runie_core::session::Store;
    let _guard = ENV_LOCK.lock().unwrap_or_else(|e| e.into_inner());

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
    let content = render_slash("/model");
    assert!(content.contains("Current model:"), "Should show current model: {}", content);
    assert!(content.contains("Usage:"), "Should show usage: {}", content);
    assert!(!content.contains("You: /model"), "Should NOT echo /model as user message: {}", content);
}

#[test]
fn test_render_save_no_args_shows_usage() {
    let content = render_slash("/save");
    assert!(content.contains("Usage:"), "Should show usage: {}", content);
    assert!(!content.contains("You: /save"), "Should NOT echo /save as user message: {}", content);
}

#[test]
fn test_render_load_no_args_shows_usage() {
    let content = render_slash("/load");
    assert!(content.contains("Usage:"), "Should show usage: {}", content);
    assert!(!content.contains("You: /load"), "Should NOT echo /load as user message: {}", content);
}

#[test]
fn test_render_delete_no_args_shows_usage() {
    let content = render_slash("/delete");
    assert!(content.contains("Usage:"), "Should show usage: {}", content);
    assert!(!content.contains("You: /delete"), "Should NOT echo /delete as user message: {}", content);
}

#[test]
fn test_render_model_m3_just_model_name() {
    let backend = TestBackend::new(60, 20);
    let mut terminal = Terminal::new(backend).expect("terminal");
    let mut state = AppState::default();

    for c in "/model m3".chars() {
        state.update(Event::Input(c));
    }
    state.update(Event::Submit);
    terminal.draw(|f| view(f, &mut state)).expect("draw");
    let buf = terminal.backend().buffer();
    let content: String = buf.content.iter().map(|c| c.symbol()).collect();
    assert!(content.contains("Switched to mock/m3"), "/model m3 should render: {}", content);
    assert!(!content.contains("You: /model m3"), "Should NOT echo /model m3 as user message: {}", content);
    assert_eq!(state.current_model, "m3");
}

#[test]
fn test_render_load_missing_shows_user_friendly_error() {
    use runie_core::session::Store;
    let _guard = ENV_LOCK.lock().unwrap_or_else(|e| e.into_inner());

    let store = Store::new(std::env::temp_dir().join("runie_render_load_missing_test"));
    let _ = std::fs::remove_dir_all(&store.dir);
    std::env::set_var("RUNIE_SESSIONS_DIR", store.dir.clone());

    let content = render_slash("/load missing");
    assert!(content.contains("not found"), "Should show not found: {}", content);
    assert!(content.contains("/sessions"), "Should suggest /sessions: {}", content);
    assert!(!content.contains("You: /load missing"), "Should NOT echo as user message: {}", content);

    std::env::remove_var("RUNIE_SESSIONS_DIR");
}

#[test]
fn test_render_sessions_empty_shows_create_hint() {
    use runie_core::session::Store;
    let _guard = ENV_LOCK.lock().unwrap_or_else(|e| e.into_inner());

    let store = Store::new(std::env::temp_dir().join("runie_render_sessions_empty_test"));
    let _ = std::fs::remove_dir_all(&store.dir);
    std::env::set_var("RUNIE_SESSIONS_DIR", store.dir.clone());

    let content = render_slash("/sessions");
    assert!(content.contains("No saved sessions"), "Should show empty: {}", content);
    assert!(content.contains("/save"), "Should suggest /save: {}", content);

    std::env::remove_var("RUNIE_SESSIONS_DIR");
}

#[test]
fn test_render_at_lookup_popup_shows_on_tab() {
    let backend = TestBackend::new(60, 20);
    let mut terminal = Terminal::new(backend).expect("terminal");
    let mut state = AppState::default();

    for c in "@Car".chars() {
        state.update(Event::Input(c));
    }
    state.update(Event::Input('\t'));

    terminal.draw(|f| view(f, &mut state)).expect("draw");
    let buf = terminal.backend().buffer();
    let content: String = buf.content.iter().map(|c| c.symbol()).collect();
    assert!(content.contains("@ files"), "Popup title must render. Buffer:\n{}", content);
    assert!(content.contains("Cargo") || content.contains("cargo"), "Must show Cargo files. Buffer:\n{}", content);
}

#[test]
fn test_render_at_lookup_popup_shows_immediately() {
    let backend = TestBackend::new(60, 20);
    let mut terminal = Terminal::new(backend).expect("terminal");
    let mut state = AppState::default();

    state.update(Event::Input('@'));
    terminal.draw(|f| view(f, &mut state)).expect("draw");

    let buf = terminal.backend().buffer();
    let content: String = buf.content.iter().map(|c| c.symbol()).collect();
    assert!(content.contains("@ files"), "Popup must show immediately on @. Buffer:\n{}", content);
}

#[test]
fn test_render_at_lookup_tab_cycles_and_enter_inserts() {
    let backend = TestBackend::new(60, 20);
    let mut terminal = Terminal::new(backend).expect("terminal");
    let mut state = AppState::default();

    for c in "@Car".chars() {
        state.update(Event::Input(c));
    }
    state.update(Event::Input('\t'));
    state.update(Event::Input('\t'));
    state.update(Event::Submit);

    terminal.draw(|f| view(f, &mut state)).expect("draw");
    assert!(!state.input.contains('@'), "@ should be replaced. Got: {}", state.input);
    assert!(state.input.contains('[') && state.input.contains(']'), "Should be inserted as [path]. Got: {}", state.input);
}

#[test]
fn test_toggle_expand_changes_rendered_output() {
    let backend = TestBackend::new(60, 20);
    let mut terminal = Terminal::new(backend).expect("terminal");
    let mut state = AppState::default();

    state.messages.push(ChatMessage {
        role: Role::Thought,
        content: "◆ Thought 1.2s\nI'll list the files.".into(),
        timestamp: 0.0,
        id: "t1".into(),
    });

    // Render expanded
    terminal.draw(|f| view(f, &mut state)).expect("draw");
    let buf_expanded = terminal.backend().buffer().clone();
    let expanded_text: String = buf_expanded.content.iter().map(|c| c.symbol()).collect();
    assert!(expanded_text.contains("I'll list the files"), "Expanded thought should show reasoning");

    // Toggle collapse
    state.update(Event::ToggleExpand);

    // Render collapsed
    terminal.draw(|f| view(f, &mut state)).expect("draw");
    let buf_collapsed = terminal.backend().buffer().clone();
    let collapsed_text: String = buf_collapsed.content.iter().map(|c| c.symbol()).collect();
    assert!(collapsed_text.contains("[+]"), "Collapsed thought should show [+] indicator");
    assert!(!collapsed_text.contains("I'll list the files"), "Collapsed thought should hide reasoning");
}

#[test]
fn test_toggle_expand_changes_tool_render() {
    let backend = TestBackend::new(60, 20);
    let mut terminal = Terminal::new(backend).expect("terminal");
    let mut state = AppState::default();

    state.messages.push(ChatMessage {
        role: Role::Tool,
        content: "◆ Ran list_files 0.5s\nfile1\nfile2".into(),
        timestamp: 0.0,
        id: "x1".into(),
    });

    // Render expanded
    terminal.draw(|f| view(f, &mut state)).expect("draw");
    let buf_expanded = terminal.backend().buffer().clone();
    let expanded_text: String = buf_expanded.content.iter().map(|c| c.symbol()).collect();
    assert!(expanded_text.contains("file1"), "Expanded tool should show output");

    // Toggle collapse
    state.update(Event::ToggleExpand);

    // Render collapsed
    terminal.draw(|f| view(f, &mut state)).expect("draw");
    let buf_collapsed = terminal.backend().buffer().clone();
    let collapsed_text: String = buf_collapsed.content.iter().map(|c| c.symbol()).collect();
    assert!(collapsed_text.contains("[+]"), "Collapsed tool should show [+] indicator");
    assert!(!collapsed_text.contains("file1"), "Collapsed tool should hide output");
}

#[test]
fn test_scrollbar_shows_when_content_overflows() {
    let backend = TestBackend::new(40, 10);
    let mut terminal = Terminal::new(backend).expect("terminal");
    let mut state = AppState::default();

    for i in 0..20 {
        state.messages.push(ChatMessage {
            role: Role::User,
            content: format!("Message {} with some text here", i),
            timestamp: i as f64,
            id: format!("u{}", i),
        });
    }

    terminal.draw(|f| view(f, &mut state)).expect("draw");
    let buf = terminal.backend().buffer();
    let area = buf.area();
    let scrollbar_col = 38; // inner x=1, content_width=37, scrollbar at x=38
    let bar_chars: Vec<String> = (1..area.height - 1)
        .map(|y| buf.get(scrollbar_col, y).symbol().to_string())
        .collect();
    assert!(
        bar_chars.iter().any(|s| s == "▐"),
        "Scrollbar thumb should render at col 38. Got: {:?}", bar_chars
    );
}

#[test]
fn test_scrollbar_thumb_at_bottom_by_default() {
    let backend = TestBackend::new(40, 10);
    let mut terminal = Terminal::new(backend).expect("terminal");
    let mut state = AppState::default();

    for i in 0..20 {
        state.messages.push(ChatMessage {
            role: Role::User,
            content: format!("Message {} with some text here", i),
            timestamp: i as f64,
            id: format!("u{}", i),
        });
    }

    terminal.draw(|f| view(f, &mut state)).expect("draw");
    let buf = terminal.backend().buffer();
    // Scrollbar is at x=38 (inner area right column, before border at x=39)
    let scrollbar_col = 38;
    let area = buf.area();
    let bar_chars: Vec<String> = (1..area.height - 1)
        .map(|y| buf.get(scrollbar_col, y).symbol().to_string())
        .collect();
    assert!(
        bar_chars.iter().any(|s| s == "▐"),
        "Thumb should be visible when content overflows. Bar chars: {:?}",
        bar_chars
    );
}

#[test]
fn test_scrollbar_moves_when_scrolled_up() {
    let backend = TestBackend::new(40, 10);
    let mut terminal = Terminal::new(backend).expect("terminal");
    let mut state = AppState::default();

    for i in 0..20 {
        state.messages.push(ChatMessage {
            role: Role::User,
            content: format!("Message {} with some text here", i),
            timestamp: i as f64,
            id: format!("u{}", i),
        });
    }

    // Render at bottom
    terminal.draw(|f| view(f, &mut state)).expect("draw");
    let buf_bottom = terminal.backend().buffer().clone();

    // Scroll up
    state.update(Event::ScrollUp);
    state.update(Event::ScrollUp);
    state.update(Event::ScrollUp);

    terminal.draw(|f| view(f, &mut state)).expect("draw");
    let buf_scrolled = terminal.backend().buffer().clone();

    let area = buf_bottom.area();
    let right_col = area.width - 2;

    let bottom_thumb_y = (0..area.height)
        .find(|y| buf_bottom.get(right_col, *y).symbol() == "▐")
        .expect("thumb at bottom");
    let scrolled_thumb_y = (0..area.height)
        .find(|y| buf_scrolled.get(right_col, *y).symbol() == "▐")
        .expect("thumb when scrolled");

    assert!(
        scrolled_thumb_y < bottom_thumb_y,
        "Thumb should move up when scrolled. bottom_y={} scrolled_y={}",
        bottom_thumb_y, scrolled_thumb_y
    );
}

#[test]
fn test_no_scrollbar_when_content_fits() {
    let backend = TestBackend::new(40, 10);
    let mut terminal = Terminal::new(backend).expect("terminal");
    let mut state = AppState::default();

    // Just 2 messages — fits easily
    state.messages.push(ChatMessage {
        role: Role::User,
        content: "Hello".into(),
        timestamp: 0.0,
        id: "u1".into(),
    });

    terminal.draw(|f| view(f, &mut state)).expect("draw");
    let buf = terminal.backend().buffer();
    let scrollbar_col = 38;
    let area = buf.area();
    let has_thumb = (1..area.height - 1)
        .any(|y| buf.get(scrollbar_col, y).symbol() == "▐");
    assert!(
        !has_thumb,
        "No scrollbar thumb when content fits"
    );
}
