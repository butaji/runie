//! Mapping from saved provider configurations to model-catalog entries.

use super::{model_catalog, ModelInfo};

/// Build a model catalog from the user's saved provider configurations.
///
/// Models that exist in the static catalog keep their metadata; models that
/// do not are represented as synthetic entries. This guarantees the `/model`
/// selector shows every configured model, even if the provider returned a
/// model name that is not yet in the bundled registry.
///
/// A provider configured without an explicit model list falls back to its
/// known models from the static catalog, so the `/model` selector is never
/// empty right after connecting a provider (the user can pick one instead of
/// having to detour through `/provider` first). Providers with no catalog
/// entry and no chosen models contribute nothing.
pub fn configured_models_catalog(configured: &[(String, String, Vec<String>)]) -> Vec<ModelInfo> {
    let catalog = model_catalog();
    let mut models = Vec::new();
    for (provider, _base_url, chosen) in configured {
        if chosen.is_empty() {
            // No models chosen yet: surface the provider's known catalog models
            // so the switcher is usable immediately after connecting.
            models.extend(catalog.iter().filter(|m| m.provider == *provider).cloned());
            continue;
        }
        for name in chosen {
            if let Some(info) = catalog
                .iter()
                .find(|m| m.provider == *provider && m.name == *name)
            {
                models.push(info.clone());
            } else {
                models.push(ModelInfo::new(provider.clone(), name.clone()));
            }
        }
    }
    models
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn includes_unknown_models() {
        let configured = vec![(
            "custom".into(),
            "http://test".into(),
            vec!["foo".into(), "bar".into()],
        )];
        let models = configured_models_catalog(&configured);
        assert_eq!(models.len(), 2);
        assert!(models.iter().any(|m| m.full() == "custom/foo"));
        assert!(models.iter().any(|m| m.full() == "custom/bar"));
    }

    #[test]
    fn preserves_static_metadata_for_known_models() {
        let configured = vec![("openai".into(), "http://test".into(), vec!["gpt-4o".into()])];
        let models = configured_models_catalog(&configured);
        let gpt4o = models.iter().find(|m| m.full() == "openai/gpt-4o").unwrap();
        assert_eq!(gpt4o.cost_prompt, Some(5.0));
    }

    #[test]
    fn excludes_models_not_in_provider_config() {
        let configured = vec![("openai".into(), "http://test".into(), vec!["gpt-4o".into()])];
        let models = configured_models_catalog(&configured);
        assert_eq!(models.len(), 1);
        assert!(!models.iter().any(|m| m.name == "gpt-4o-mini"));
    }

    /// Regression (live-test #7): a provider that is configured but has no
    /// models chosen yet must still show its known models in `/model`, so the
    /// switcher is not empty right after connecting a provider.
    #[test]
    fn falls_back_to_known_models_when_none_configured() {
        let configured = vec![("openai".into(), "http://test".into(), vec![])];
        let models = configured_models_catalog(&configured);
        assert!(
            !models.is_empty(),
            "configured provider with no chosen models should fall back to its known catalog models"
        );
        assert!(
            models.iter().any(|m| m.full() == "openai/gpt-4o"),
            "fallback should include the provider's known models (e.g. openai/gpt-4o), got {models:?}"
        );
        assert!(
            models.iter().all(|m| m.provider == "openai"),
            "fallback must only add models for the empty provider itself"
        );
    }

    /// A custom provider with no chosen models and no static-catalog entry has
    /// nothing to fall back to, so it still contributes no models.
    #[test]
    fn unknown_provider_without_models_stays_empty() {
        let configured = vec![("custom-provider-xyz".into(), "http://test".into(), vec![])];
        let models = configured_models_catalog(&configured);
        assert!(
            models.is_empty(),
            "unknown provider with no chosen models has no known models to fall back to, got {models:?}"
        );
    }
}
