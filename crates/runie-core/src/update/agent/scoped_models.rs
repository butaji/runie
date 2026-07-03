use crate::model::AppState;

pub fn toggle_scoped_model(state: &mut AppState, provider: &str, name: &str) {
    if let Some(idx) = state
        .config()
        .scoped_models
        .iter()
        .position(|m| m.provider == provider && m.name == name)
    {
        state.config_mut().scoped_models[idx].enabled = !state.config().scoped_models[idx].enabled;
        state.view_mut().dirty = true;
    }
}

pub fn enable_all(state: &mut AppState) {
    for m in &mut state.config_mut().scoped_models {
        m.enabled = true;
    }
    state.view_mut().dirty = true;
}

pub fn disable_all(state: &mut AppState) {
    for m in &mut state.config_mut().scoped_models {
        m.enabled = false;
    }
    state.view_mut().dirty = true;
}

pub fn toggle_provider(state: &mut AppState, provider: &str) {
    let all_enabled = state
        .config()
        .scoped_models
        .iter()
        .filter(|m| m.provider == provider)
        .all(|m| m.enabled);
    for m in &mut state.config_mut().scoped_models {
        if m.provider == provider {
            m.enabled = !all_enabled;
        }
    }
    state.view_mut().dirty = true;
}
