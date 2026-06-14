use crate::commands::handlers::subagent::handle_spawn;
use crate::commands::CommandResult;
use crate::dialog::PanelItem;
use crate::event::Event;
use crate::model::AppState;

#[test]
fn spawn_without_args_opens_form() {
    let mut state = AppState::default();
    let result = handle_spawn(&mut state, "");

    match result {
        CommandResult::OpenPanelStack(_) => {}
        CommandResult::Message(msg) => panic!(
            "spawn without args should open form, not show message: {}",
            msg
        ),
        CommandResult::Warning(msg) => {
            panic!("spawn without args should open form, not warn: {}", msg)
        }
        other => panic!("unexpected result: {:?}", other),
    }
}

#[test]
fn spawn_with_whitespace_only_opens_form() {
    let mut state = AppState::default();
    let result = handle_spawn(&mut state, "   \t  ");
    match result {
        CommandResult::OpenPanelStack(_) => {}
        other => panic!("expected form dialog, got {:?}", other),
    }
}

#[test]
fn spawn_with_args_emits_event() {
    let mut state = AppState::default();
    let result = handle_spawn(&mut state, "list files in /tmp");
    match result {
        CommandResult::Event(Event::SpawnAgent { prompt }) => {
            assert_eq!(prompt, "list files in /tmp");
        }
        other => panic!("expected SpawnAgent event, got {:?}", other),
    }
}

#[test]
fn spawn_trims_whitespace_from_args() {
    let mut state = AppState::default();
    let result = handle_spawn(&mut state, "  hello world  ");
    match result {
        CommandResult::Event(Event::SpawnAgent { prompt }) => {
            assert_eq!(prompt, "hello world");
        }
        other => panic!("expected event, got {:?}", other),
    }
}

#[test]
fn spawn_form_panel_has_prompt_field() {
    let mut state = AppState::default();
    let result = handle_spawn(&mut state, "");
    if let CommandResult::OpenPanelStack(stack) = result {
        let panel = stack.current().unwrap();
        let has_prompt_field = panel.form_values.keys().any(|k| k == "prompt")
            || panel.items.iter().any(|it| {
                if let PanelItem::FormField { key, .. } = it {
                    key == "prompt"
                } else {
                    false
                }
            });
        assert!(has_prompt_field, "spawn form should have 'prompt' field");
    } else {
        panic!("expected panel stack");
    }
}
