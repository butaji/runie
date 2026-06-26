//! Extra session command tests — export, import, display name
use crate::message::Part;
use crate::model::{AppState, ChatMessage, Role};
use crate::tests::exec;
use crate::tests::fresh_state;
use crate::tests::slash::ENV_LOCK;
use crate::Event;

/// Open palette and select a command by name
fn palette_select(state: &mut AppState, cmd: &str) {
    state.update(crate::Event::Input('/'));
    for c in cmd.chars() {
        state.update(crate::Event::PaletteFilter(c));
    }
    state.update(crate::Event::PaletteSelect);
}

fn imported_session() -> crate::session::Session {
    crate::session::Session {
        name: "imported".to_string(),
        display_name: Some("My Session".to_string()),
        created_at: 1.0,
        updated_at: 2.0,
        messages: vec![ChatMessage {
            role: Role::Assistant,
            timestamp: 0.0,
            id: "resp.0".into(),
            parts: vec![Part::Text {
                content: "imported msg".into(),
            }],
            ..Default::default()
        }],
        provider: "openai".into(),
        model: "gpt-4o".into(),
        theme_name: "tokyo-night".into(),
        thinking_level: crate::model::ThinkingLevel::Medium,
        read_only: true,
        session_tree: None,
    }
}

#[test]
fn name_sets_display_name() {
    let mut state = fresh_state();
    exec(&mut state, "/name my-project"); // Opens form with pre-filled name
    state.update(crate::Event::CommandFormSubmit); // Submits the form
    assert_eq!(
        state.session.session_display_name,
        Some("my-project".to_string())
    );
}

#[test]
fn name_form_submit_via_submit_event_also_works() {
    let mut state = fresh_state();
    exec(&mut state, "/name via-submit-event"); // Opens form
    state.update(Event::submit()); // Submits the form (PaletteSelect path)
    assert_eq!(
        state.session.session_display_name,
        Some("via-submit-event".to_string())
    );
}

#[test]
fn name_form_submit_via_palette_select_event_works() {
    let mut state = fresh_state();
    exec(&mut state, "/name via-palette"); // Opens form
    state.update(crate::Event::PaletteSelect); // Submits the form (panel-stack path)
    assert_eq!(
        state.session.session_display_name,
        Some("via-palette".to_string())
    );
}

#[test]
fn name_shows_current_when_no_args() {
    let mut state = fresh_state();
    state.session.session_display_name = Some("existing".to_string());
    palette_select(&mut state, "name");
    // Form shows with current value pre-filled, submit to see behavior
    state.update(crate::Event::CommandFormSubmit);
    let sys_msgs: Vec<_> = state
        .session
        .messages
        .iter()
        .filter(|m| m.role == Role::System)
        .collect();
    let last = sys_msgs.last().expect("system msg");
    assert!(
        last.content().contains("existing"),
        "shows current name: {}",
        last.content()
    );
}

#[test]
fn name_truncates_long_input() {
    let mut state = fresh_state();
    let long_name = "a".repeat(100);
    state.input.input.push_str("/name ");
    state.input.input.push_str(&long_name);
    state.update(Event::submit()); // Opens form with pre-filled name
    state.update(crate::Event::CommandFormSubmit); // Submits the form
    let name = state.session.session_display_name.as_ref().unwrap();
    assert_eq!(name.chars().count(), 65, "truncated to 64 + ellipsis");
    assert!(name.ends_with('…'), "ends with ellipsis");
}

#[test]
fn export_creates_file() {
    let mut state = fresh_state();
    state.session.messages.push(ChatMessage {
        role: Role::User,
        timestamp: 0.0,
        id: "req.0".into(),
        parts: vec![Part::Text {
            content: "hello".into(),
        }],
        ..Default::default()
    });
    let tmp = std::env::temp_dir().join(format!("runie_export_{}.json", std::process::id()));
    let path = tmp.to_string_lossy();
    exec(&mut state, &format!("/export {}", path)); // Opens form with pre-filled path
    state.update(Event::submit()); // Submits the form
    assert!(tmp.exists(), "export file created");
    let json = std::fs::read_to_string(&tmp).unwrap();
    let session: crate::session::Session = serde_json::from_str(&json).unwrap();
    assert_eq!(session.messages.len(), 1);
    assert_eq!(session.messages[0].content(), "hello");
    let _ = std::fs::remove_file(&tmp);
}

#[test]
fn import_loads_file() {
    let mut state = fresh_state();
    let tmp = std::env::temp_dir().join(format!("runie_import_{}.json", std::process::id()));
    std::fs::write(
        &tmp,
        serde_json::to_string_pretty(&imported_session()).unwrap(),
    )
    .unwrap();
    let path = tmp.to_string_lossy();
    exec(&mut state, &format!("/import {}", path)); // Opens form with pre-filled path
    state.update(Event::submit()); // Submits the form
    assert_eq!(state.session.messages.len(), 2); // imported + system confirmation
    assert_eq!(state.session.messages[0].content(), "imported msg");
    assert_eq!(state.config.current_provider, "openai");
    assert_eq!(state.config.current_model, "gpt-4o");
    assert_eq!(state.config.theme_name, "tokyo-night");
    assert_eq!(
        state.config.thinking_level,
        crate::model::ThinkingLevel::Medium
    );
    assert!(state.config.read_only);
    assert_eq!(
        state.session.session_display_name,
        Some("My Session".to_string())
    );
    let _ = std::fs::remove_file(&tmp);
}

#[test]
fn roundtrip_save_load_preserves_display_name() {
    use crate::session::replay::{load_session, save_snapshot};
    let _guard = ENV_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    let dir = std::env::temp_dir().join(format!("runie_roundtrip_{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    // Create the directory before using it (SessionStore doesn't auto-create)
    std::fs::create_dir_all(&dir).unwrap();
    // FIXME: Audit that the environment access only happens in single-threaded code.
    unsafe { std::env::set_var("RUNIE_SESSIONS_DIR", &dir) };

    let session = crate::session::Session {
        name: "roundtrip".to_string(),
        display_name: Some("Display Name".to_string()),
        created_at: 1.0,
        updated_at: 2.0,
        messages: vec![],
        provider: "mock".into(),
        model: "echo".into(),
        theme_name: "default".into(),
        thinking_level: crate::model::ThinkingLevel::Off,
        read_only: false,
        session_tree: None,
    };
    save_snapshot("roundtrip", &session).unwrap();

    let mut state = crate::model::AppState::default();
    load_session("roundtrip", &mut state).unwrap();
    assert_eq!(
        state.session.session_display_name,
        Some("Display Name".to_string())
    );
    // FIXME: Audit that the environment access only happens in single-threaded code.
    unsafe { std::env::remove_var("RUNIE_SESSIONS_DIR") };
    let _ = std::fs::remove_dir_all(&dir);
}
