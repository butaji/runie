//! Scoped models tests (Layer 1 + Layer 2)

use crate::commands::DialogState;
use crate::event::{ControlEvent, DialogEvent, Event, InputEvent, ModelConfigEvent};
use crate::model::{AppState, ScopedModel};

fn sm(provider: &str, name: &str, enabled: bool) -> ScopedModel {
    ScopedModel {
        provider: provider.into(),
        name: name.into(),
        enabled,
    }
}

/// Open palette and select a command by name
fn palette_select(state: &mut AppState, cmd: &str) {
    state.update(InputEvent::Input('/'));
    for c in cmd.chars() {
        state.update(DialogEvent::PaletteFilter(c));
    }
    state.update(DialogEvent::PaletteSelect);
}

#[test]
fn toggle_model_excludes_from_cycle() {
    let mut state = AppState::default();
    state.config.scoped_models = vec![sm("mock", "echo", true), sm("openai", "gpt-4o", true)];

    state.update(ModelConfigEvent::ScopedModelToggle {
        name: "gpt-4o".to_string(),
    });

    assert!(!state.config.scoped_models[1].enabled);
}

#[test]
fn enable_all_includes_all() {
    let mut state = AppState::default();
    state.config.scoped_models = vec![sm("mock", "echo", false), sm("openai", "gpt-4o", false)];

    state.update(ModelConfigEvent::ScopedModelEnableAll);

    assert!(state.config.scoped_models.iter().all(|m| m.enabled));
}

#[test]
fn disable_all_excludes_all() {
    let mut state = AppState::default();
    state.config.scoped_models = vec![sm("mock", "echo", true), sm("openai", "gpt-4o", true)];

    state.update(ModelConfigEvent::ScopedModelDisableAll);

    assert!(state.config.scoped_models.iter().all(|m| !m.enabled));
}

#[test]
fn provider_toggle_affects_all() {
    let mut state = AppState::default();
    state.config.scoped_models = vec![
        sm("openai", "gpt-4o", true),
        sm("openai", "gpt-4o-mini", true),
        sm("anthropic", "claude-3", true),
    ];

    state.update(ModelConfigEvent::ScopedModelToggleProvider {
        provider: "openai".to_string(),
    });

    assert!(!state.config.scoped_models[0].enabled);
    assert!(!state.config.scoped_models[1].enabled);
    assert!(state.config.scoped_models[2].enabled);
}

#[test]
fn provider_toggle_re_enables_all() {
    let mut state = AppState::default();
    state.config.scoped_models = vec![
        sm("openai", "gpt-4o", false),
        sm("openai", "gpt-4o-mini", true),
        sm("anthropic", "claude-3", true),
    ];

    state.update(ModelConfigEvent::ScopedModelToggleProvider {
        provider: "openai".to_string(),
    });

    assert!(state.config.scoped_models[0].enabled);
    assert!(state.config.scoped_models[1].enabled);
    assert!(state.config.scoped_models[2].enabled);
}

fn scoped_selected(state: &AppState) -> Option<usize> {
    match &state.open_dialog {
        Some(DialogState::ScopedModels(stack)) => stack.current().map(|p| p.selected),
        _ => None,
    }
}

#[test]
fn slash_scoped_models_opens_dialog() {
    let mut state = AppState::default();
    state.config.scoped_models = vec![sm("mock", "echo", true), sm("openai", "gpt-4o", true)];

    palette_select(&mut state, "scoped-models");

    assert!(
        scoped_selected(&state).is_some(),
        "Expected ScopedModels dialog, got {:?}",
        state.open_dialog
    );
}

#[test]
fn scoped_models_dialog_navigates_up() {
    let mut state = AppState::default();
    state.config.scoped_models = vec![
        sm("mock", "echo", true),
        sm("openai", "gpt-4o", true),
        sm("anthropic", "claude-3", true),
    ];
    state.update(ModelConfigEvent::ToggleScopedModelsDialog);

    state.update(InputEvent::HistoryPrev);

    assert_eq!(
        scoped_selected(&state).unwrap(),
        2,
        "Up at first should wrap to last"
    );
}

#[test]
fn scoped_models_dialog_navigates_down() {
    let mut state = AppState::default();
    state.config.scoped_models = vec![
        sm("mock", "echo", true),
        sm("openai", "gpt-4o", true),
        sm("anthropic", "claude-3", true),
    ];
    state.update(ModelConfigEvent::ToggleScopedModelsDialog);
    state.update(InputEvent::HistoryNext);
    state.update(InputEvent::HistoryNext);
    state.update(InputEvent::HistoryNext);

    assert_eq!(
        scoped_selected(&state).unwrap(),
        0,
        "Down at last should wrap to first"
    );
}

#[test]
fn scoped_models_dialog_submit_toggles() {
    let mut state = AppState::default();
    state.config.scoped_models = vec![sm("mock", "echo", true), sm("openai", "gpt-4o", true)];
    state.update(ModelConfigEvent::ToggleScopedModelsDialog);
    state.update(InputEvent::HistoryNext);

    state.update(Event::submit());

    assert!(!state.config.scoped_models[1].enabled);
    assert!(scoped_selected(&state).is_some());
}

#[test]
fn scoped_models_dialog_esc_closes() {
    let mut state = AppState::default();
    state.config.scoped_models = vec![sm("mock", "echo", true)];
    state.update(ModelConfigEvent::ToggleScopedModelsDialog);

    state.update(ControlEvent::Abort);

    assert!(state.open_dialog.is_none());
}
