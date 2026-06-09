//! Scoped models tests (Layer 1 + Layer 2)

use crate::commands::DialogState;
use crate::event::Event;
use crate::model::{AppState, ScopedModel};

fn sm(provider: &str, name: &str, enabled: bool) -> ScopedModel {
    ScopedModel {
        provider: provider.into(),
        name: name.into(),
        enabled,
    }
}

#[test]
fn toggle_model_excludes_from_cycle() {
    let mut state = AppState::default();
    state.config.scoped_models = vec![
        sm("mock", "echo", true),
        sm("openai", "gpt-4o", true),
    ];

    state.update(Event::ScopedModelToggle {
        name: "gpt-4o".to_string(),
    });

    assert!(!state.config.scoped_models[1].enabled);
}

#[test]
fn enable_all_includes_all() {
    let mut state = AppState::default();
    state.config.scoped_models = vec![
        sm("mock", "echo", false),
        sm("openai", "gpt-4o", false),
    ];

    state.update(Event::ScopedModelEnableAll);

    assert!(state.config.scoped_models.iter().all(|m| m.enabled));
}

#[test]
fn disable_all_excludes_all() {
    let mut state = AppState::default();
    state.config.scoped_models = vec![
        sm("mock", "echo", true),
        sm("openai", "gpt-4o", true),
    ];

    state.update(Event::ScopedModelDisableAll);

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

    state.update(Event::ScopedModelToggleProvider {
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

    state.update(Event::ScopedModelToggleProvider {
        provider: "openai".to_string(),
    });

    assert!(state.config.scoped_models[0].enabled);
    assert!(state.config.scoped_models[1].enabled);
    assert!(state.config.scoped_models[2].enabled);
}

#[test]
fn slash_scoped_models_opens_dialog() {
    let mut state = AppState::default();
    state.config.scoped_models = vec![
        sm("mock", "echo", true),
        sm("openai", "gpt-4o", true),
    ];

    for c in "/scoped-models".chars() {
        state.update(Event::Input(c));
    }
    state.update(Event::Submit);

    assert!(
        matches!(state.open_dialog, Some(DialogState::ScopedModels { .. })),
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
    state.open_dialog = Some(DialogState::ScopedModels { selected: 0 });

    state.update(Event::HistoryPrev);

    if let Some(DialogState::ScopedModels { selected }) = state.open_dialog {
        assert_eq!(selected, 2, "Up at first should wrap to last");
    } else {
        panic!("Dialog should still be open");
    }
}

#[test]
fn scoped_models_dialog_navigates_down() {
    let mut state = AppState::default();
    state.config.scoped_models = vec![
        sm("mock", "echo", true),
        sm("openai", "gpt-4o", true),
        sm("anthropic", "claude-3", true),
    ];
    state.open_dialog = Some(DialogState::ScopedModels { selected: 2 });

    state.update(Event::HistoryNext);

    if let Some(DialogState::ScopedModels { selected }) = state.open_dialog {
        assert_eq!(selected, 0, "Down at last should wrap to first");
    } else {
        panic!("Dialog should still be open");
    }
}

#[test]
fn scoped_models_dialog_space_toggles() {
    let mut state = AppState::default();
    state.config.scoped_models = vec![
        sm("mock", "echo", true),
        sm("openai", "gpt-4o", true),
    ];
    state.open_dialog = Some(DialogState::ScopedModels { selected: 1 });

    state.update(Event::Input(' '));

    assert!(!state.config.scoped_models[1].enabled);
    assert!(matches!(state.open_dialog, Some(DialogState::ScopedModels { .. })));
}

#[test]
fn scoped_models_dialog_a_enables_all() {
    let mut state = AppState::default();
    state.config.scoped_models = vec![
        sm("mock", "echo", false),
        sm("openai", "gpt-4o", false),
    ];
    state.open_dialog = Some(DialogState::ScopedModels { selected: 0 });

    state.update(Event::Input('a'));

    assert!(state.config.scoped_models.iter().all(|m| m.enabled));
}

#[test]
fn scoped_models_dialog_x_disables_all() {
    let mut state = AppState::default();
    state.config.scoped_models = vec![
        sm("mock", "echo", true),
        sm("openai", "gpt-4o", true),
    ];
    state.open_dialog = Some(DialogState::ScopedModels { selected: 0 });

    state.update(Event::Input('x'));

    assert!(state.config.scoped_models.iter().all(|m| !m.enabled));
}

#[test]
fn scoped_models_dialog_p_toggles_provider() {
    let mut state = AppState::default();
    state.config.scoped_models = vec![
        sm("openai", "gpt-4o", true),
        sm("openai", "gpt-4o-mini", true),
        sm("anthropic", "claude-3", true),
    ];
    state.open_dialog = Some(DialogState::ScopedModels { selected: 0 });

    state.update(Event::Input('p'));

    assert!(!state.config.scoped_models[0].enabled);
    assert!(!state.config.scoped_models[1].enabled);
    assert!(state.config.scoped_models[2].enabled);
}

#[test]
fn scoped_models_dialog_esc_closes() {
    let mut state = AppState::default();
    state.config.scoped_models = vec![sm("mock", "echo", true)];
    state.open_dialog = Some(DialogState::ScopedModels { selected: 0 });

    state.update(Event::Abort);

    assert!(state.open_dialog.is_none());
}
