//! Model selector tests (Layer 1 + Layer 2)

use crate::commands::DialogState;
use crate::event::Event;
use crate::model::AppState;
use crate::model_catalog::{ModelInfo, filter_models, build_model_selector_items, model_catalog};

fn sample_catalog() -> Vec<ModelInfo> {
    vec![
        ModelInfo::new("anthropic", "claude-sonnet").with_cost(3.0, 15.0),
        ModelInfo::new("openai", "gpt-4o").with_cost(5.0, 15.0),
        ModelInfo::new("openai", "gpt-4o-mini").with_cost(0.15, 0.6),
        ModelInfo::new("google", "gemini-pro").with_cost(1.0, 4.0),
    ]
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
    let recent: Vec<String> = (0..10)
        .map(|i| format!("openai/gpt-{}", i))
        .collect();
    // Only the first 4 exist in catalog, but recent cap is 5
    let items = build_model_selector_items(&models, &recent, "", "mock", "echo");
    let recent_count = items.iter().filter(|(h, _, _, _, _)| h == "Recent").count();
    assert!(recent_count <= 5, "Recent section should cap at 5, got {}", recent_count);
}

#[test]
fn recent_only_shows_known_models() {
    let models = sample_catalog();
    let recent = vec!["openai/gpt-4o".to_string(), "unknown/model".to_string()];
    let items = build_model_selector_items(&models, &recent, "", "mock", "echo");
    assert!(items.iter().any(|(_, name, _, _, _)| name == "openai/gpt-4o"));
    assert!(!items.iter().any(|(_, name, _, _, _)| name == "unknown/model"));
}

#[test]
fn select_emits_switch_model() {
    let mut state = AppState::default();
    state.update(Event::ToggleModelSelector);
    assert!(matches!(state.open_dialog, Some(DialogState::ModelSelector { .. })));
    // Select first item (should be a model from catalog)
    state.update(Event::ModelSelectorSelect);
    assert!(state.open_dialog.is_none());
    // Model should have switched from default mock/echo
    let switched = state.config.current_provider != "mock" || state.config.current_model != "echo";
    assert!(switched, "Should switch model on select");
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
    assert_eq!(state.recent_models.len(), 1);
    assert_eq!(state.recent_models[0], "openai/gpt-4o");
}

#[test]
fn record_model_usage_dedupes() {
    let mut state = AppState::default();
    state.record_model_usage("openai", "gpt-4o");
    state.record_model_usage("anthropic", "claude");
    state.record_model_usage("openai", "gpt-4o");
    assert_eq!(state.recent_models.len(), 2);
    assert_eq!(state.recent_models[1], "openai/gpt-4o");
}

#[test]
fn record_model_usage_caps_at_5() {
    let mut state = AppState::default();
    for i in 0..10 {
        state.record_model_usage("provider", &format!("model-{}", i));
    }
    assert_eq!(state.recent_models.len(), 5);
}

#[test]
fn groups_by_provider() {
    let models = sample_catalog();
    let items = build_model_selector_items(&models, &[], "", "mock", "echo");
    let headers: Vec<_> = items.iter()
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
    state.update(Event::ToggleModelSelector);
    assert!(matches!(state.open_dialog, Some(DialogState::ModelSelector { .. })));
}

#[test]
fn slash_model_no_args_opens_selector() {
    let mut state = AppState::default();
    for c in "/model".chars() {
        state.update(Event::Input(c));
    }
    state.update(Event::Submit);
    assert!(matches!(state.open_dialog, Some(DialogState::ModelSelector { .. })));
}

#[test]
fn esc_closes_selector() {
    let mut state = AppState::default();
    state.update(Event::ToggleModelSelector);
    assert!(state.open_dialog.is_some());
    state.update(Event::ModelSelectorClose);
    assert!(state.open_dialog.is_none());
}

#[test]
fn abort_closes_selector() {
    let mut state = AppState::default();
    state.update(Event::ToggleModelSelector);
    assert!(state.open_dialog.is_some());
    state.update(Event::Abort);
    assert!(state.open_dialog.is_none());
}

#[test]
fn filter_narrows_selector() {
    let mut state = AppState::default();
    state.update(Event::ToggleModelSelector);
    state.update(Event::ModelSelectorFilter('g'));
    state.update(Event::ModelSelectorFilter('p'));
    state.update(Event::ModelSelectorFilter('t'));
    if let Some(DialogState::ModelSelector { filter, .. }) = &state.open_dialog {
        assert_eq!(filter, "gpt");
    } else {
        panic!("ModelSelector should be open");
    }
}

#[test]
fn up_down_navigates_selector() {
    let mut state = AppState::default();
    state.update(Event::ToggleModelSelector);
    state.update(Event::ModelSelectorDown);
    if let Some(DialogState::ModelSelector { selected, .. }) = &state.open_dialog {
        assert_eq!(*selected, 1);
    } else {
        panic!("ModelSelector should be open");
    }
    state.update(Event::ModelSelectorUp);
    if let Some(DialogState::ModelSelector { selected, .. }) = &state.open_dialog {
        assert_eq!(*selected, 0);
    } else {
        panic!("ModelSelector should be open");
    }
}

#[test]
fn selector_wraps_up() {
    let mut state = AppState::default();
    state.update(Event::ToggleModelSelector);
    state.update(Event::ModelSelectorUp);
    let catalog = model_catalog();
    let count = build_model_selector_items(&catalog, &[], "", "mock", "echo").len();
    if let Some(DialogState::ModelSelector { selected, .. }) = &state.open_dialog {
        assert_eq!(*selected, count - 1, "Up at first should wrap to last");
    } else {
        panic!("ModelSelector should be open");
    }
}

#[test]
fn selector_wraps_down() {
    let mut state = AppState::default();
    state.update(Event::ToggleModelSelector);
    let catalog = model_catalog();
    let count = build_model_selector_items(&catalog, &[], "", "mock", "echo").len();
    for _ in 0..count {
        state.update(Event::ModelSelectorDown);
    }
    if let Some(DialogState::ModelSelector { selected, .. }) = &state.open_dialog {
        assert_eq!(*selected, 0, "Down at last should wrap to first");
    } else {
        panic!("ModelSelector should be open");
    }
}
