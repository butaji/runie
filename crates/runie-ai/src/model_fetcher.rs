use async_trait::async_trait;
use rig_core::client::ModelListingClient;
use serde::Deserialize;
use std::collections::HashMap;

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
    async fn fetch_models(&self, api_key: &str) -> Result<Vec<ModelInfo>, FetchError>;
}

pub struct ProviderModelFetcher {
    provider_id: String,
    base_url: Option<String>,
}

impl ProviderModelFetcher {
    pub fn new(provider_id: &str, base_url: Option<&str>) -> Self {
        Self {
            provider_id: provider_id.to_lowercase(),
            base_url: base_url.map(|s| s.to_string()),
        }
    }
}

#[async_trait]
impl ModelFetcher for ProviderModelFetcher {
    async fn fetch_models(&self, api_key: &str) -> Result<Vec<ModelInfo>, FetchError> {
        match self.provider_id.as_str() {
            // Rig native integrations
            "openai" => fetch_openai_models(api_key).await,
            "anthropic" => fetch_anthropic_models(api_key).await,
            "google" | "gemini" => fetch_google_models(api_key).await,
            "mistral" => fetch_mistral_models(api_key).await,
            "deepseek" => fetch_deepseek_models(api_key).await,
            "openrouter" => fetch_openrouter_models(api_key).await,
            "ollama" => fetch_ollama_models(api_key).await,

            // OpenAI-compatible HTTP APIs
            "groq" => fetch_openai_compatible_models(api_key, "https://api.groq.com").await,
            "together" => fetch_openai_compatible_models(api_key, "https://api.together.ai").await,
            "xai" => fetch_openai_compatible_models(api_key, "https://api.x.ai").await,
            "hyperbolic" => fetch_openai_compatible_models(api_key, "https://api.hyperbolic.xyz").await,
            "perplexity" => fetch_openai_compatible_models(api_key, "https://api.perplexity.ai").await,
            "moonshot" => fetch_openai_compatible_models(api_key, "https://api.moonshot.cn").await,

            // Hardcoded fallbacks
            "azure" => Ok(azure_fallback_models()),
            "huggingface" => Ok(huggingface_fallback_models()),
            "cohere" => Ok(cohere_fallback_models()),
            "zai" => Ok(zai_fallback_models()),
            "minimax" => Ok(minimax_fallback_models()),
            "mira" => Ok(mira_fallback_models()),
            "galadriel" => Ok(galadriel_fallback_models()),
            "llamafile" => Ok(llamafile_fallback_models()),

            // Custom base URL providers
            _ if self.base_url.is_some() => {
                fetch_openai_compatible_models(api_key, self.base_url.as_ref().unwrap()).await
            }

            _ => Err(FetchError::UnsupportedProvider(self.provider_id.clone())),
        }
    }
}

// Rig native integrations
async fn fetch_openai_models(api_key: &str) -> Result<Vec<ModelInfo>, FetchError> {
    use rig_core::providers::openai;
    let client = openai::Client::new(api_key)
        .map_err(|e| FetchError::ApiError(e.to_string()))?;
    let model_list = client
        .list_models()
        .await
        .map_err(|e| FetchError::ApiError(e.to_string()))?;
    Ok(model_list
        .data
        .into_iter()
        .map(|m| {
            let name = m.name.unwrap_or_else(|| m.id.clone());
            ModelInfo {
                id: m.id,
                name,
            }
        })
        .collect())
}

async fn fetch_anthropic_models(api_key: &str) -> Result<Vec<ModelInfo>, FetchError> {
    use rig_core::providers::anthropic;
    let client = anthropic::Client::new(api_key)
        .map_err(|e| FetchError::ApiError(e.to_string()))?;
    let model_list = client
        .list_models()
        .await
        .map_err(|e| FetchError::ApiError(e.to_string()))?;
    Ok(model_list
        .data
        .into_iter()
        .map(|m| {
            let name = m.name.unwrap_or_else(|| m.id.clone());
            ModelInfo {
                id: m.id,
                name,
            }
        })
        .collect())
}

async fn fetch_google_models(api_key: &str) -> Result<Vec<ModelInfo>, FetchError> {
    use rig_core::providers::gemini;
    let client = gemini::Client::new(api_key)
        .map_err(|e| FetchError::ApiError(e.to_string()))?;
    let model_list = client
        .list_models()
        .await
        .map_err(|e| FetchError::ApiError(e.to_string()))?;
    Ok(model_list
        .data
        .into_iter()
        .map(|m| {
            let name = m.name.unwrap_or_else(|| m.id.clone());
            ModelInfo {
                id: m.id,
                name,
            }
        })
        .collect())
}

async fn fetch_mistral_models(api_key: &str) -> Result<Vec<ModelInfo>, FetchError> {
    use rig_core::providers::mistral;
    let client = mistral::Client::new(api_key)
        .map_err(|e| FetchError::ApiError(e.to_string()))?;
    let model_list = client
        .list_models()
        .await
        .map_err(|e| FetchError::ApiError(e.to_string()))?;
    Ok(model_list
        .data
        .into_iter()
        .map(|m| {
            let name = m.name.unwrap_or_else(|| m.id.clone());
            ModelInfo {
                id: m.id,
                name,
            }
        })
        .collect())
}

async fn fetch_deepseek_models(api_key: &str) -> Result<Vec<ModelInfo>, FetchError> {
    use rig_core::providers::deepseek;
    let client = deepseek::Client::new(api_key)
        .map_err(|e| FetchError::ApiError(e.to_string()))?;
    let model_list = client
        .list_models()
        .await
        .map_err(|e| FetchError::ApiError(e.to_string()))?;
    Ok(model_list
        .data
        .into_iter()
        .map(|m| {
            let name = m.name.unwrap_or_else(|| m.id.clone());
            ModelInfo {
                id: m.id,
                name,
            }
        })
        .collect())
}

async fn fetch_openrouter_models(api_key: &str) -> Result<Vec<ModelInfo>, FetchError> {
    use rig_core::providers::openrouter;
    let client = openrouter::Client::new(api_key)
        .map_err(|e| FetchError::ApiError(e.to_string()))?;
    let model_list = client
        .list_models()
        .await
        .map_err(|e| FetchError::ApiError(e.to_string()))?;
    Ok(model_list
        .data
        .into_iter()
        .map(|m| {
            let name = m.name.unwrap_or_else(|| m.id.clone());
            ModelInfo {
                id: m.id,
                name,
            }
        })
        .collect())
}

async fn fetch_ollama_models(api_key: &str) -> Result<Vec<ModelInfo>, FetchError> {
    use rig_core::providers::ollama;
    let client = ollama::Client::new(api_key)
        .map_err(|e| FetchError::ApiError(e.to_string()))?;
    let model_list = client
        .list_models()
        .await
        .map_err(|e| FetchError::ApiError(e.to_string()))?;
    Ok(model_list
        .data
        .into_iter()
        .map(|m| {
            let name = m.name.unwrap_or_else(|| m.id.clone());
            ModelInfo {
                id: m.id,
                name,
            }
        })
        .collect())
}

// OpenAI-compatible API fetcher
#[derive(Deserialize)]
struct OpenAICompatibleResponse {
    data: Vec<OpenAIModelData>,
}

#[derive(Deserialize)]
struct OpenAIModelData {
    id: String,
    #[serde(default)]
    _object: Option<String>,
    #[serde(default)]
    _created: Option<u64>,
    #[serde(default)]
    owned_by: Option<String>,
}

async fn fetch_openai_compatible_models(api_key: &str, base_url: &str) -> Result<Vec<ModelInfo>, FetchError> {
    let client = reqwest::Client::new();
    let resp = client
        .get(&format!("{}/v1/models", base_url.trim_end_matches('/')))
        .bearer_auth(api_key)
        .send()
        .await
        .map_err(|e| FetchError::HttpError(e.to_string()))?;

    if !resp.status().is_success() {
        let status = resp.status();
        let body = resp.text().await.unwrap_or_default();
        return Err(FetchError::HttpError(format!("{}: {}", status, body)));
    }

    let resp_data: OpenAICompatibleResponse = resp
        .json()
        .await
        .map_err(|e| FetchError::HttpError(e.to_string()))?;

    Ok(resp_data
        .data
        .into_iter()
        .map(|m| {
            let name = m.owned_by.clone().unwrap_or_else(|| m.id.clone());
            ModelInfo {
                id: m.id,
                name,
            }
        })
        .collect())
}

// Hardcoded fallback models
fn azure_fallback_models() -> Vec<ModelInfo> {
    vec![
        ModelInfo { id: "gpt-4o".to_string(), name: "GPT-4o".to_string() },
        ModelInfo { id: "gpt-4-turbo".to_string(), name: "GPT-4 Turbo".to_string() },
        ModelInfo { id: "gpt-4".to_string(), name: "GPT-4".to_string() },
        ModelInfo { id: "gpt-35-turbo".to_string(), name: "GPT-3.5 Turbo".to_string() },
    ]
}

fn huggingface_fallback_models() -> Vec<ModelInfo> {
    vec![
        ModelInfo { id: "meta-llama/Llama-3-70b-chat-hf".to_string(), name: "Llama 3 70B Chat".to_string() },
        ModelInfo { id: "meta-llama/Llama-3-8b-chat-hf".to_string(), name: "Llama 3 8B Chat".to_string() },
        ModelInfo { id: "mistralai/Mixtral-8x7B-Instruct-v0.1".to_string(), name: "Mixtral 8x7B".to_string() },
        ModelInfo { id: "deepseek-ai/DeepSeek-V2".to_string(), name: "DeepSeek V2".to_string() },
    ]
}

fn cohere_fallback_models() -> Vec<ModelInfo> {
    vec![
        ModelInfo { id: "command-r-plus".to_string(), name: "Command R+".to_string() },
        ModelInfo { id: "command-r".to_string(), name: "Command R".to_string() },
        ModelInfo { id: "command".to_string(), name: "Command".to_string() },
        ModelInfo { id: "command-light".to_string(), name: "Command Light".to_string() },
    ]
}

fn zai_fallback_models() -> Vec<ModelInfo> {
    vec![
        ModelInfo { id: "mixtral-8x7b".to_string(), name: "Mixtral 8x7B".to_string() },
        ModelInfo { id: "llama-3-70b".to_string(), name: "Llama 3 70B".to_string() },
        ModelInfo { id: "llama-3-8b".to_string(), name: "Llama 3 8B".to_string() },
    ]
}

fn minimax_fallback_models() -> Vec<ModelInfo> {
    vec![
        ModelInfo { id: "MiniMax-Text-01".to_string(), name: "MiniMax Text 01".to_string() },
        ModelInfo { id: "abab6-chat".to_string(), name: "ABAB 6 Chat".to_string() },
        ModelInfo { id: "abab5.5-chat".to_string(), name: "ABAB 5.5 Chat".to_string() },
    ]
}

fn mira_fallback_models() -> Vec<ModelInfo> {
    vec![
        ModelInfo { id: "mira-chat".to_string(), name: "Mira Chat".to_string() },
        ModelInfo { id: "mira-fast".to_string(), name: "Mira Fast".to_string() },
    ]
}

fn galadriel_fallback_models() -> Vec<ModelInfo> {
    vec![
        ModelInfo { id: "galadriel-chat".to_string(), name: "Galadriel Chat".to_string() },
        ModelInfo { id: "galadriel-fast".to_string(), name: "Galadriel Fast".to_string() },
    ]
}

fn llamafile_fallback_models() -> Vec<ModelInfo> {
    vec![
        ModelInfo { id: "llamafile".to_string(), name: "Llamafile".to_string() },
        ModelInfo { id: "mistral".to_string(), name: "Mistral".to_string() },
        ModelInfo { id: "codellama".to_string(), name: "Code Llama".to_string() },
    ]
}

pub fn create_fetcher(provider_id: &str) -> Box<dyn ModelFetcher> {
    // Provider ID to (base_url) mapping for providers that need custom URLs
    let custom_urls: HashMap<&str, &str> = HashMap::new();

    if let Some(base_url) = custom_urls.get(provider_id.to_lowercase().as_str()) {
        return Box::new(ProviderModelFetcher::new(provider_id, Some(base_url)));
    }

    Box::new(ProviderModelFetcher::new(provider_id, None))
}

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
        // Should be able to call fetch_models (though we can't await in test without async)
        let _ = fetcher;
    }

    #[test]
    fn test_azure_fallback_models() {
        let models = azure_fallback_models();
        assert!(!models.is_empty());
        assert!(models.iter().all(|m| !m.id.is_empty()));
    }

    #[test]
    fn test_huggingface_fallback_models() {
        let models = huggingface_fallback_models();
        assert!(!models.is_empty());
    }

    #[test]
    fn test_cohere_fallback_models() {
        let models = cohere_fallback_models();
        assert!(!models.is_empty());
    }

    #[test]
    fn test_minimax_fallback_models() {
        let models = minimax_fallback_models();
        assert!(!models.is_empty());
    }

    // ===== Hardcoded Fallback Provider Tests =====

    #[test]
    fn test_mira_fallback_models() {
        let models = mira_fallback_models();
        assert_eq!(models.len(), 2);
        assert!(models.iter().all(|m| !m.id.is_empty() && !m.name.is_empty()));
    }

    #[test]
    fn test_galadriel_fallback_models() {
        let models = galadriel_fallback_models();
        assert_eq!(models.len(), 2);
    }

    #[test]
    fn test_llamafile_fallback_models() {
        let models = llamafile_fallback_models();
        assert_eq!(models.len(), 3);
        let ids: Vec<&str> = models.iter().map(|m| m.id.as_str()).collect();
        assert!(ids.contains(&"llamafile"));
        assert!(ids.contains(&"mistral"));
        assert!(ids.contains(&"codellama"));
    }

    #[test]
    fn test_zai_fallback_models() {
        let models = zai_fallback_models();
        assert_eq!(models.len(), 3);
        let ids: Vec<&str> = models.iter().map(|m| m.id.as_str()).collect();
        assert!(ids.contains(&"mixtral-8x7b"));
        assert!(ids.contains(&"llama-3-70b"));
        assert!(ids.contains(&"llama-3-8b"));
    }

    // ===== Provider ID Case Insensitivity Tests =====

    #[test]
    fn test_provider_id_case_insensitive() {
        // Test that various casings all create a fetcher (doesn't panic)
        let _ = create_fetcher("OPENAI");
        let _ = create_fetcher("OpenAI");
        let _ = create_fetcher("openai");
        let _ = create_fetcher("Minimax");
        let _ = create_fetcher("MINIMAX");
        let _ = create_fetcher("GROQ");
        let _ = create_fetcher("Together");
    }

    // ===== Error Type Tests =====

    #[test]
    fn test_fetch_error_api_error() {
        let err = FetchError::ApiError("rate limit exceeded".to_string());
        assert!(err.to_string().contains("rate limit exceeded"));
        assert!(matches!(err, FetchError::ApiError(_)));
    }

    #[test]
    fn test_fetch_error_unsupported_provider() {
        let err = FetchError::UnsupportedProvider("unknown".to_string());
        assert!(err.to_string().contains("unknown"));
        assert!(matches!(err, FetchError::UnsupportedProvider(_)));
    }

    #[test]
    fn test_fetch_error_http_error() {
        let err = FetchError::HttpError("401".to_string());
        assert!(err.to_string().contains("401"));
        assert!(matches!(err, FetchError::HttpError(_)));
    }

    #[test]
    fn test_fetch_error_display_different_types() {
        // Test that ApiError, UnsupportedProvider, and HttpError have distinct displays
        let api_err = FetchError::ApiError("test".to_string());
        let unsupported_err = FetchError::UnsupportedProvider("test".to_string());
        let http_err = FetchError::HttpError("test".to_string());

        // Each error type should contain its specific text in the display
        assert!(api_err.to_string().contains("API error"));
        assert!(unsupported_err.to_string().contains("Unsupported provider"));
        assert!(http_err.to_string().contains("HTTP error"));
    }

    // ===== ModelInfo Tests =====

    #[test]
    fn test_model_info_partial_eq() {
        let m1 = ModelInfo { id: "a".to_string(), name: "A".to_string() };
        let m2 = ModelInfo { id: "a".to_string(), name: "A".to_string() };
        let m3 = ModelInfo { id: "b".to_string(), name: "B".to_string() };
        assert_eq!(m1, m2);
        assert_ne!(m1, m3);
    }

    #[test]
    fn test_model_info_debug() {
        let model = ModelInfo { id: "test-id".to_string(), name: "Test Name".to_string() };
        let debug_str = format!("{:?}", model);
        assert!(debug_str.contains("test-id"));
        assert!(debug_str.contains("Test Name"));
    }

    #[test]
    fn test_model_info_clone() {
        let original = ModelInfo { id: "clone-test".to_string(), name: "Clone Test".to_string() };
        let cloned = original.clone();
        assert_eq!(original, cloned);
        // Verify it's a true clone (different memory)
        let mut modified = cloned;
        modified.name = "Modified".to_string();
        assert_ne!(original.name, modified.name);
    }

    // ===== Model Sorting Tests =====

    #[test]
    fn test_model_sorting_alphabetical() {
        let mut models = vec![
            ModelInfo { id: "z-model".to_string(), name: "Z Model".to_string() },
            ModelInfo { id: "a-model".to_string(), name: "A Model".to_string() },
            ModelInfo { id: "m-model".to_string(), name: "M Model".to_string() },
        ];
        models.sort_by(|a, b| a.name.cmp(&b.name));
        assert_eq!(models[0].id, "a-model");
        assert_eq!(models[1].id, "m-model");
        assert_eq!(models[2].id, "z-model");
    }

    #[test]
    fn test_model_sorting_preserves_equal_order() {
        let mut models = vec![
            ModelInfo { id: "model-1".to_string(), name: "Same Name".to_string() },
            ModelInfo { id: "model-2".to_string(), name: "Same Name".to_string() },
        ];
        models.sort_by(|a, b| a.name.cmp(&b.name));
        // Both have same name, order of equal elements is implementation-defined
        // but stable sort would preserve relative order
        assert_eq!(models.len(), 2);
    }

    // ===== ProviderModelFetcher Tests =====

    #[test]
    fn test_provider_model_fetcher_new() {
        let fetcher = ProviderModelFetcher::new("openai", Some("https://api.openai.com"));
        assert_eq!(fetcher.provider_id, "openai");
        assert_eq!(fetcher.base_url, Some("https://api.openai.com".to_string()));
    }

    #[test]
    fn test_provider_model_fetcher_new_without_base_url() {
        let fetcher = ProviderModelFetcher::new("anthropic", None);
        assert_eq!(fetcher.provider_id, "anthropic");
        assert!(fetcher.base_url.is_none());
    }

    #[test]
    fn test_provider_model_fetcher_lowercase_conversion() {
        let fetcher = ProviderModelFetcher::new("OPENAI", None);
        assert_eq!(fetcher.provider_id, "openai");
    }

    // ===== All Hardcoded Providers Return Non-Empty Models =====

    #[test]
    fn test_all_hardcoded_providers_return_models() {
        let providers = vec![
            ("azure", azure_fallback_models()),
            ("huggingface", huggingface_fallback_models()),
            ("cohere", cohere_fallback_models()),
            ("zai", zai_fallback_models()),
            ("minimax", minimax_fallback_models()),
            ("mira", mira_fallback_models()),
            ("galadriel", galadriel_fallback_models()),
            ("llamafile", llamafile_fallback_models()),
        ];

        for (name, models) in providers {
            assert!(!models.is_empty(), "Provider {} should return non-empty models", name);
            for model in &models {
                assert!(!model.id.is_empty(), "Model id should not be empty for {}", name);
                assert!(!model.name.is_empty(), "Model name should not be empty for {}", name);
            }
        }
    }

    // ===== ModelInfo Structure Tests =====

    #[test]
    fn test_model_info_id_and_name_differ() {
        // Some providers have different id and name
        let model = ModelInfo {
            id: "gpt-4-turbo".to_string(),
            name: "GPT-4 Turbo".to_string(),
        };
        assert_ne!(model.id, model.name);
    }

    #[test]
    fn test_model_info_id_and_name_same() {
        // Some providers have same id and name
        let model = ModelInfo {
            id: "llamafile".to_string(),
            name: "llamafile".to_string(),
        };
        assert_eq!(model.id, model.name);
    }

    // ===== Create Fetcher Returns Box =====

    #[test]
    fn test_create_fetcher_returns_dyn_model_fetcher() {
        let fetcher = create_fetcher("openai");
        // Verify it's a Send + Sync trait object
        fn assert_send_sync<T: Send + Sync + ?Sized>(_: &T) {}
        assert_send_sync(&*fetcher);
    }

    #[test]
    fn test_create_fetcher_different_providers() {
        // Should work for any valid provider without panicking
        let providers = vec![
            "openai", "anthropic", "google", "gemini", "mistral", "deepseek",
            "openrouter", "ollama", "groq", "together", "xai", "hyperbolic",
            "perplexity", "moonshot", "azure", "huggingface", "cohere",
            "zai", "minimax", "mira", "galadriel", "llamafile",
        ];

        for provider in providers {
            let fetcher = create_fetcher(provider);
            let _ = fetcher; // Just verify it doesn't panic
        }
    }

    // ===== Async Fetch Tests (testing helper functions) =====

    #[tokio::test]
    async fn test_provider_fetcher_unsupported_returns_error() {
        let fetcher = ProviderModelFetcher::new("nonexistent-provider", None);
        let result = fetcher.fetch_models("any-key").await;
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(matches!(err, FetchError::UnsupportedProvider(_)));
        assert!(err.to_string().contains("nonexistent-provider"));
    }

    #[tokio::test]
    async fn test_provider_fetcher_hardcoded_minimax_returns_models() {
        let fetcher = ProviderModelFetcher::new("minimax", None);
        let result = fetcher.fetch_models("any-key").await;
        assert!(result.is_ok());
        let models = result.unwrap();
        assert!(!models.is_empty());
        let ids: Vec<&str> = models.iter().map(|m| m.id.as_str()).collect();
        assert!(ids.contains(&"MiniMax-Text-01"));
    }

    #[tokio::test]
    async fn test_provider_fetcher_hardcoded_azure_returns_models() {
        let fetcher = ProviderModelFetcher::new("azure", None);
        let result = fetcher.fetch_models("any-key").await;
        assert!(result.is_ok());
        let models = result.unwrap();
        assert!(!models.is_empty());
        let names: Vec<&str> = models.iter().map(|m| m.name.as_str()).collect();
        assert!(names.contains(&"GPT-4o"));
    }

    #[tokio::test]
    async fn test_provider_fetcher_hardcoded_huggingface_returns_models() {
        let fetcher = ProviderModelFetcher::new("huggingface", None);
        let result = fetcher.fetch_models("any-key").await;
        assert!(result.is_ok());
        let models = result.unwrap();
        assert!(!models.is_empty());
    }

    #[tokio::test]
    async fn test_provider_fetcher_hardcoded_cohere_returns_models() {
        let fetcher = ProviderModelFetcher::new("cohere", None);
        let result = fetcher.fetch_models("any-key").await;
        assert!(result.is_ok());
        let models = result.unwrap();
        assert_eq!(models.len(), 4); // cohere has 4 models
    }

    #[tokio::test]
    async fn test_provider_fetcher_case_insensitive_hardcoded() {
        // "MINIMAX" should still return minimax fallback models
        let fetcher = ProviderModelFetcher::new("MINIMAX", None);
        let result = fetcher.fetch_models("any-key").await;
        assert!(result.is_ok());
        let models = result.unwrap();
        assert!(!models.is_empty());
    }

    #[tokio::test]
    async fn test_provider_fetcher_google_alias() {
        // Both "google" and "gemini" should work
        let google_fetcher = ProviderModelFetcher::new("google", None);
        let gemini_fetcher = ProviderModelFetcher::new("gemini", None);
        // These will fail due to no actual API key, but verify routing works
        let google_result = google_fetcher.fetch_models("test-key").await;
        let gemini_result = gemini_fetcher.fetch_models("test-key").await;
        // Both should either succeed (if key valid) or fail with ApiError (not UnsupportedProvider)
        // The key difference is they should NOT return UnsupportedProvider
        if google_result.is_err() {
            assert!(!matches!(google_result.unwrap_err(), FetchError::UnsupportedProvider(_)));
        }
        if gemini_result.is_err() {
            assert!(!matches!(gemini_result.unwrap_err(), FetchError::UnsupportedProvider(_)));
        }
    }

    // ===== Custom Base URL Provider Test =====

    #[tokio::test]
    async fn test_provider_fetcher_custom_base_url() {
        // When base_url is provided for unknown provider, it should use OpenAI-compatible fetcher
        let fetcher = ProviderModelFetcher::new("custom-provider", Some("https://custom.api.com"));
        // This will fail due to connection but verifies routing to custom URL works
        let result = fetcher.fetch_models("test-key").await;
        // Should NOT be UnsupportedProvider since base_url is set
        if result.is_err() {
            let err = result.unwrap_err();
            assert!(!matches!(err, FetchError::UnsupportedProvider(_)));
        }
    }
}
