//! Extra session command tests — export, import, display name
use crate::model::{AppState, ChatMessage, Role};
use crate::event::Event;

fn fresh_state() -> AppState {
    AppState::default()
}

fn type_str(state: &mut AppState, text: &str) {
    for c in text.chars() {
        state.update(Event::Input(c));
    }
}

#[test]
fn name_sets_display_name() {
    let mut state = fresh_state();
    type_str(&mut state, "/name my-project");
    state.update(Event::Submit);
    assert_eq!(state.session.session_display_name, Some("my-project".to_string()));
}

#[test]
fn name_shows_current_when_no_args() {
    let mut state = fresh_state();
    state.session.session_display_name = Some("existing".to_string());
    type_str(&mut state, "/name");
    state.update(Event::Submit);
    let sys_msgs: Vec<_> = state.session.messages.iter().filter(|m| m.role == Role::System).collect();
    let last = sys_msgs.last().expect("system msg");
    assert!(last.content.contains("existing"), "shows current name: {}", last.content);
}

#[test]
fn name_truncates_long_input() {
    let mut state = fresh_state();
    let long_name = "a".repeat(100);
    state.input.input.push_str("/name ");
    state.input.input.push_str(&long_name);
    state.update(Event::Submit);
    let name = state.session.session_display_name.as_ref().unwrap();
    assert_eq!(name.chars().count(), 65, "truncated to 64 + ellipsis");
    assert!(name.ends_with('…'), "ends with ellipsis");
}

#[test]
fn export_creates_file() {
    let mut state = fresh_state();
    state.session.messages.push(ChatMessage {
        role: Role::User,
        content: "hello".into(),
        timestamp: 0.0,
        id: "req.0".into(),
        ..Default::default()
    });
    let tmp = std::env::temp_dir().join(format!("runie_export_{}.json", std::process::id()));
    let path = tmp.to_string_lossy();
    type_str(&mut state, &format!("/export {}", path));
    state.update(Event::Submit);
    assert!(tmp.exists(), "export file created");
    let json = std::fs::read_to_string(&tmp).unwrap();
    let session: crate::session::Session = serde_json::from_str(&json).unwrap();
    assert_eq!(session.messages.len(), 1);
    assert_eq!(session.messages[0].content, "hello");
    let _ = std::fs::remove_file(&tmp);
}

#[test]
fn import_loads_file() {
    let mut state = fresh_state();
    let tmp = std::env::temp_dir().join(format!("runie_import_{}.json", std::process::id()));
    let session = crate::session::Session {
        name: "imported".to_string(),
        display_name: Some("My Session".to_string()),
        created_at: 1.0,
        updated_at: 2.0,
        messages: vec![ChatMessage {
            role: Role::Assistant,
            content: "imported msg".into(),
            timestamp: 0.0,
            id: "resp.0".into(),
            ..Default::default()
        }],
        provider: "openai".into(),
        model: "gpt-4o".into(),
        theme_name: "tokyo-night".into(),
        thinking_level: crate::model::ThinkingLevel::Medium,
        read_only: true,
        session_tree: None,
    };
    std::fs::write(&tmp, serde_json::to_string_pretty(&session).unwrap()).unwrap();

    let path = tmp.to_string_lossy();
    type_str(&mut state, &format!("/import {}", path));
    state.update(Event::Submit);

    assert_eq!(state.session.messages.len(), 2); // imported + system confirmation
    assert_eq!(state.session.messages[0].content, "imported msg");
    assert_eq!(state.config.current_provider, "openai");
    assert_eq!(state.config.current_model, "gpt-4o");
    assert_eq!(state.config.theme_name, "tokyo-night");
    assert_eq!(state.config.thinking_level, crate::model::ThinkingLevel::Medium);
    assert!(state.config.read_only);
    assert_eq!(state.session.session_display_name, Some("My Session".to_string()));
    let _ = std::fs::remove_file(&tmp);
}

#[test]
fn roundtrip_save_load_preserves_display_name() {
    use crate::session::Store;
    let dir = std::env::temp_dir().join(format!("runie_roundtrip_{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    let store = Store::new(dir.clone());

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
    store.save("roundtrip", &session).unwrap();
    let loaded = store.load("roundtrip").unwrap();
    assert_eq!(loaded.display_name, Some("Display Name".to_string()));
    let _ = std::fs::remove_dir_all(&dir);
}
