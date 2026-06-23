use super::super::*;
use runie_core::event::DialogEvent;
use std::sync::Mutex;

static ENV_LOCK: Mutex<()> = Mutex::new(());

fn type_str(state: &mut AppState, text: &str) {
    runie_testing::type_str(state, text);
    state.update(runie_core::event::InputEvent::Submit);
}

fn render_slash(input: &str) -> String {
    let mut state = AppState::default();
    render_slash_with_state(input, &mut state)
}

fn render_slash_with_state(input: &str, state: &mut AppState) -> String {
    let backend = TestBackend::new(60, 20);
    let mut terminal = Terminal::new(backend).expect("terminal");
    type_str(state, input);
    terminal.draw(|f| view(f, state)).expect("draw");
    let buf = terminal.backend().buffer();
    buf.content.iter().map(|c| c.symbol()).collect()
}

fn buffer_lines(terminal: &Terminal<TestBackend>) -> Vec<String> {
    let buf = terminal.backend().buffer();
    (0..buf.area().height)
        .map(|y| {
            (0..buf.area().width)
                .map(|x| buf[(x, y)].symbol())
                .collect()
        })
        .collect()
}

fn save_test_session(name: &str) {
    let session = runie_core::Session {
        name: name.to_string(),
        created_at: 1.0,
        updated_at: 1.0,
        messages: vec![],
        provider: "mock".into(),
        model: "echo".into(),
        theme_name: "runie".into(),
        thinking_level: runie_core::model::ThinkingLevel::Off,
        read_only: false,
        display_name: None,
        session_tree: None,
    };
    let mut state = AppState::default();
    state.restore_session(&session);
    runie_core::session_replay::save_session(name, &state).unwrap();
}

#[test]
fn test_render_sessions_list_on_separate_lines() {
    let _guard = ENV_LOCK.lock().unwrap_or_else(|e| e.into_inner());

    let dir = std::env::temp_dir().join("runie_render_sessions_test");
    let _ = std::fs::remove_dir_all(&dir);
    std::env::set_var("RUNIE_SESSIONS_DIR", dir);

    save_test_session("alpha");
    save_test_session("beta");

    let backend = TestBackend::new(60, 20);
    let mut terminal = Terminal::new(backend).expect("terminal");
    let mut state = AppState::default();
    type_str(&mut state, "/sessions");
    terminal.draw(|f| view(f, &mut state)).expect("draw");

    let lines = buffer_lines(&terminal);
    let session_line_count = lines
        .iter()
        .filter(|l| l.contains("alpha") || l.contains("beta"))
        .count();
    assert_eq!(
        session_line_count, 2,
        "Sessions should render on 2 separate lines, got: {:?}",
        lines
    );

    std::env::remove_var("RUNIE_SESSIONS_DIR");
}

#[test]
fn test_render_model_no_args_opens_selector() {
    super::super::configure_test_providers(&[("openai".into(), vec!["gpt-4o".into()])]);
    let mut state = AppState::default();
    super::super::apply_test_config_to_state(&mut state);
    let content = render_slash_with_state("/model", &mut state);
    assert!(
        content.contains("Select Model"),
        "Should open model selector dialog: {}",
        content
    );
    assert!(
        !content.contains("❯ /model"),
        "Should NOT echo /model as user message: {}",
        content
    );
}

#[test]
fn test_render_save_no_args_opens_form() {
    let content = render_slash("/save");
    assert!(
        content.contains("Save Session"),
        "Should open save form: {}",
        content
    );
    assert!(
        content.contains("Name"),
        "Should show name field: {}",
        content
    );
    assert!(
        !content.contains("❯ /save"),
        "Should NOT echo /save as user message: {}",
        content
    );
}

#[test]
fn test_render_load_no_args_opens_form() {
    let content = render_slash("/load");
    assert!(
        content.contains("Load Session"),
        "Should open load form: {}",
        content
    );
    assert!(
        content.contains("Name"),
        "Should show name field: {}",
        content
    );
    assert!(
        !content.contains("❯ /load"),
        "Should NOT echo /load as user message: {}",
        content
    );
}

#[test]
fn test_render_delete_no_args_opens_form() {
    let content = render_slash("/delete");
    assert!(
        content.contains("Delete Session"),
        "Should open delete form: {}",
        content
    );
    assert!(
        content.contains("Name"),
        "Should show name field: {}",
        content
    );
    assert!(
        !content.contains("❯ /delete"),
        "Should NOT echo /delete as user message: {}",
        content
    );
}

#[test]
fn test_render_model_m3_just_model_name() {
    super::super::configure_test_providers(&[("mock".into(), vec!["m3".into()])]);
    let backend = TestBackend::new(60, 20);
    let mut terminal = Terminal::new(backend).expect("terminal");
    let mut state = AppState::default();
    super::super::apply_test_config_to_state(&mut state);
    state.config.current_provider = "mock".into();
    state.config.current_model = "m1".into();

    for c in "/model m3".chars() {
        state.update(InputEvent::Input(c));
    }
    state.update(InputEvent::Submit);
    terminal.draw(|f| view(f, &mut state)).expect("draw");
    let buf = terminal.backend().buffer();
    let content: String = buf.content.iter().map(|c| c.symbol()).collect();
    assert!(
        content.contains("Switched to mock/m3"),
        "/model m3 should render: {}",
        content
    );
    assert!(
        !content.contains("❯ /model m3"),
        "Should NOT echo /model m3 as user message: {}",
        content
    );
    assert_eq!(state.config.current_model, "m3");
}

#[test]
fn test_render_load_missing_shows_user_friendly_error() {
    let _guard = ENV_LOCK.lock().unwrap_or_else(|e| e.into_inner());

    let dir = std::env::temp_dir().join("runie_render_load_missing_test");
    let _ = std::fs::remove_dir_all(&dir);
    std::env::set_var("RUNIE_SESSIONS_DIR", dir);

    // Test that /load opens a form, then submit to trigger the error
    let backend = TestBackend::new(60, 20);
    let mut terminal = Terminal::new(backend).expect("terminal");
    let mut state = AppState::default();
    for c in "/load missing".chars() {
        state.update(InputEvent::Input(c));
    }
    state.update(InputEvent::Submit); // Opens form with pre-filled name
    state.update(DialogEvent::CommandFormSubmit); // Submits form, triggers error
    terminal.draw(|f| view(f, &mut state)).expect("draw");
    let buf = terminal.backend().buffer();
    let content: String = buf.content.iter().map(|c| c.symbol()).collect();

    assert!(
        content.contains("not found"),
        "Should show not found: {}",
        content
    );
    assert!(
        content.contains("/sessions"),
        "Should suggest /sessions: {}",
        content
    );
    assert!(
        !content.contains("❯ /load missing"),
        "Should NOT echo as user message: {}",
        content
    );

    std::env::remove_var("RUNIE_SESSIONS_DIR");
}

#[test]
fn test_render_sessions_empty_shows_create_hint() {
    let _guard = ENV_LOCK.lock().unwrap_or_else(|e| e.into_inner());

    let dir = std::env::temp_dir().join("runie_render_sessions_empty_test");
    let _ = std::fs::remove_dir_all(&dir);
    std::env::set_var("RUNIE_SESSIONS_DIR", dir);

    let content = render_slash("/sessions");
    assert!(
        content.contains("No saved sessions"),
        "Should show empty: {}",
        content
    );
    assert!(
        content.contains("/save"),
        "Should suggest /save: {}",
        content
    );

    std::env::remove_var("RUNIE_SESSIONS_DIR");
}
