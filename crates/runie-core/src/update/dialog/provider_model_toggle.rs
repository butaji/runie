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
    let path = crate::login_config::config_path();
    let mut config = crate::async_io::block_in_place_if_runtime(|| {
        crate::config::Config::load(Some(&path))
    });
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
    let _ = crate::async_io::block_in_place_if_runtime(|| config.save_to(&path));
    if provider == state.config.current_provider && !models.contains(&model.to_string()) {
        if let Some(first) = models.first() {
            state.switch_model(provider.into(), first.clone(), false);
        }
    }
    state.view.cached_settings_valid = false;
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
