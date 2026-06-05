use async_trait::async_trait;

#[derive(Debug, Clone, PartialEq)]
pub struct ModelInfo {
    pub id: String,
    pub name: String,
}

#[derive(Debug, thiserror::Error)]
pub enum FetchError {
    #[error("API error: {0}")]
    ApiError(String),
    #[error("Unsupported provider: {0}")]
    UnsupportedProvider(String),
    #[error("HTTP error: {0}")]
    HttpError(String),
}

#[async_trait]
pub trait ModelFetcher: Send + Sync {
    async fn fetch_models(&self, _api_key: &str) -> Result<Vec<ModelInfo>, FetchError>;
}

pub struct ProviderModelFetcher {
    provider_id: String,
}

impl ProviderModelFetcher {

    #[must_use]
    pub fn new(provider_id: &str, _base_url: Option<&str>) -> Self {
        Self {
            provider_id: provider_id.to_lowercase(),
        }
    }
}

#[async_trait]
impl ModelFetcher for ProviderModelFetcher {
    async fn fetch_models(&self, _api_key: &str) -> Result<Vec<ModelInfo>, FetchError> {
        registry::get_provider_models(&self.provider_id)
            .ok_or_else(|| FetchError::UnsupportedProvider(self.provider_id.clone()))
    }
}

pub fn create_fetcher(provider_id: &str) -> Box<dyn ModelFetcher> {
    Box::new(ProviderModelFetcher::new(provider_id, None))
}

pub use registry::get_provider_models;

pub mod providers;
pub mod registry;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_model_info_creation() {
        let model = ModelInfo {
            id: "gpt-4".to_string(),
            name: "GPT-4".to_string(),
        };
        assert_eq!(model.id, "gpt-4");
        assert_eq!(model.name, "GPT-4");
    }

    #[test]
    fn test_fetch_error_display() {
        let err = FetchError::ApiError("test error".to_string());
        assert!(err.to_string().contains("test error"));

        let err = FetchError::UnsupportedProvider("test".to_string());
        assert!(err.to_string().contains("test"));

        let err = FetchError::HttpError("connection failed".to_string());
        assert!(err.to_string().contains("connection failed"));
    }

    #[test]
    fn test_create_fetcher_returns_trait_object() {
        let fetcher = create_fetcher("openai");
        let _ = fetcher;
    }

    #[test]
    fn test_get_provider_models_openai() {
        let models = get_provider_models("openai").unwrap();
        assert!(!models.is_empty());
        assert!(models.iter().any(|m| m.id == "gpt-4o"));
    }

    #[test]
    fn test_get_provider_models_anthropic() {
        let models = get_provider_models("anthropic").unwrap();
        assert!(!models.is_empty());
        assert!(models.iter().any(|m| m.id == "claude-3-5-sonnet-20241022"));
    }

    #[test]
    fn test_get_provider_models_unsupported() {
        assert!(get_provider_models("nonexistent").is_none());
    }

    #[test]
    fn test_model_registry_lookup_known_provider() {
        let models = get_provider_models("openai");
        assert!(models.is_some());
        let models = models.unwrap();
        assert!(!models.is_empty());
    }

    #[test]
    fn test_model_registry_lookup_unknown_provider() {
        let models = get_provider_models("nonexistent");
        assert!(models.is_none());
    }

    #[test]
    fn test_all_providers_have_models() {
        let providers = vec![
            "openai", "anthropic", "groq", "together", "xai", "mistral",
            "deepseek", "openrouter", "ollama", "minimax", "azure",
            "huggingface", "cohere", "zai", "mira", "galadriel",
            "llamafile", "perplexity", "moonshot", "hyperbolic",
        ];

        for provider in providers {
            let models = get_provider_models(provider);
            assert!(
                models.is_some(),
                "Provider {} should have models",
                provider
            );
            let models = models.unwrap();
            assert!(
                !models.is_empty(),
                "Provider {} should have non-empty models",
                provider
            );
        }
    }

    #[test]
    fn test_provider_id_case_insensitive() {
        let _ = create_fetcher("OPENAI");
        let _ = create_fetcher("OpenAI");
        let _ = create_fetcher("openai");
    }

    #[test]
    fn test_model_info_partial_eq() {
        let m1 = ModelInfo { id: "a".to_string(), name: "A".to_string() };
        let m2 = ModelInfo { id: "a".to_string(), name: "A".to_string() };
        let m3 = ModelInfo { id: "b".to_string(), name: "B".to_string() };
        assert_eq!(m1, m2);
        assert_ne!(m1, m3);
    }

    #[tokio::test]
    async fn test_provider_fetcher_returns_models() {
        let fetcher = ProviderModelFetcher::new("openai", None);
        let result = fetcher.fetch_models("any-key").await;
        assert!(result.is_ok());
        let models = result.unwrap();
        assert!(!models.is_empty());
    }

    #[tokio::test]
    async fn test_provider_fetcher_unsupported_returns_error() {
        let fetcher = ProviderModelFetcher::new("nonexistent-provider", None);
        let result = fetcher.fetch_models("any-key").await;
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(matches!(err, FetchError::UnsupportedProvider(_)));
    }

    #[tokio::test]
    async fn test_provider_fetcher_case_insensitive() {
        let fetcher = ProviderModelFetcher::new("OPENAI", None);
        let result = fetcher.fetch_models("any-key").await;
        assert!(result.is_ok());
    }
}
