//! Provider/model data loaded from YAML files.
//!
//! YAML files live in `resources/models/` and are embedded via `include_str!`.
//! A build script validates the YAML at compile time and generates checksums.

use serde::Deserialize;

/// YAML representation of a provider's metadata.
#[derive(Debug, Clone, Deserialize)]
pub struct ProviderYaml {
    pub key: String,
    pub display_name: String,
    pub base_url: String,
    pub env_var: String,
    pub models: Vec<ModelYaml>,
}

/// YAML representation of a model's metadata.
#[derive(Debug, Clone, Deserialize)]
pub struct ModelYaml {
    pub name: String,
    #[serde(default)]
    pub cost_prompt: Option<f64>,
    #[serde(default)]
    pub cost_completion: Option<f64>,
    #[serde(default)]
    pub supports_thinking: bool,
    #[serde(default)]
    pub supports_vision: bool,
    #[serde(default)]
    pub tokenizer: Option<String>,
    #[serde(default)]
    pub context_window: Option<usize>,
    #[serde(default = "default_true")]
    pub streaming: bool,
    #[serde(default = "default_true")]
    pub supports_tools: bool,
    #[serde(default)]
    pub supports_reasoning: bool,
    #[serde(default = "default_true")]
    pub supports_system: bool,
    #[serde(default)]
    pub max_output_tokens: usize,
    #[serde(default)]
    pub cache_control: bool,
}

fn default_true() -> bool {
    true
}

/// Parse a provider YAML file.
pub fn parse_provider_yaml(yaml: &str) -> Result<ProviderYaml, serde_yaml::Error> {
    serde_yaml::from_str(yaml)
}

/// Get the list of embedded YAML files for all providers.
pub fn provider_yaml_files() -> Vec<(&'static str, &'static str)> {
    vec![
        ("anthropic", include_str!("../../../resources/models/anthropic.yaml")),
        ("openai", include_str!("../../../resources/models/openai.yaml")),
        ("google", include_str!("../../../resources/models/google.yaml")),
        ("deepseek", include_str!("../../../resources/models/deepseek.yaml")),
        ("openrouter", include_str!("../../../resources/models/openrouter.yaml")),
        ("groq", include_str!("../../../resources/models/groq.yaml")),
        ("mistral", include_str!("../../../resources/models/mistral.yaml")),
        ("fireworks", include_str!("../../../resources/models/fireworks.yaml")),
        ("together", include_str!("../../../resources/models/together.yaml")),
        ("minimax", include_str!("../../../resources/models/minimax.yaml")),
        ("moonshotai", include_str!("../../../resources/models/moonshotai.yaml")),
        ("xai", include_str!("../../../resources/models/xai.yaml")),
        ("ollama", include_str!("../../../resources/models/ollama.yaml")),
    ]
}

/// Mock provider YAML (dev-only).
pub fn mock_provider_yaml() -> ProviderYaml {
    ProviderYaml {
        key: "mock".to_string(),
        display_name: "Mock (dev only)".to_string(),
        base_url: "http://localhost/mock".to_string(),
        env_var: String::new(),
        models: vec![ModelYaml {
            name: "echo".to_string(),
            cost_prompt: None,
            cost_completion: None,
            supports_thinking: false,
            supports_vision: false,
            tokenizer: None,
            context_window: None,
            streaming: true,
            supports_tools: true,
            supports_reasoning: false,
            supports_system: true,
            max_output_tokens: 0,
            cache_control: false,
        }],
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_anthropic_yaml() {
        let yaml = include_str!("../../../resources/models/anthropic.yaml");
        let provider: ProviderYaml = serde_yaml::from_str(yaml).unwrap();
        assert_eq!(provider.key, "anthropic");
        assert!(!provider.models.is_empty());
    }

    #[test]
    fn parse_openai_yaml() {
        let yaml = include_str!("../../../resources/models/openai.yaml");
        let provider: ProviderYaml = serde_yaml::from_str(yaml).unwrap();
        assert_eq!(provider.key, "openai");
        assert!(!provider.models.is_empty());
    }

    #[test]
    fn all_provider_yaml_files_parse() {
        for (key, yaml) in provider_yaml_files() {
            let provider: ProviderYaml = serde_yaml::from_str(yaml).unwrap();
            assert_eq!(provider.key, key, "YAML key should match filename");
            assert!(!provider.models.is_empty(), "Provider {} should have models", key);
        }
    }

    #[test]
    fn model_has_required_fields() {
        let yaml = include_str!("../../../resources/models/openai.yaml");
        let provider: ProviderYaml = serde_yaml::from_str(yaml).unwrap();
        for model in &provider.models {
            assert!(!model.name.is_empty(), "Model name should not be empty");
            assert!(
                model.context_window.is_some(),
                "Model {} should have context_window",
                model.name
            );
        }
    }
}
