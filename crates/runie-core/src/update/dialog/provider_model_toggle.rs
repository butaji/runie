//! Provider model toggling inside the settings dialog.
//!
//! Settings exposes each configured/known model for the current provider as a
//! checkbox. Toggling updates `[model_providers.<provider>].models` directly.

use crate::model::AppState;

/// Parse a toggle key produced by the settings dialog for a provider/model.
pub fn parse_provider_model_toggle(key: &str) -> Option<(&str, &str)> {
    let rest = key.strip_prefix("edit_provider:edit_provider:")?;
    rest.split_once(':')
}

/// Toggle whether `model` is enabled for `provider` in the saved config.
pub fn toggle_provider_model(state: &mut AppState, provider: &str, model: &str) {
    let mut config = crate::async_io::block_in_place_if_runtime(|| crate::config::Config::load(None));
    let Some(entry) = config.model_providers.get_mut(provider) else {
        return;
    };
    let pos = entry.models.iter().position(|m| m == model);
    if let Some(idx) = pos {
        entry.models.remove(idx);
    } else {
        entry.models.push(model.into());
        entry.models.sort();
    }
    let models = entry.models.clone();
    let _ = crate::async_io::block_in_place_if_runtime(|| config.save());
    if provider == state.config.current_provider && !models.contains(&model.to_string()) {
        if let Some(first) = models.first() {
            state.switch_model(provider.into(), first.clone(), false);
        }
    }
    state.view.cached_settings_valid = false;
}
