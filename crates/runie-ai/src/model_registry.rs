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
        self.register_mock_models();
    }

    fn register_mock_models(&mut self) {
        self.register(ModelInfo {
            id: "mock-gpt-4".to_string(),
            name: "Mock GPT-4".to_string(),
            provider: "mock".to_string(),
            context_window: 128_000,
            input_price_per_1k: 0.0,
            output_price_per_1k: 0.0,
            supports_tools: true,
            supports_vision: true,
        });
    }

    fn register_openai_models(&mut self) {
        let models = [
            ("gpt-4o", "GPT-4o", 128_000, 0.005, 0.015, true, true),
            ("gpt-4o-mini", "GPT-4o Mini", 128_000, 0.00015, 0.0006, true, true),
            ("gpt-4-turbo", "GPT-4 Turbo", 128_000, 0.01, 0.03, true, true),
            ("gpt-4", "GPT-4", 8_192, 0.03, 0.06, true, false),
        ];
        self.register_provider_models("openai", &models);
    }

    fn register_anthropic_models(&mut self) {
        let models = [
            ("claude-3-5-sonnet-20241022", "Claude 3.5 Sonnet", 200_000, 0.003, 0.015, true, true),
            ("claude-3-5-sonnet-20240620", "Claude 3.5 Sonnet (June)", 200_000, 0.003, 0.015, true, true),
            ("claude-3-opus-20240229", "Claude 3 Opus", 200_000, 0.015, 0.075, true, true),
            ("claude-3-sonnet-20240229", "Claude 3 Sonnet", 200_000, 0.003, 0.015, true, true),
            ("claude-3-haiku-20240307", "Claude 3 Haiku", 200_000, 0.00025, 0.00125, true, true),
        ];
        self.register_provider_models("anthropic", &models);
    }

    fn register_google_models(&mut self) {
        let models = [
            ("gemini-1.5-pro", "Gemini 1.5 Pro", 2_000_000, 0.00125, 0.005, true, true),
            ("gemini-1.5-flash", "Gemini 1.5 Flash", 1_000_000, 0.000075, 0.0003, true, true),
            ("gemini-1.5-flash-8b", "Gemini 1.5 Flash-8B", 1_000_000, 0.0000375, 0.00015, true, true),
            ("gemini-2.0-flash", "Gemini 2.0 Flash", 1_000_000, 0.000075, 0.0003, true, true),
            ("gemini-pro", "Gemini Pro", 32_768, 0.00125, 0.005, true, true),
        ];
        self.register_provider_models("google", &models);
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

    fn register_provider_models(
        &mut self,
        provider: &str,
        models: &[(&str, &str, usize, f64, f64, bool, bool)],
    ) {
        for (id, name, context_window, input_price, output_price, tools, vision) in models {
            self.register(ModelInfo {
                id: (*id).to_string(),
                name: (*name).to_string(),
                provider: provider.to_string(),
                context_window: *context_window,
                input_price_per_1k: *input_price,
                output_price_per_1k: *output_price,
                supports_tools: *tools,
                supports_vision: *vision,
            });
        }
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
    fn test_registry_contains_mock_model() {
        let registry = ModelRegistry::new();
        let mock = registry.get("mock-gpt-4").expect("mock-gpt-4 should be registered");
        assert_eq!(mock.provider, "mock");
        assert_eq!(mock.context_window, 128_000);
    }

    #[test]
    fn test_mock_provider_in_providers_list() {
        let registry = ModelRegistry::new();
        let providers = registry.providers();
        assert!(providers.contains(&&"mock".to_string()));
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
