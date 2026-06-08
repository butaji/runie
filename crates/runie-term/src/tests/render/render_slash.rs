use super::super::*;
use std::sync::Mutex;

static ENV_LOCK: Mutex<()> = Mutex::new(());

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
        .map(|y| (0..buf.area().width).map(|x| buf[(x, y)].symbol()).collect::<String>())
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
    assert!(!content.contains("$ /model"), "Should NOT echo /model as user message: {}", content);
}

#[test]
fn test_render_save_no_args_shows_usage() {
    let content = render_slash("/save");
    assert!(content.contains("Usage:"), "Should show usage: {}", content);
    assert!(!content.contains("$ /save"), "Should NOT echo /save as user message: {}", content);
}

#[test]
fn test_render_load_no_args_shows_usage() {
    let content = render_slash("/load");
    assert!(content.contains("Usage:"), "Should show usage: {}", content);
    assert!(!content.contains("$ /load"), "Should NOT echo /load as user message: {}", content);
}

#[test]
fn test_render_delete_no_args_shows_usage() {
    let content = render_slash("/delete");
    assert!(content.contains("Usage:"), "Should show usage: {}", content);
    assert!(!content.contains("$ /delete"), "Should NOT echo /delete as user message: {}", content);
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
    assert!(!content.contains("$ /model m3"), "Should NOT echo /model m3 as user message: {}", content);
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
    assert!(!content.contains("$ /load missing"), "Should NOT echo as user message: {}", content);

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
