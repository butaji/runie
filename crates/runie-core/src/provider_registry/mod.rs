//! Provider registry — metadata for known LLM providers.
//!
//! This module is the single source of truth for provider names, display names,
//! base URLs, API type, environment variable, and the models each provider supports.

use std::sync::OnceLock;

/// Returns true when dev flags enable the mock provider. Without this,
/// the app is production-ready: no silent mock fallback, the mock provider
/// is NOT listed in the picker, and `ConfigState` does NOT default to it.
///
/// `dev.sh` sets `RUNIE_MOCK=1`. `RUNIE_MOCK_DELAY=1` is also accepted as
/// a back-compat alias (it both enables the mock and adds streaming delays).
pub fn is_mock_enabled() -> bool {
    std::env::var_os("RUNIE_MOCK").is_some() || std::env::var_os("RUNIE_MOCK_DELAY").is_some()
}

/// Metadata for a model supported by a provider.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ModelMeta {
    pub name: &'static str,
    pub cost_prompt: Option<f64>,
    pub cost_completion: Option<f64>,
    pub supports_thinking: bool,
    pub supports_vision: bool,
    pub tokenizer: Option<&'static str>,
    pub context_window: Option<usize>,
    pub streaming: bool,
    pub supports_tools: bool,
    pub supports_reasoning: bool,
    pub supports_system: bool,
    pub max_output_tokens: usize,
    pub cache_control: bool,
}

impl ModelMeta {
    pub const fn new(name: &'static str) -> Self {
        Self {
            name,
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
        }
    }

    pub const fn with_cost(self, prompt: f64, completion: f64) -> Self {
        Self {
            name: self.name,
            cost_prompt: Some(prompt),
            cost_completion: Some(completion),
            supports_thinking: self.supports_thinking,
            supports_vision: self.supports_vision,
            tokenizer: self.tokenizer,
            context_window: self.context_window,
            streaming: self.streaming,
            supports_tools: self.supports_tools,
            supports_reasoning: self.supports_reasoning,
            supports_system: self.supports_system,
            max_output_tokens: self.max_output_tokens,
            cache_control: self.cache_control,
        }
    }

    pub const fn with_thinking(self) -> Self {
        Self {
            name: self.name,
            cost_prompt: self.cost_prompt,
            cost_completion: self.cost_completion,
            supports_thinking: true,
            supports_vision: self.supports_vision,
            tokenizer: self.tokenizer,
            context_window: self.context_window,
            streaming: self.streaming,
            supports_tools: self.supports_tools,
            supports_reasoning: self.supports_reasoning,
            supports_system: self.supports_system,
            max_output_tokens: self.max_output_tokens,
            cache_control: self.cache_control,
        }
    }

    pub const fn with_vision(self) -> Self {
        Self {
            name: self.name,
            cost_prompt: self.cost_prompt,
            cost_completion: self.cost_completion,
            supports_thinking: self.supports_thinking,
            supports_vision: true,
            tokenizer: self.tokenizer,
            context_window: self.context_window,
            streaming: self.streaming,
            supports_tools: self.supports_tools,
            supports_reasoning: self.supports_reasoning,
            supports_system: self.supports_system,
            max_output_tokens: self.max_output_tokens,
            cache_control: self.cache_control,
        }
    }

    pub const fn with_tokenizer(self, tokenizer: &'static str) -> Self {
        Self {
            name: self.name,
            cost_prompt: self.cost_prompt,
            cost_completion: self.cost_completion,
            supports_thinking: self.supports_thinking,
            supports_vision: self.supports_vision,
            tokenizer: Some(tokenizer),
            context_window: self.context_window,
            streaming: self.streaming,
            supports_tools: self.supports_tools,
            supports_reasoning: self.supports_reasoning,
            supports_system: self.supports_system,
            max_output_tokens: self.max_output_tokens,
            cache_control: self.cache_control,
        }
    }

    pub const fn with_context_window(self, context_window: usize) -> Self {
        Self {
            name: self.name,
            cost_prompt: self.cost_prompt,
            cost_completion: self.cost_completion,
            supports_thinking: self.supports_thinking,
            supports_vision: self.supports_vision,
            tokenizer: self.tokenizer,
            context_window: Some(context_window),
            streaming: self.streaming,
            supports_tools: self.supports_tools,
            supports_reasoning: self.supports_reasoning,
            supports_system: self.supports_system,
            max_output_tokens: self.max_output_tokens,
            cache_control: self.cache_control,
        }
    }

    pub const fn with_streaming(self, streaming: bool) -> Self {
        Self { streaming, ..self }
    }

    pub const fn with_tools(self, supports_tools: bool) -> Self {
        Self {
            supports_tools,
            ..self
        }
    }

    pub const fn with_reasoning(self) -> Self {
        Self {
            supports_reasoning: true,
            ..self
        }
    }

    pub const fn with_output_limit(self, max_output_tokens: usize) -> Self {
        Self {
            max_output_tokens,
            ..self
        }
    }

    pub const fn with_cache_control(self) -> Self {
        Self {
            cache_control: true,
            ..self
        }
    }

    pub const fn with_no_system(self) -> Self {
        Self {
            supports_system: false,
            ..self
        }
    }
}

/// Metadata for a known provider.
#[derive(Debug, Clone, PartialEq)]
pub struct ProviderMeta {
    pub key: &'static str,
    pub display_name: &'static str,
    pub base_url: &'static str,
    pub env_var: &'static str,
    pub models: &'static [ModelMeta],
}

impl ProviderMeta {
    pub const fn new(
        key: &'static str,
        display_name: &'static str,
        base_url: &'static str,
        env_var: &'static str,
        models: &'static [ModelMeta],
    ) -> Self {
        Self {
            key,
            display_name,
            base_url,
            env_var,
            models,
        }
    }
}

/// Mock provider entry — included in `known_providers()` only when
/// `is_mock_enabled()` returns true.
const MOCK_PROVIDER: ProviderMeta = ProviderMeta::new(
    "mock",
    "Mock (dev only)",
    "http://localhost/mock",
    "",
    &[ModelMeta::new("echo")],
);

/// Cached combined list (real + mock when enabled). The mock entry is
/// appended on first access if dev flags are set.
fn providers() -> &'static [ProviderMeta] {
    static CACHE: OnceLock<Vec<ProviderMeta>> = OnceLock::new();
    CACHE.get_or_init(|| {
        let mut v: Vec<ProviderMeta> = KNOWN_PROVIDERS.to_vec();
        if is_mock_enabled() {
            v.push(MOCK_PROVIDER);
        }
        v
    })
}

/// All known providers. In production (no `RUNIE_MOCK`), this is the
/// real provider list only. With dev flags, the mock provider is
/// appended at the end.
pub fn known_providers() -> &'static [ProviderMeta] {
    providers()
}

/// Find a provider by its key (e.g. "minimax").
pub fn find_provider(key: &str) -> Option<&'static ProviderMeta> {
    known_providers().iter().find(|p| p.key == key)
}

/// Find a provider by its environment variable name.
pub fn find_provider_by_env_var(env_var: &str) -> Option<&'static ProviderMeta> {
    known_providers().iter().find(|p| p.env_var == env_var)
}

/// Find a model across all known providers by its canonical model name.
pub fn find_model(model: &str) -> Option<&'static ModelMeta> {
    known_providers()
        .iter()
        .flat_map(|p| p.models.iter())
        .find(|m| m.name == model)
}

/// Check if a provider key is known.
pub fn is_known_provider(key: &str) -> bool {
    find_provider(key).is_some()
}

/// Get the display name for a provider key, or the key itself if unknown.
pub fn display_name(key: &str) -> String {
    find_provider(key)
        .map(|p| p.display_name.to_string())
        .unwrap_or_else(|| key.to_string())
}

mod data;
pub(crate) use data::KNOWN_PROVIDERS;

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn provider_registry_lists_known_providers() {
        let providers = known_providers();
        assert!(!providers.is_empty(), "Registry should not be empty");
        // Spot-check well-known providers
        assert!(providers.iter().any(|p| p.key == "anthropic"));
        assert!(providers.iter().any(|p| p.key == "openai"));
        assert!(providers.iter().any(|p| p.key == "minimax"));
    }

    #[test]
    fn provider_registry_find_by_name() {
        let p = find_provider("minimax").expect("minimax should exist");
        assert_eq!(p.display_name, "MiniMax");
        assert_eq!(p.base_url, "https://api.minimaxi.chat/v1");
        assert_eq!(p.env_var, "MINIMAX_API_KEY");
        assert_eq!(
            p.models.iter().map(|m| m.name).collect::<Vec<_>>(),
            vec!["MiniMax-M3", "MiniMax-M2.7"]
        );
    }

    #[test]
    fn provider_registry_find_missing_returns_none() {
        assert!(find_provider("nonexistent").is_none());
    }

    #[test]
    fn provider_registry_find_by_env_var() {
        let p = find_provider_by_env_var("MINIMAX_API_KEY").expect("should find by env var");
        assert_eq!(p.key, "minimax");
    }

    #[test]
    fn provider_registry_is_known() {
        assert!(is_known_provider("openai"));
        assert!(!is_known_provider("fake-provider"));
    }

    #[test]
    fn provider_registry_display_name_known() {
        assert_eq!(display_name("minimax"), "MiniMax");
    }

    #[test]
    fn provider_registry_display_name_unknown_fallback() {
        assert_eq!(display_name("custom"), "custom");
    }

    #[test]
    fn provider_registry_all_have_base_url() {
        for p in known_providers() {
            assert!(
                p.base_url.starts_with("http"),
                "Provider {} should have a valid base URL, got {}",
                p.key,
                p.base_url
            );
        }
    }

    #[test]
    fn provider_registry_all_have_models() {
        for p in known_providers() {
            assert!(
                !p.models.is_empty(),
                "Provider {} should have default models",
                p.key
            );
        }
    }

    #[test]
    fn provider_registry_model_names_are_unique_per_provider() {
        for p in known_providers() {
            let mut names: Vec<_> = p.models.iter().map(|m| m.name).collect();
            let before = names.len();
            names.sort_unstable();
            names.dedup();
            assert_eq!(
                names.len(),
                before,
                "Provider {} has duplicate model names",
                p.key
            );
        }
    }

    fn canonical_for_openrouter(model: &ModelMeta) -> &'static ModelMeta {
        let (provider_key, base_name) = model
            .name
            .split_once('/')
            .expect("OpenRouter model should be provider/name");
        let provider = find_provider(provider_key).expect("Canonical provider should exist");
        if let Some(m) = provider.models.iter().find(|m| m.name == base_name) {
            return m;
        }
        provider
            .models
            .iter()
            .find(|m| capabilities_match(m, model))
            .expect("Canonical model should exist")
    }

    fn capabilities_match(a: &ModelMeta, b: &ModelMeta) -> bool {
        a.supports_thinking == b.supports_thinking
            && a.supports_vision == b.supports_vision
            && a.context_window == b.context_window
    }

    #[test]
    fn openrouter_model_matches_canonical() {
        let openrouter = find_provider("openrouter").expect("openrouter should exist");
        for model in openrouter.models {
            let canonical = canonical_for_openrouter(model);
            assert!(
                capabilities_match(model, canonical),
                "capabilities mismatch for {}",
                model.name
            );
        }
    }

    #[test]
    fn context_window_comes_from_registry() {
        for p in known_providers() {
            if p.key == "mock" {
                continue;
            }
            for model in p.models {
                assert!(
                    model.context_window.is_some(),
                    "Provider {} model {} should have a context window",
                    p.key,
                    model.name
                );
            }
        }
        assert_eq!(
            find_provider("openai")
                .and_then(|p| p.models.iter().find(|m| m.name == "gpt-4o"))
                .and_then(|m| m.context_window),
            Some(128_000)
        );
        assert_eq!(
            find_provider("anthropic")
                .and_then(|p| p.models.iter().find(|m| m.name == "claude-sonnet-4-6"))
                .and_then(|m| m.context_window),
            Some(200_000)
        );
    }
}
