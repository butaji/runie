//! Model selector tests (Layer 1 + Layer 2)

use crate::config::ModelProvider;

use crate::commands::{DialogKind, DialogState};
use crate::model::{AppState, ScopedModel};
use crate::model_catalog::{build_model_selector_items, filter_models, model_catalog, ModelInfo};

fn selector_state(state: &AppState) -> Option<(String, usize)> {
    match &state.open_dialog {
        Some(DialogState::Active {
            kind: DialogKind::ModelSelector,
            panels: stack,
        }) => stack.current().map(|p| (p.filter.clone(), p.selected)),
        _ => None,
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

fn sample_catalog() -> Vec<ModelInfo> {
    vec![
        ModelInfo::new("anthropic", "claude-sonnet").with_cost(3.0, 15.0),
        ModelInfo::new("openai", "gpt-4o").with_cost(5.0, 15.0),
        ModelInfo::new("openai", "gpt-4o-mini").with_cost(0.15, 0.6),
        ModelInfo::new("google", "gemini-pro").with_cost(1.0, 4.0),
    ]
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

/// Reset config by clearing model_providers.
fn reset_config(state: &mut AppState) {
    state.config_mut().model_providers_mut().clear();
}

// === Layer 1: State/Logic ===

#[test]
fn filter_matches_name() {
    let models = sample_catalog();
    let indices = filter_models(&models, "gpt");
    assert_eq!(indices.len(), 2);
    assert!(indices.iter().any(|&i| models[i].name == "gpt-4o"));
    assert!(indices.iter().any(|&i| models[i].name == "gpt-4o-mini"));
}

#[test]
fn filter_matches_provider() {
    let models = sample_catalog();
    let indices = filter_models(&models, "anthropic");
    assert_eq!(indices.len(), 1);
    assert_eq!(models[indices[0]].provider, "anthropic");
}

#[test]
fn filter_case_insensitive() {
    let models = sample_catalog();
    let lower = filter_models(&models, "gpt");
    let upper = filter_models(&models, "GPT");
    assert_eq!(lower.len(), upper.len());
    assert!(upper.iter().any(|&i| models[i].name == "gpt-4o"));
}

#[test]
fn filter_empty_shows_all() {
    let models = sample_catalog();
    let indices = filter_models(&models, "");
    assert_eq!(indices.len(), models.len());
}

#[test]
fn recent_shows_max_5() {
    let models = sample_catalog();
    let recent: Vec<String> = (0..10).map(|i| format!("openai/gpt-{}", i)).collect();
    let items = build_model_selector_items(&models, &recent, "", "mock", "echo");
    let recent_count = items.iter().filter(|(h, _, _, _, _)| h == "Recent").count();
    assert!(
        recent_count <= 5,
        "Recent section should cap at 5, got {}",
        recent_count
    );
}

#[test]
fn recent_only_shows_known_models() {
    let models = sample_catalog();
    let recent = vec!["openai/gpt-4o".to_string(), "unknown/model".to_string()];
    let items = build_model_selector_items(&models, &recent, "", "mock", "echo");
    assert!(items
        .iter()
        .any(|(_, name, _, _, _)| name == "openai/gpt-4o"));
    assert!(!items
        .iter()
        .any(|(_, name, _, _, _)| name == "unknown/model"));
}

#[test]
fn select_emits_switch_model() {
    let mut state = AppState::default();
    configure(&mut state, &[("openai".into(), vec!["gpt-4o".into()])]);
    state.update(crate::Event::ToggleModelSelector);
    assert!(selector_state(&state).is_some());
    state.update(crate::Event::ModelSelectorSelect);
    assert!(state.open_dialog.is_none());
    assert_eq!(state.config.current_provider, "openai");
    assert_eq!(state.config.current_model, "gpt-4o");
}

#[test]
fn current_model_marked_with_star() {
    let models = sample_catalog();
    let items = build_model_selector_items(&models, &[], "", "openai", "gpt-4o");
    let current = items.iter().find(|(_, _, _, _, is_current)| *is_current);
    assert!(current.is_some(), "Current model should be marked");
    assert_eq!(current.unwrap().1, "openai/gpt-4o");
}

#[test]
fn record_model_usage_tracks_recent() {
    let mut state = AppState::default();
    state.record_model_usage("openai", "gpt-4o");
    assert_eq!(state.config.recent_models.len(), 1);
    assert_eq!(state.config.recent_models[0], "openai/gpt-4o");
}

#[test]
fn record_model_usage_dedupes() {
    let mut state = AppState::default();
    state.record_model_usage("openai", "gpt-4o");
    state.record_model_usage("anthropic", "claude");
    state.record_model_usage("openai", "gpt-4o");
    assert_eq!(state.config.recent_models.len(), 2);
    assert_eq!(state.config.recent_models[1], "openai/gpt-4o");
}

#[test]
fn record_model_usage_caps_at_5() {
    let mut state = AppState::default();
    for i in 0..10 {
        state.record_model_usage("provider", &format!("model-{}", i));
    }
    assert_eq!(state.config.recent_models.len(), 5);
}

#[test]
fn groups_by_provider() {
    let models = sample_catalog();
    let items = build_model_selector_items(&models, &[], "", "mock", "echo");
    let headers: Vec<_> = items
        .iter()
        .filter(|(h, _, _, _, _)| !h.is_empty())
        .map(|(h, _, _, _, _)| h.clone())
        .collect();
    assert!(headers.contains(&"anthropic".to_string()));
    assert!(headers.contains(&"openai".to_string()));
    assert!(headers.contains(&"google".to_string()));
}

// === Layer 2: Event Handling ===

#[test]
fn ctrl_l_opens_selector() {
    let mut state = AppState::default();
    assert!(state.open_dialog.is_none());
    state.update(crate::Event::ToggleModelSelector);
    assert!(selector_state(&state).is_some());
}

#[test]
fn slash_model_no_args_opens_selector() {
    let mut state = AppState::default();
    configure(&mut state, &[("openai".into(), vec!["gpt-4o".into()])]);
    palette_select(&mut state, "model");
    assert!(selector_state(&state).is_some());
}

#[test]
fn esc_closes_selector() {
    let mut state = AppState::default();
    state.update(crate::Event::ToggleModelSelector);
    assert!(state.open_dialog.is_some());
    state.update(crate::Event::ModelSelectorClose);
    assert!(state.open_dialog.is_none());
}

#[test]
fn empty_current_marker_when_no_active_model() {
    let mut state = AppState::default();
    reset_config(&mut state);
    state.config.current_provider.clear();
    state.config.current_model.clear();
    state.update(crate::Event::ToggleModelSelector);

    let items = match &state.open_dialog {
        Some(DialogState::Active {
            kind: DialogKind::ModelSelector,
            panels: stack,
        }) => stack.current().map(|p| p.items.clone()).unwrap_or_default(),
        _ => Vec::new(),
    };
    assert!(
        !items
            .iter()
            .any(|i| i.label().map(|l| l.starts_with('★')).unwrap_or(false)),
        "no item should be starred when there is no active model"
    );
}

#[test]
fn abort_closes_selector() {
    let mut state = AppState::default();
    state.update(crate::Event::ToggleModelSelector);
    assert!(state.open_dialog.is_some());
    state.update(crate::Event::Abort);
    assert!(state.open_dialog.is_none());
}

#[test]
fn filter_narrows_selector() {
    let mut state = AppState::default();
    state.update(crate::Event::ToggleModelSelector);
    state.update(crate::Event::ModelSelectorFilter('g'));
    state.update(crate::Event::ModelSelectorFilter('p'));
    state.update(crate::Event::ModelSelectorFilter('t'));
    let (filter, _) = selector_state(&state).expect("ModelSelector should be open");
    assert_eq!(filter, "gpt");
}

#[test]
fn up_down_navigates_selector() {
    let mut state = AppState::default();
    configure(
        &mut state,
        &[("openai".into(), vec!["gpt-4o".into(), "gpt-4o-mini".into()])],
    );
    state.update(crate::Event::ToggleModelSelector);
    state.update(crate::Event::ModelSelectorDown);
    let (_, selected) = selector_state(&state).expect("ModelSelector should be open");
    assert_eq!(selected, 1);
    state.update(crate::Event::ModelSelectorUp);
    let (_, selected) = selector_state(&state).expect("ModelSelector should be open");
    assert_eq!(selected, 0);
}

#[test]
fn selector_wraps_up() {
    let mut state = AppState::default();
    configure(
        &mut state,
        &[("openai".into(), vec!["gpt-4o".into(), "gpt-4o-mini".into()])],
    );
    state.update(crate::Event::ToggleModelSelector);
    state.update(crate::Event::ModelSelectorUp);
    let (_, selected) = selector_state(&state).expect("ModelSelector should be open");
    assert!(
        selected > 0,
        "Up at first should wrap to last (got {})",
        selected
    );
}

#[test]
fn selector_wraps_down() {
    let mut state = AppState::default();
    configure(
        &mut state,
        &[("openai".into(), vec!["gpt-4o".into(), "gpt-4o-mini".into()])],
    );
    state.update(crate::Event::ToggleModelSelector);
    let count = match &state.open_dialog {
        Some(DialogState::Active {
            kind: DialogKind::ModelSelector,
            panels: stack,
        }) => stack.current().map(|p| p.navigable_count()).unwrap_or(0),
        _ => 0,
    };
    for _ in 0..count {
        state.update(crate::Event::ModelSelectorDown);
    }
    let (_, selected) = selector_state(&state).expect("ModelSelector should be open");
    assert_eq!(selected, 0, "Down at last should wrap to first");
}

#[test]
fn cycle_model_next_uses_unified_catalog() {
    let catalog = model_catalog();
    assert!(
        catalog.len() >= 2,
        "unified catalog must have at least two models"
    );

    let mut state = AppState::default();
    state.config.scoped_models = catalog
        .iter()
        .take(10)
        .map(|m| ScopedModel {
            provider: m.provider.clone(),
            name: m.name.clone(),
            enabled: true,
        })
        .collect();
    state.config.scoped_index = 0;
    let first = &state.config.scoped_models[0];
    state.config.current_provider = first.provider.clone();
    state.config.current_model = first.name.clone();

    state.cycle_model(1);

    let second = &state.config.scoped_models[1];
    assert_eq!(state.config.current_provider, second.provider);
    assert_eq!(state.config.current_model, second.name);
}

#[test]
fn model_selector_renders_grouped_models() {
    let catalog = model_catalog();
    let items = build_model_selector_items(&catalog, &[], "", "mock", "echo");
    let providers: std::collections::HashSet<String> = items
        .iter()
        .filter(|(header, _, _, _, _)| !header.is_empty() && header != "Recent")
        .map(|(header, _, _, _, _)| header.clone())
        .collect();

    assert!(
        providers.contains("openai"),
        "selector should render an openai group"
    );
    assert!(
        providers.contains("anthropic"),
        "selector should render an anthropic group"
    );

    // Every item belongs to a provider present in the catalog.
    for (_, full, _, _, _) in &items {
        if full == "Recent" {
            continue;
        }
        let (provider, name) = full
            .split_once('/')
            .expect("model name should be provider/name");
        assert!(
            catalog
                .iter()
                .any(|m| m.provider == provider && m.name == name),
            "rendered model {full} must exist in unified catalog"
        );
    }
}
