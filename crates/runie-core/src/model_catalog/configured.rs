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
        let configured = vec![(
            "openai".into(),
            "http://test".into(),
            vec!["gpt-4o".into()],
        )];
        let models = configured_models_catalog(&configured);
        assert_eq!(models.len(), 1);
        assert!(!models.iter().any(|m| m.name == "gpt-4o-mini"));
    }
}
