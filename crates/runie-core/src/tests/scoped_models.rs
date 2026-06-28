//! Scoped models tests (Layer 1 + Layer 2)

use crate::commands::{DialogKind, DialogState};
use crate::config::ModelProvider;
use crate::model::{AppState, ScopedModel};
use crate::Event;

fn sm(provider: &str, name: &str, enabled: bool) -> ScopedModel {
    ScopedModel {
        provider: provider.into(),
        name: name.into(),
        enabled,
    }
}

/// Seed providers directly into state.config.model_providers.
fn configure(state: &mut AppState, providers: &[(String, Vec<String>)]) {
    for (name, models) in providers {
        state.config_mut().model_providers_mut().insert(
            name.clone(),
            ModelProvider {
                provider_type: None,
                base_url: String::new(),
                api_key: String::new(),
                models: models.clone(),
            },
        );
    }
}

/// Open palette and select a command by name
fn palette_select(state: &mut AppState, cmd: &str) {
    state.update(crate::Event::Input('/'));
    for c in cmd.chars() {
        state.update(crate::Event::PaletteFilter(c));
    }
    state.update(crate::Event::PaletteSelect);
}

#[test]
fn toggle_model_excludes_from_cycle() {
    let mut state = AppState::default();
    state.config.scoped_models = vec![sm("mock", "echo", true), sm("openai", "gpt-4o", true)];

    state.update(crate::Event::ScopedModelToggle {
        provider: "openai".to_string(),
        name: "gpt-4o".to_string(),
    });

    assert!(!state.config.scoped_models[1].enabled);
}

#[test]
fn enable_all_includes_all() {
    let mut state = AppState::default();
    state.config.scoped_models = vec![sm("mock", "echo", false), sm("openai", "gpt-4o", false)];

    state.update(crate::Event::ScopedModelEnableAll);

    assert!(state.config.scoped_models.iter().all(|m| m.enabled));
}

#[test]
fn disable_all_excludes_all() {
    let mut state = AppState::default();
    state.config.scoped_models = vec![sm("mock", "echo", true), sm("openai", "gpt-4o", true)];

    state.update(crate::Event::ScopedModelDisableAll);

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

    state.update(crate::Event::ScopedModelToggleProvider {
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

    state.update(crate::Event::ScopedModelToggleProvider {
        provider: "openai".to_string(),
    });

    assert!(state.config.scoped_models[0].enabled);
    assert!(state.config.scoped_models[1].enabled);
    assert!(state.config.scoped_models[2].enabled);
}

fn scoped_selected(state: &AppState) -> Option<usize> {
    match &state.open_dialog {
        Some(DialogState::Active { kind: DialogKind::ScopedModels, panels: stack }) => stack.current().map(|p| p.selected),
        _ => None,
    }
}

#[test]
fn slash_scoped_models_opens_dialog() {
    let mut state = AppState::default();
    configure(
        &mut state,
        &[
            ("mock".into(), vec!["echo".into()]),
            ("openai".into(), vec!["gpt-4o".into()]),
        ],
    );

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
    configure(
        &mut state,
        &[
            ("mock".into(), vec!["echo".into()]),
            ("openai".into(), vec!["gpt-4o".into()]),
            ("anthropic".into(), vec!["claude-3".into()]),
        ],
    );
    state.update(crate::Event::ToggleScopedModelsDialog);

    state.update(crate::Event::HistoryPrev);

    assert_eq!(
        scoped_selected(&state).unwrap(),
        2,
        "Up at first should wrap to last"
    );
}

#[test]
fn scoped_models_dialog_navigates_down() {
    let mut state = AppState::default();
    configure(
        &mut state,
        &[
            ("mock".into(), vec!["echo".into()]),
            ("openai".into(), vec!["gpt-4o".into()]),
            ("anthropic".into(), vec!["claude-3".into()]),
        ],
    );
    state.update(crate::Event::ToggleScopedModelsDialog);
    state.update(crate::Event::HistoryNext);
    state.update(crate::Event::HistoryNext);
    state.update(crate::Event::HistoryNext);

    assert_eq!(
        scoped_selected(&state).unwrap(),
        0,
        "Down at last should wrap to first"
    );
}

#[test]
fn scoped_models_dialog_submit_toggles() {
    let mut state = AppState::default();
    configure(
        &mut state,
        &[
            ("mock".into(), vec!["echo".into()]),
            ("openai".into(), vec!["gpt-4o".into()]),
        ],
    );
    state.update(crate::Event::ToggleScopedModelsDialog);
    state.update(crate::Event::HistoryNext);

    state.update(Event::submit());

    assert!(!state.config.scoped_models[1].enabled);
    assert!(scoped_selected(&state).is_some());
}

#[test]
fn scoped_models_dialog_esc_closes() {
    let mut state = AppState::default();
    configure(&mut state, &[("mock".into(), vec!["echo".into()])]);
    state.update(crate::Event::ToggleScopedModelsDialog);

    state.update(crate::Event::Abort);

    assert!(state.open_dialog.is_none());
}

#[test]
fn toggle_scoped_model_uses_provider_and_name() {
    let mut state = AppState::default();
    state.config.scoped_models = vec![
        sm("openai", "gpt-4o", true),
        sm("anthropic", "gpt-4o", true),
    ];

    state.update(crate::Event::ScopedModelToggle {
        provider: "anthropic".into(),
        name: "gpt-4o".into(),
    });

    assert!(
        state.config.scoped_models[0].enabled,
        "openai model stayed on"
    );
    assert!(
        !state.config.scoped_models[1].enabled,
        "anthropic model was toggled off"
    );
}

#[test]
fn scoped_models_dialog_populates_from_configured_providers() {
    let mut state = AppState::default();
    configure(
        &mut state,
        &[(
            "minimax".into(),
            vec!["MiniMax-M3".into(), "MiniMax-M2.7".into()],
        )],
    );
    state.config.scoped_models.clear();

    state.update(crate::Event::ToggleScopedModelsDialog);

    let items = match &state.open_dialog {
        Some(DialogState::Active { kind: DialogKind::ScopedModels, panels: stack }) => {
            stack.current().map(|p| p.items.clone()).unwrap_or_default()
        }
        _ => Vec::new(),
    };
    let labels: Vec<_> = items.iter().filter_map(|i| i.label()).collect();
    assert!(
        labels.iter().any(|l| l.contains("MiniMax-M3")),
        "expected MiniMax-M3 in {:?}",
        labels
    );
    assert!(
        labels.iter().any(|l| l.contains("MiniMax-M2.7")),
        "expected MiniMax-M2.7 in {:?}",
        labels
    );
}
