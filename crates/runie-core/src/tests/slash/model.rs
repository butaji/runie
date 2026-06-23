use super::{exec, tmp_store, ENV_LOCK};
use crate::event::Event;
use crate::event::DialogEvent;
use crate::model::Role;
use crate::tests::{fresh_state, type_str};

/// Open palette and select a command by name
fn palette_select(state: &mut crate::model::AppState, cmd: &str) {
    state.update(DialogEvent::ToggleCommandPalette);
    for c in cmd.chars() {
        state.update(DialogEvent::PaletteFilter(c));
    }
    state.update(DialogEvent::PaletteSelect);
}

#[test]
fn model_gpt4o_just_model_name() {
    crate::login_config::set_test_config_with_providers(&[(
        "openai".into(),
        vec!["gpt-4o".into(), "gpt-4o-mini".into()],
    )]);
    let mut state = fresh_state();
    state.config.current_provider = "openai".into();
    state.config.current_model = "gpt-4o-mini".into();
    exec(&mut state, "/model gpt-4o");

    assert_eq!(state.config.current_provider, "openai");
    assert_eq!(state.config.current_model, "gpt-4o");
    let sys_msgs: Vec<_> = state
        .session
        .messages
        .iter()
        .filter(|m| m.role == Role::System)
        .collect();
    assert_eq!(sys_msgs.len(), 1);
    assert!(
        sys_msgs[0].content().contains("Switched to openai/gpt-4o"),
        "/model gpt-4o should work: {}",
        sys_msgs[0].content()
    );
}

#[test]
fn model_leading_slash_ignored_for_model_name() {
    crate::login_config::set_test_config_with_providers(&[(
        "openai".into(),
        vec!["gpt-4o".into(), "gpt-4o-mini".into()],
    )]);
    let mut state = fresh_state();
    state.config.current_provider = "openai".into();
    state.config.current_model = "gpt-4o".into();
    let initial_provider = state.config.current_provider.clone();
    exec(&mut state, "/model /gpt-4o-mini");

    assert_eq!(state.config.current_provider, initial_provider);
    assert_eq!(state.config.current_model, "gpt-4o-mini");
    let sys_msgs: Vec<_> = state
        .session
        .messages
        .iter()
        .filter(|m| m.role == Role::System)
        .collect();
    assert_eq!(sys_msgs.len(), 1);
    assert!(
        sys_msgs[0]
            .content()
            .contains("Switched to openai/gpt-4o-mini"),
        "leading slash ignored: {}",
        sys_msgs[0].content()
    );
}

#[test]
fn model_only_slashes_shows_usage() {
    let mut state = fresh_state();
    exec(&mut state, "/model /");

    let sys_msgs: Vec<_> = state
        .session
        .messages
        .iter()
        .filter(|m| m.role == Role::System)
        .collect();
    assert_eq!(sys_msgs.len(), 1);
    assert!(
        sys_msgs[0].content().contains("Current:"),
        "only slashes shows usage: {}",
        sys_msgs[0].content()
    );
}

#[test]
fn slash_opens_palette_and_typing_filters_commands() {
    // Typing "/" opens command palette, then typing filters commands
    crate::login_config::set_test_config_with_providers(&[(
        "openai".into(),
        vec!["gpt-4o".into(), "gpt-4o-mini".into()],
    )]);
    let mut state = fresh_state();
    state.config.vim_mode = false;
    state.config.current_provider = "openai".into();
    state.config.current_model = "gpt-4o-mini".into();

    // Type "/" to open palette, then "model" to filter to the model command
    type_str(&mut state, "/model");

    // Verify the palette is open with "model" as filter
    let stack = match &state.open_dialog {
        Some(crate::commands::DialogState::CommandPalette(s)) => s,
        _ => panic!("Expected command palette"),
    };
    let panel = stack.current().expect("panel");
    assert_eq!(panel.filter, "model");
    // The model command should be first in filtered results
    let selected_label = panel.selected_item().expect("selected item").label();
    assert!(selected_label.expect("label").starts_with("model "), "Expected model command, got: {selected_label:?}");
}

#[test]
fn model_no_args_opens_selector() {
    crate::login_config::set_test_config_with_providers(&[(
        "openai".into(),
        vec!["gpt-4o".into()],
    )]);
    let mut state = fresh_state();
    palette_select(&mut state, "model");

    assert!(
        matches!(
            state.open_dialog,
            Some(crate::commands::DialogState::ModelSelector { .. })
        ),
        "no args should open model selector dialog"
    );
}

#[test]
fn provider_dialog_shows_edit_models_action() {
    crate::login_config::set_test_config_with_providers(&[(
        "openai".into(),
        vec!["gpt-4o".into(), "gpt-4o-mini".into()],
    )]);
    let mut state = fresh_state();
    exec(&mut state, "/provider");

    let dialog = state.open_dialog.expect("dialog should be open");
    let stack = dialog.panel_stack().expect("panel stack");
    let panel = stack.current().expect("panel");
    assert!(
        panel
            .items
            .iter()
            .any(|i| i.label().is_some_and(|l| l.contains("Edit models"))),
        "provider dialog should offer Edit models action"
    );
}

#[test]
fn save_creates_session_file() {
    let _guard = ENV_LOCK.lock().unwrap_or_else(|e| e.into_inner());

    let store = tmp_store();
    std::env::set_var("RUNIE_SESSIONS_DIR", store.dir().to_path_buf());

    let mut state = fresh_state();
    type_str(&mut state, "hello world");
    state.update(Event::submit());
    exec(&mut state, "/save mysession"); // Opens form with pre-filled name
    state.update(Event::submit()); // Submits the form

    let redb_path = crate::session_store::SessionStore::new(store.dir().to_path_buf()).path("mysession");
    assert!(redb_path.exists(), "session file created");

    let sys_msgs: Vec<_> = state
        .session
        .messages
        .iter()
        .filter(|m| m.role == Role::System)
        .collect();
    let last = sys_msgs.last().expect("system msg");
    assert!(last.content().contains("saved"), "confirmation shown");

    std::env::remove_var("RUNIE_SESSIONS_DIR");
}

#[test]
fn save_preserves_messages_provider_model() {
    let _guard = ENV_LOCK.lock().unwrap_or_else(|e| e.into_inner());

    let store = tmp_store();
    std::env::set_var("RUNIE_SESSIONS_DIR", store.dir().to_path_buf());

    let mut state = fresh_state();
    state.config.current_provider = "openai".to_string();
    state.config.current_model = "gpt-4o".to_string();
    type_str(&mut state, "test message");
    state.update(Event::submit());
    exec(&mut state, "/save preserved"); // Opens form with pre-filled name
    state.update(Event::submit()); // Submits the form

    let redb_store = crate::session_store::SessionStore::new(store.dir().to_path_buf());
    let events = redb_store.load_events("preserved").unwrap();
    let mut loaded = crate::model::AppState::default();
    crate::session_replay::replay_events(&mut loaded, &events);
    assert_eq!(loaded.config.current_provider, "openai");
    assert_eq!(loaded.config.current_model, "gpt-4o");
    assert_eq!(loaded.session.messages.len(), 1);
    assert_eq!(loaded.session.messages[0].content(), "test message");
    assert_eq!(loaded.session.messages[0].role, Role::User);

    std::env::remove_var("RUNIE_SESSIONS_DIR");
}

#[test]
fn save_no_args_opens_form() {
    let mut state = fresh_state();
    palette_select(&mut state, "save");

    // Should open form dialog
    assert!(state.open_dialog.is_some(), "should open dialog");
    if let Some(crate::commands::DialogState::PanelStack(stack)) = &state.open_dialog {
        let panel = stack.current().expect("should have panel");
        assert_eq!(panel.id, "save", "should be save form");
    } else {
        panic!("expected PanelStack dialog");
    }
}
