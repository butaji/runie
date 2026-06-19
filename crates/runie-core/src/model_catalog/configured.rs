//! Mapping from saved provider configurations to model-catalog entries.

use super::{model_catalog, ModelInfo};

/// Build a model catalog from the user's saved provider configurations.
///
/// Models that exist in the static catalog keep their metadata; models that
/// do not are represented as synthetic entries. This guarantees the `/model`
/// selector shows every configured model, even if the provider returned a
/// model name that is not yet in the bundled registry.
pub fn configured_models_catalog(
    configured: &[(String, String, Vec<String>)],
) -> Vec<ModelInfo> {
    let catalog = model_catalog();
    let mut models = Vec::new();
    for (provider, _base_url, chosen) in configured {
        let names: Vec<String> = if chosen.is_empty() {
            // When a provider is configured without an explicit model list,
            // fall back to the static catalog so the /model selector is usable.
            catalog
                .iter()
                .filter(|m| m.provider == *provider)
                .map(|m| m.name.clone())
                .collect()
        } else {
            chosen.clone()
        };
        for name in names {
            if let Some(info) = catalog
                .iter()
                .find(|m| m.provider == *provider && m.name == name)
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
        let configured = vec![(
            "openai".into(),
            "http://test".into(),
            vec!["gpt-4o".into()],
        )];
        let models = configured_models_catalog(&configured);
        assert_eq!(models.len(), 1);
        assert!(!models.iter().any(|m| m.name == "gpt-4o-mini"));
    }

    #[test]
    fn falls_back_to_static_catalog_when_no_models_configured() {
        let configured = vec![("minimax".into(), "http://test".into(), vec![])];
        let models = configured_models_catalog(&configured);
        assert!(
            models.iter().any(|m| m.full() == "minimax/MiniMax-M3"),
            "should include MiniMax-M3 from static catalog"
        );
        assert!(
            models.iter().any(|m| m.full() == "minimax/MiniMax-M2.7"),
            "should include MiniMax-M2.7 from static catalog"
        );
    }
}
