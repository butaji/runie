//! Provider model toggling inside the settings dialog.
//!
//! Settings exposes each configured/known model for the current provider as a
//! checkbox. Toggling updates `[model_providers.<provider>].models` directly.

use crate::model::AppState;

/// Parse a toggle key produced by the settings dialog for a provider/model.
/// Keys have the form `edit_provider:<provider>:<model>`.
pub fn parse_provider_model_toggle(key: &str) -> Option<(&str, &str)> {
    let rest = key.strip_prefix("edit_provider:")?;
    rest.split_once(':')
}

/// Toggle whether `model` is enabled for `provider` in the saved config.
pub fn toggle_provider_model(state: &mut AppState, provider: &str, model: &str) {
    let provider = provider.to_string();
    let model = model.to_string();
    // Read current models: first from config_cache, then from file as fallback.
    // config_cache may be stale after save_provider_config is called externally.
    let current_models: Vec<String> = state
        .provider_config(&provider)
        .map(|p| p.models)
        .or_else(|| {
            crate::login_config::get_provider_config(&provider)
                .map(|(_, _, m)| m)
        })
        .unwrap_or_default();
    let mut models = current_models;
    let pos = models.iter().position(|m| m == &model);
    if let Some(idx) = pos {
        models.remove(idx);
    } else {
        models.push(model.clone());
        models.sort();
    }
    // Update config_cache synchronously and persist to file.
    sync_provider_models(state, &provider, &models);
    state.set_provider_models(&provider, models.clone());
    if provider == state.config.current_provider && !models.contains(&model) {
        if let Some(first) = models.first() {
            state.switch_model(provider.clone(), first.clone(), false);
        }
    }
    state.view.cached_settings_valid = false;
}

fn sync_provider_models(state: &mut AppState, provider: &str, models: &[String]) {
    // Read current provider config from file (bypasses stale config_cache).
    let current_from_file = crate::login_config::get_provider_config(provider);
    let (base_url, api_key) = current_from_file
        .map(|(b, k, _)| (b, k))
        .unwrap_or_else(|| {
            (
                crate::provider_registry::find_provider(provider)
                    .map(|p| p.base_url.to_string())
                    .unwrap_or_default(),
                String::new(),
            )
        });
    // Persist to file.
    if let Err(e) = crate::login_config::save_provider_config(provider, &base_url, &api_key, models)
    {
        tracing::warn!("failed to persist provider models: {}", e);
        return;
    }
    // Sync config_cache.
    if let Some(ref mut cache) = state.config_cache {
        cache
            .model_providers
            .entry(provider.into())
            .or_insert_with(|| crate::config::ModelProvider {
                provider_type: None,
                base_url: base_url.clone(),
                api_key: api_key.clone(),
                models: vec![],
            })
            .models = models.into();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    fn temp_config_path() -> PathBuf {
        use std::sync::atomic::{AtomicU64, Ordering};
        static COUNTER: AtomicU64 = AtomicU64::new(0);
        let n = COUNTER.fetch_add(1, Ordering::SeqCst);
        PathBuf::from(format!(
            "/tmp/runie_provider_toggle_test_{}_{}.toml",
            std::process::id(),
            n
        ))
    }

    #[test]
    fn parse_settings_toggle_key_extracts_provider_and_model() {
        assert_eq!(
            parse_provider_model_toggle("edit_provider:openai:gpt-4o"),
            Some(("openai", "gpt-4o"))
        );
    }

    #[test]
    fn parse_settings_toggle_key_rejects_malformed_keys() {
        assert!(parse_provider_model_toggle("edit_provider:gpt-4o").is_none());
        assert!(parse_provider_model_toggle("other:openai:gpt-4o").is_none());
    }

    #[test]
    fn toggle_provider_model_disables_model_and_switches_active() {
        let path = temp_config_path();
        crate::login_config::set_test_config_path(path);
        crate::login_config::save_provider_config(
            "openai",
            "https://api.openai.com/v1",
            "sk-test",
            &["gpt-4o".into(), "gpt-4o-mini".into()],
        )
        .unwrap();

        let mut state = AppState::default();
        state.config.current_provider = "openai".into();
        state.config.current_model = "gpt-4o-mini".into();

        toggle_provider_model(&mut state, "openai", "gpt-4o-mini");

        let models = crate::login_config::get_provider_config("openai")
            .map(|(_, _, m)| m)
            .unwrap_or_default();
        assert_eq!(models, vec!["gpt-4o"]);
        assert_eq!(state.config.current_model, "gpt-4o");
    }

    #[test]
    fn toggle_provider_model_enables_missing_model() {
        let path = temp_config_path();
        crate::login_config::set_test_config_path(path);
        crate::login_config::save_provider_config(
            "openai",
            "https://api.openai.com/v1",
            "sk-test",
            &["gpt-4o".into()],
        )
        .unwrap();

        let mut state = AppState::default();
        state.config.current_provider = "openai".into();
        state.config.current_model = "gpt-4o".into();

        toggle_provider_model(&mut state, "openai", "gpt-4o-mini");

        let models = crate::login_config::get_provider_config("openai")
            .map(|(_, _, m)| m)
            .unwrap_or_default();
        assert!(models.contains(&"gpt-4o".to_string()));
        assert!(models.contains(&"gpt-4o-mini".to_string()));
    }
}
