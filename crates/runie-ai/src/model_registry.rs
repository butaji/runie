use std::collections::HashMap;

#[derive(Debug, Clone)]
pub struct ModelInfo {
    pub id: String,
    pub name: String,
    pub provider: String,
    pub context_window: usize,
    pub input_price_per_1k: f64,
    pub output_price_per_1k: f64,
    pub supports_tools: bool,
    pub supports_vision: bool,
}

pub struct ModelRegistry {
    models: HashMap<String, ModelInfo>,
    provider_models: HashMap<String, Vec<String>>,
}

impl Default for ModelRegistry {
    fn default() -> Self {
        Self::new()
    }
}

impl ModelRegistry {
    pub fn new() -> Self {
        let mut registry = Self {
            models: HashMap::new(),
            provider_models: HashMap::new(),
        };
        registry.register_defaults();
        registry
    }

    fn register_defaults(&mut self) {
        self.register_openai_models();
        self.register_anthropic_models();
        self.register_google_models();
    }

    fn register_openai_models(&mut self) {
        self.register(ModelInfo {
            id: "gpt-4o".to_string(),
            name: "GPT-4o".to_string(),
            provider: "openai".to_string(),
            context_window: 128_000,
            input_price_per_1k: 0.005,
            output_price_per_1k: 0.015,
            supports_tools: true,
            supports_vision: true,
        });
        self.register(ModelInfo {
            id: "gpt-4o-mini".to_string(),
            name: "GPT-4o Mini".to_string(),
            provider: "openai".to_string(),
            context_window: 128_000,
            input_price_per_1k: 0.00015,
            output_price_per_1k: 0.0006,
            supports_tools: true,
            supports_vision: true,
        });
        self.register(ModelInfo {
            id: "gpt-4-turbo".to_string(),
            name: "GPT-4 Turbo".to_string(),
            provider: "openai".to_string(),
            context_window: 128_000,
            input_price_per_1k: 0.01,
            output_price_per_1k: 0.03,
            supports_tools: true,
            supports_vision: true,
        });
        self.register(ModelInfo {
            id: "gpt-4".to_string(),
            name: "GPT-4".to_string(),
            provider: "openai".to_string(),
            context_window: 8_192,
            input_price_per_1k: 0.03,
            output_price_per_1k: 0.06,
            supports_tools: true,
            supports_vision: false,
        });
    }

    fn register_anthropic_models(&mut self) {
        self.register(ModelInfo {
            id: "claude-3-5-sonnet-20241022".to_string(),
            name: "Claude 3.5 Sonnet".to_string(),
            provider: "anthropic".to_string(),
            context_window: 200_000,
            input_price_per_1k: 0.003,
            output_price_per_1k: 0.015,
            supports_tools: true,
            supports_vision: true,
        });
        self.register(ModelInfo {
            id: "claude-3-5-sonnet-20240620".to_string(),
            name: "Claude 3.5 Sonnet (June)".to_string(),
            provider: "anthropic".to_string(),
            context_window: 200_000,
            input_price_per_1k: 0.003,
            output_price_per_1k: 0.015,
            supports_tools: true,
            supports_vision: true,
        });
        self.register(ModelInfo {
            id: "claude-3-opus-20240229".to_string(),
            name: "Claude 3 Opus".to_string(),
            provider: "anthropic".to_string(),
            context_window: 200_000,
            input_price_per_1k: 0.015,
            output_price_per_1k: 0.075,
            supports_tools: true,
            supports_vision: true,
        });
        self.register(ModelInfo {
            id: "claude-3-sonnet-20240229".to_string(),
            name: "Claude 3 Sonnet".to_string(),
            provider: "anthropic".to_string(),
            context_window: 200_000,
            input_price_per_1k: 0.003,
            output_price_per_1k: 0.015,
            supports_tools: true,
            supports_vision: true,
        });
        self.register(ModelInfo {
            id: "claude-3-haiku-20240307".to_string(),
            name: "Claude 3 Haiku".to_string(),
            provider: "anthropic".to_string(),
            context_window: 200_000,
            input_price_per_1k: 0.00025,
            output_price_per_1k: 0.00125,
            supports_tools: true,
            supports_vision: true,
        });
    }

    fn register_google_models(&mut self) {
        self.register(ModelInfo {
            id: "gemini-1.5-pro".to_string(),
            name: "Gemini 1.5 Pro".to_string(),
            provider: "google".to_string(),
            context_window: 2_000_000,
            input_price_per_1k: 0.00125,
            output_price_per_1k: 0.005,
            supports_tools: true,
            supports_vision: true,
        });
        self.register(ModelInfo {
            id: "gemini-1.5-flash".to_string(),
            name: "Gemini 1.5 Flash".to_string(),
            provider: "google".to_string(),
            context_window: 1_000_000,
            input_price_per_1k: 0.000075,
            output_price_per_1k: 0.0003,
            supports_tools: true,
            supports_vision: true,
        });
        self.register(ModelInfo {
            id: "gemini-1.5-flash-8b".to_string(),
            name: "Gemini 1.5 Flash-8B".to_string(),
            provider: "google".to_string(),
            context_window: 1_000_000,
            input_price_per_1k: 0.0000375,
            output_price_per_1k: 0.00015,
            supports_tools: true,
            supports_vision: true,
        });
        self.register(ModelInfo {
            id: "gemini-2.0-flash".to_string(),
            name: "Gemini 2.0 Flash".to_string(),
            provider: "google".to_string(),
            context_window: 1_000_000,
            input_price_per_1k: 0.000075,
            output_price_per_1k: 0.0003,
            supports_tools: true,
            supports_vision: true,
        });
        self.register(ModelInfo {
            id: "gemini-pro".to_string(),
            name: "Gemini Pro".to_string(),
            provider: "google".to_string(),
            context_window: 32_768,
            input_price_per_1k: 0.00125,
            output_price_per_1k: 0.005,
            supports_tools: true,
            supports_vision: true,
        });
    }

    fn register(&mut self, model: ModelInfo) {
        let provider = model.provider.clone();
        let id = model.id.clone();
        self.models.insert(id.clone(), model);
        self.provider_models
            .entry(provider)
            .or_insert_with(Vec::new)
            .push(id);
    }

    pub fn get(&self, id: &str) -> Option<&ModelInfo> {
        self.models.get(id)
    }

    pub fn list_by_provider(&self, provider: &str) -> Vec<&ModelInfo> {
        self.provider_models
            .get(provider)
            .map(|ids| ids.iter().filter_map(|id| self.models.get(id)).collect())
            .unwrap_or_default()
    }

    pub fn list_all(&self) -> Vec<&ModelInfo> {
        self.models.values().collect()
    }

    pub fn providers(&self) -> Vec<&String> {
        self.provider_models.keys().collect()
    }

    pub fn find_by_name(&self, query: &str) -> Vec<&ModelInfo> {
        let query_lower = query.to_lowercase();
        self.models
            .values()
            .filter(|m| {
                m.id.to_lowercase().contains(&query_lower)
                    || m.name.to_lowercase().contains(&query_lower)
            })
            .collect()
    }
}

#[allow(clippy::unwrap_used)]
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_registry_contains_default_models() {
        let registry = ModelRegistry::new();
        assert!(registry.get("gpt-4o").is_some());
        assert!(registry.get("claude-3-5-sonnet-20241022").is_some());
        assert!(registry.get("gemini-1.5-pro").is_some());
    }

    #[test]
    fn test_list_by_provider() {
        let registry = ModelRegistry::new();
        let openai_models = registry.list_by_provider("openai");
        assert!(!openai_models.is_empty());
        assert!(openai_models.iter().all(|m| m.provider == "openai"));
    }

    #[test]
    fn test_find_by_name() {
        let registry = ModelRegistry::new();
        let results = registry.find_by_name("gpt");
        assert!(!results.is_empty());
    }

    #[test]
    fn test_providers() {
        let registry = ModelRegistry::new();
        let providers = registry.providers();
        assert!(providers.contains(&&"openai".to_string()));
        assert!(providers.contains(&&"anthropic".to_string()));
        assert!(providers.contains(&&"google".to_string()));
    }

    #[test]
    fn test_model_info() {
        let registry = ModelRegistry::new();
        let model = registry.get("gpt-4o").unwrap();
        assert_eq!(model.name, "GPT-4o");
        assert_eq!(model.context_window, 128_000);
        assert!(model.supports_tools);
        assert!(model.supports_vision);
    }
}
