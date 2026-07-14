//! Per-model thinking (reasoning) level tests.
//!
//! Contract: `/model` model selection is a two-step flow — picking a model
//! opens a reasoning-level panel for that model; the chosen level is stored
//! per model (`[models.thinking]` in config.toml) and overrides the global
//! thinking level whenever that model is active. Choosing "default" removes
//! the override so the model inherits the global level again.

use crate::commands::{DialogKind, DialogState};
use crate::config::ModelProvider;
use crate::model::{AppState, ThinkingLevel};

fn configure(state: &mut AppState, providers: &[(String, Vec<String>)]) {
    for (name, models) in providers {
        state.config_mut().model_providers_mut().insert(
            name.clone(),
            ModelProvider {
                provider_type: None,
                base_url: String::new(),
                models: models.clone(),
                headers: std::collections::HashMap::new(),
            },
        );
    }
}

fn current_panel(state: &AppState) -> Option<&crate::dialog::Panel> {
    match &state.open_dialog {
        Some(DialogState::Active {
            kind: DialogKind::ModelSelector,
            panels: stack,
        }) => stack.current(),
        _ => None,
    }
}

fn panel_labels(state: &AppState) -> Vec<String> {
    current_panel(state)
        .map(|p| {
            p.items
                .iter()
                .filter_map(|i| i.label().map(str::to_owned))
                .collect()
        })
        .unwrap_or_default()
}

/// Selecting a model opens a reasoning-level panel instead of switching
/// straight away.
#[test]
fn select_model_opens_reasoning_panel() {
    let mut state = AppState::default();
    configure(&mut state, &[("openai".into(), vec!["gpt-4o".into()])]);

    state.update(crate::Event::ToggleModelSelector);
    state.update(crate::Event::ModelSelectorSelect);

    let panel = current_panel(&state).expect("reasoning panel should open");
    assert!(
        panel.title.contains("openai/gpt-4o"),
        "reasoning panel title should name the chosen model, got {:?}",
        panel.title
    );
    let labels = panel_labels(&state);
    assert!(
        labels.iter().any(|l| l.contains("default")),
        "panel should offer a default (inherit global) row: {labels:?}"
    );
    for level in ["off", "low", "medium", "high"] {
        assert!(
            labels.iter().any(|l| l.contains(level)),
            "panel should offer the {level} level: {labels:?}"
        );
    }

    // The model has not switched yet — that happens when a level is picked.
    assert_ne!(state.config.current_model, "gpt-4o");
}

/// Picking a level switches the model and records the per-model override.
#[test]
fn reasoning_panel_pick_switches_model_and_sets_override() {
    let mut state = AppState::default();
    configure(&mut state, &[("openai".into(), vec!["gpt-4o".into()])]);

    state.update(crate::Event::ToggleModelSelector);
    state.update(crate::Event::ModelSelectorSelect);
    assert!(current_panel(&state).is_some());

    // Rows: default, off, low, medium, high → "high" is the 5th row.
    for _ in 0..4 {
        state.update(crate::Event::ModelSelectorDown);
    }
    state.update(crate::Event::ModelSelectorSelect);

    assert!(
        state.open_dialog.is_none(),
        "dialog should close after pick"
    );
    assert_eq!(state.config.current_provider, "openai");
    assert_eq!(state.config.current_model, "gpt-4o");
    assert_eq!(
        state.config.model_thinking.get("openai/gpt-4o"),
        Some(&ThinkingLevel::High),
        "per-model override must be recorded"
    );
}

/// The effective thinking level prefers the per-model override and falls
/// back to the global level for models without an override.
#[test]
fn effective_thinking_level_prefers_model_override() {
    let mut state = AppState::default();
    state.config_mut().current_provider = "openai".into();
    state.config_mut().current_model = "gpt-4o".into();
    state.config_mut().thinking_level = ThinkingLevel::Low;

    assert_eq!(state.effective_thinking_level(), ThinkingLevel::Low);

    state.set_model_thinking_level("openai", "gpt-4o", Some(ThinkingLevel::High));
    assert_eq!(state.effective_thinking_level(), ThinkingLevel::High);

    // A different model still inherits the global level.
    state.config_mut().current_model = "gpt-4o-mini".into();
    assert_eq!(state.effective_thinking_level(), ThinkingLevel::Low);
}

/// The "default" row removes the override: the model inherits the global
/// level again and the map entry is gone.
#[test]
fn default_row_clears_override() {
    let mut state = AppState::default();
    configure(&mut state, &[("openai".into(), vec!["gpt-4o".into()])]);
    state.config_mut().thinking_level = ThinkingLevel::Medium;
    state.set_model_thinking_level("openai", "gpt-4o", Some(ThinkingLevel::High));

    state.update(crate::Event::ToggleModelSelector);
    state.update(crate::Event::ModelSelectorSelect);
    // The override row (high) is pre-selected; move to the "default" row
    // (Down wraps from the last row to the first) and activate it.
    state.update(crate::Event::ModelSelectorDown);
    state.update(crate::Event::ModelSelectorSelect);

    assert!(state.open_dialog.is_none());
    assert_eq!(state.config.current_model, "gpt-4o");
    assert!(
        !state.config.model_thinking.contains_key("openai/gpt-4o"),
        "default row must remove the per-model override"
    );
    assert_eq!(state.effective_thinking_level(), ThinkingLevel::Medium);
}

/// The reasoning panel marks the effective choice as current: the override
/// when set, otherwise the global level on the "default" row.
#[test]
fn reasoning_panel_marks_effective_choice() {
    let mut state = AppState::default();
    configure(&mut state, &[("openai".into(), vec!["gpt-4o".into()])]);
    state.set_model_thinking_level("openai", "gpt-4o", Some(ThinkingLevel::High));

    state.update(crate::Event::ToggleModelSelector);
    state.update(crate::Event::ModelSelectorSelect);

    let labels = panel_labels(&state);
    let high = labels
        .iter()
        .find(|l| l.contains("high"))
        .expect("high row");
    assert!(
        high.contains("current") || high.contains('★'),
        "override level should be marked current, got {high:?}"
    );
}

/// Model rows in the selector show the chosen per-model level as a suffix.
#[test]
fn selector_labels_show_override_suffix() {
    let mut state = AppState::default();
    configure(&mut state, &[("openai".into(), vec!["gpt-4o".into()])]);
    state.set_model_thinking_level("openai", "gpt-4o", Some(ThinkingLevel::High));

    state.update(crate::Event::ToggleModelSelector);

    let labels = panel_labels(&state);
    let row = labels
        .iter()
        .find(|l| l.contains("openai/gpt-4o"))
        .expect("model row should be listed");
    assert!(
        row.contains("high"),
        "model row should show the per-model level, got {row:?}"
    );
}

/// The status-bar snapshot carries the effective (per-model) thinking level.
#[test]
fn snapshot_uses_effective_thinking_level() {
    let mut state = AppState::default();
    state.config_mut().current_provider = "openai".into();
    state.config_mut().current_model = "gpt-4o".into();
    state.config_mut().thinking_level = ThinkingLevel::Low;
    state.set_model_thinking_level("openai", "gpt-4o", Some(ThinkingLevel::High));

    assert_eq!(state.snapshot().thinking_level, ThinkingLevel::High);
}

/// ConfigLoaded populates the per-model thinking map from `[models.thinking]`.
#[test]
fn config_loaded_populates_model_thinking() {
    let mut state = AppState::default();
    let mut config = crate::config::Config::default();
    config
        .models
        .thinking
        .insert("openai/gpt-4o".to_string(), ThinkingLevel::High);

    state.update(crate::Event::ConfigLoaded {
        config: Box::new(config),
    });

    assert_eq!(
        state.config.model_thinking.get("openai/gpt-4o"),
        Some(&ThinkingLevel::High)
    );
}
