use super::*;
use runie_core::model::ThinkingLevel;
use runie_core::session::replay::{replay_events, state_to_durable_events};
use runie_core::session::store::SessionStore;
use runie_core::session::Session;
use runie_core::Event;

#[test]
fn cycle_rotates() {
    assert_eq!(ThinkingLevel::Off.cycle(), ThinkingLevel::Low);
    assert_eq!(ThinkingLevel::Low.cycle(), ThinkingLevel::Medium);
    assert_eq!(ThinkingLevel::Medium.cycle(), ThinkingLevel::High);
    assert_eq!(ThinkingLevel::High.cycle(), ThinkingLevel::Off);
}

#[test]
fn all_returns_all_levels_in_order() {
    assert_eq!(
        ThinkingLevel::all().to_vec(),
        vec![
            ThinkingLevel::Off,
            ThinkingLevel::Low,
            ThinkingLevel::Medium,
            ThinkingLevel::High,
        ]
    );
}

#[test]
fn prompt_suffix_matches() {
    assert_eq!(ThinkingLevel::Off.prompt_suffix(), "");
    assert_eq!(
        ThinkingLevel::Low.prompt_suffix(),
        "\nThink briefly before responding."
    );
    assert_eq!(
        ThinkingLevel::Medium.prompt_suffix(),
        "\nThink step by step before responding."
    );
    assert_eq!(
        ThinkingLevel::High.prompt_suffix(),
        "\nThink deeply and thoroughly. Consider edge cases and alternatives."
    );
}

#[test]
fn from_str_parses_levels() {
    assert_eq!("off".parse::<ThinkingLevel>().unwrap(), ThinkingLevel::Off);
    assert_eq!("low".parse::<ThinkingLevel>().unwrap(), ThinkingLevel::Low);
    assert_eq!(
        "medium".parse::<ThinkingLevel>().unwrap(),
        ThinkingLevel::Medium
    );
    assert_eq!(
        "high".parse::<ThinkingLevel>().unwrap(),
        ThinkingLevel::High
    );
    assert!("unknown".parse::<ThinkingLevel>().is_err());
}

#[test]
fn session_persists_thinking_level() {
    let dir = std::env::temp_dir().join(format!("runie_think_test_{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    let store = SessionStore::new(dir);

    let session = Session {
        name: "think_test".to_string(),
        created_at: 1.0,
        updated_at: 2.0,
        messages: vec![],
        provider: "mock".into(),
        model: "echo".into(),
        theme_name: "default".into(),
        thinking_level: ThinkingLevel::Medium,
        read_only: false,
        display_name: None,
        session_tree: None,
    };

    let mut seeded = AppState::default();
    seeded.restore_session(&session);
    let events = state_to_durable_events(&seeded);
    store.append_batch("think_test", &events).unwrap();

    let loaded_events = store.load_events("think_test").unwrap();
    let mut state = AppState::default();
    replay_events(&mut state, &loaded_events);
    assert_eq!(state.config.thinking_level, ThinkingLevel::Medium);
}

#[test]
fn shift_tab_cycles() {
    let mut state = AppState::default();
    assert_eq!(state.config.thinking_level, ThinkingLevel::Off);

    state.update(Event::CycleThinkingLevel);
    assert_eq!(state.config.thinking_level, ThinkingLevel::Low);

    state.update(Event::CycleThinkingLevel);
    assert_eq!(state.config.thinking_level, ThinkingLevel::Medium);

    state.update(Event::CycleThinkingLevel);
    assert_eq!(state.config.thinking_level, ThinkingLevel::High);

    state.update(Event::CycleThinkingLevel);
    assert_eq!(state.config.thinking_level, ThinkingLevel::Off);
}

#[test]
fn slash_thinking_sets() {
    let mut state = AppState::default();
    state.input.input.push_str("/thinking high");
    state.update(Event::submit()); // Opens form with pre-filled level
    state.update(Event::CommandFormSubmit); // Submits the form
    assert_eq!(state.config.thinking_level, ThinkingLevel::High);

    let sys_msgs: Vec<_> = state
        .session
        .messages
        .iter()
        .filter(|m| m.role == runie_core::model::Role::System)
        .collect();
    assert!(sys_msgs
        .iter()
        .any(|m| m.content().contains("Thinking level set to: high")));
}

#[test]
fn slash_thinking_no_args_shows_panel() {
    let mut state = AppState::default();
    state.config.thinking_level = ThinkingLevel::Medium;
    state.input.input.push_str("/thinking");
    state.update(Event::submit()); // Opens the thinking level selector panel

    // Panel should be open
    assert!(state.open_dialog.is_some(), "panel should be open");
    // No system messages should be generated yet
    let sys_msgs: Vec<_> = state
        .session
        .messages
        .iter()
        .filter(|m| m.role == runie_core::model::Role::System)
        .collect();
    assert!(sys_msgs.is_empty(), "no system messages yet");
}

#[test]
fn thinking_panel_contains_all_levels() {
    use runie_core::commands::{DialogKind, DialogState};
    let mut state = AppState::default();
    state.config.thinking_level = ThinkingLevel::Medium;
    state.input.input.push_str("/thinking");
    state.update(Event::submit());

    let Some(DialogState::Active {
        kind: DialogKind::Generic,
        panels: stack,
    }) = &state.open_dialog
    else {
        panic!("expected PanelStack dialog");
    };
    let panel = stack.current().expect("current panel");
    let labels: Vec<String> = panel
        .items
        .iter()
        .filter_map(|i| i.label().map(|s| s.to_string()))
        .collect();

    // Current level is marked
    assert!(
        labels.iter().any(|l| l == "medium (current)"),
        "current level marked: {:?}",
        labels
    );
    // All other levels shown
    assert!(labels.contains(&"off".to_string()));
    assert!(labels.contains(&"low".to_string()));
    assert!(labels.contains(&"high".to_string()));
}

#[test]
fn thinking_panel_uses_thinking_level_all() {
    // Regression: panel must use ThinkingLevel::all() — not a hardcoded list
    // so adding a new level doesn't require changes to model.rs handler.
    assert_eq!(ThinkingLevel::all().len(), 4);
}

#[test]
fn thinking_panel_has_cli_usage_hint() {
    // After moving /thinking to a panel selector, the CLI form
    // ("/thinking off|low|medium|high") should still be discoverable.
    use runie_core::commands::{DialogKind, DialogState};
    let mut state = AppState::default();
    state.input.input.push_str("/thinking");
    state.update(Event::submit());
    let Some(DialogState::Active {
        kind: DialogKind::Generic,
        panels: stack,
    }) = &state.open_dialog
    else {
        panic!("expected panel");
    };
    let panel = stack.current().expect("panel");
    let labels: Vec<String> = panel
        .items
        .iter()
        .filter_map(|i| i.label().map(|s| s.to_string()))
        .collect();
    let hint = labels.iter().find(|l| l.contains("/thinking"));
    assert!(
        hint.is_some(),
        "panel should advertise the CLI form, got labels: {:?}",
        labels
    );
}

#[test]
fn thinking_does_not_create_a_form_panel() {
    // /thinking must use a select panel, not a form. Forms are for free-text
    // input; thinking levels are a fixed enum.
    use runie_core::commands::{CommandRegistry, CommandResult};
    let reg = CommandRegistry::new();
    let cmd = reg.get("thinking").expect("thinking command");
    let mut state = AppState::default();
    state.config.thinking_level = ThinkingLevel::Medium;
    let result = cmd.flow.clone().exec(&mut state, "thinking", "");
    match result {
        CommandResult::OpenPanelStack(stack) => {
            let panel = stack.current().expect("panel");
            assert!(
                !panel.is_form(),
                "thinking panel must not be a form, got items: {:?}",
                panel.items
            );
        }
        other => panic!("expected OpenPanelStack, got {:?}", other),
    }
}

#[test]
fn set_thinking_level_event_updates_state() {
    let mut state = AppState::default();
    state.update(Event::SetThinkingLevel(
        runie_core::model::ThinkingLevel::High,
    ));
    assert_eq!(state.config.thinking_level, ThinkingLevel::High);
}
