//! Scoped models dialog update logic

use crate::model::AppState;
use crate::Event;
use crate::commands::DialogState;

pub fn update_scoped_models(state: &mut AppState, event: Event, selected: usize) {
    match event {
        Event::Abort | Event::ToggleScopedModelsDialog => {
            state.open_dialog = None;
            state.mark_dirty();
        }
        Event::HistoryPrev | Event::PaletteUp => {
            let new_sel = if selected == 0 {
                state.config.scoped_models.len().saturating_sub(1)
            } else {
                selected - 1
            };
            state.open_dialog = Some(DialogState::ScopedModels { selected: new_sel });
            state.mark_dirty();
        }
        Event::HistoryNext | Event::PaletteDown => {
            let new_sel = if state.config.scoped_models.is_empty() {
                0
            } else {
                (selected + 1) % state.config.scoped_models.len()
            };
            state.open_dialog = Some(DialogState::ScopedModels { selected: new_sel });
            state.mark_dirty();
        }
        Event::Submit | Event::PaletteSelect => {
            if let Some(model) = state.config.scoped_models.get(selected) {
                let name = model.name.clone();
                toggle_scoped_model(state, &name);
            }
            state.open_dialog = Some(DialogState::ScopedModels { selected });
            state.mark_dirty();
        }
        Event::Input(' ') => {
            if let Some(model) = state.config.scoped_models.get(selected) {
                let name = model.name.clone();
                toggle_scoped_model(state, &name);
            }
            state.open_dialog = Some(DialogState::ScopedModels { selected });
            state.mark_dirty();
        }
        Event::Input('a') | Event::Input('A') => {
            enable_all(state);
            state.open_dialog = Some(DialogState::ScopedModels { selected });
            state.mark_dirty();
        }
        Event::Input('x') | Event::Input('X') => {
            disable_all(state);
            state.open_dialog = Some(DialogState::ScopedModels { selected });
            state.mark_dirty();
        }
        Event::Input('p') | Event::Input('P') => {
            if let Some(model) = state.config.scoped_models.get(selected) {
                let provider = model.provider.clone();
                toggle_provider(state, &provider);
            }
            state.open_dialog = Some(DialogState::ScopedModels { selected });
            state.mark_dirty();
        }
        _ => {
            state.open_dialog = Some(DialogState::ScopedModels { selected });
        }
    }
}

pub fn toggle_scoped_model(state: &mut AppState, name: &str) {
    if let Some(idx) = state.config.scoped_models.iter().position(|m| m.name == name) {
        state.config.scoped_models[idx].enabled = !state.config.scoped_models[idx].enabled;
        state.mark_dirty();
    }
}

pub fn enable_all(state: &mut AppState) {
    for m in &mut state.config.scoped_models {
        m.enabled = true;
    }
    state.mark_dirty();
}

pub fn disable_all(state: &mut AppState) {
    for m in &mut state.config.scoped_models {
        m.enabled = false;
    }
    state.mark_dirty();
}

pub fn toggle_provider(state: &mut AppState, provider: &str) {
    let all_enabled = state
        .config.scoped_models
        .iter()
        .filter(|m| m.provider == provider)
        .all(|m| m.enabled);
    for m in &mut state.config.scoped_models {
        if m.provider == provider {
            m.enabled = !all_enabled;
        }
    }
    state.mark_dirty();
}
