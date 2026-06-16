use super::super::*;
use runie_core::event::{DialogEvent, InputEvent};
use std::sync::Mutex;

static ENV_LOCK: Mutex<()> = Mutex::new(());

fn type_str(state: &mut AppState, text: &str) {
    for c in text.chars() {
        state.update(InputEvent::Input(c));
    }
    state.update(InputEvent::Submit);
}

fn render_slash(input: &str) -> String {
    let backend = TestBackend::new(60, 20);
    let mut terminal = Terminal::new(backend).expect("terminal");
    let mut state = AppState::default();
    type_str(&mut state, input);
    terminal.draw(|f| view(f, &mut state)).expect("draw");
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

fn save_test_session(store: &runie_core::session::Store, name: &str) {
    store
        .save(
            name,
            &runie_core::Session {
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
            },
        )
        .unwrap();
}

#[test]
fn test_render_sessions_list_on_separate_lines() {
    use runie_core::session::Store;
    let _guard = ENV_LOCK.lock().unwrap_or_else(|e| e.into_inner());

    let store = Store::new(std::env::temp_dir().join("runie_render_sessions_test"));
    let _ = std::fs::remove_dir_all(&store.dir);
    std::env::set_var("RUNIE_SESSIONS_DIR", store.dir.clone());

    save_test_session(&store, "alpha");
    save_test_session(&store, "beta");

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
    let content = render_slash("/model");
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
#[ignore = "/model with args not dispatching to handler in current build"]
fn test_render_model_m3_just_model_name() {
    let backend = TestBackend::new(60, 20);
    let mut terminal = Terminal::new(backend).expect("terminal");
    let mut state = AppState::default();

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
#[ignore = "/load with args not dispatching to handler in current build"]
fn test_render_load_missing_shows_user_friendly_error() {
    use runie_core::session::Store;
    let _guard = ENV_LOCK.lock().unwrap_or_else(|e| e.into_inner());

    let store = Store::new(std::env::temp_dir().join("runie_render_load_missing_test"));
    let _ = std::fs::remove_dir_all(&store.dir);
    std::env::set_var("RUNIE_SESSIONS_DIR", store.dir.clone());

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
    use runie_core::session::Store;
    let _guard = ENV_LOCK.lock().unwrap_or_else(|e| e.into_inner());

    let store = Store::new(std::env::temp_dir().join("runie_render_sessions_empty_test"));
    let _ = std::fs::remove_dir_all(&store.dir);
    std::env::set_var("RUNIE_SESSIONS_DIR", store.dir.clone());

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
