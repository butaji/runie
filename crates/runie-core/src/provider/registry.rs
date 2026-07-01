//! Provider registry — metadata for known LLM providers.
//!
//! This module is the single source of truth for provider names, display names,
//! base URLs, API type, environment variable, and the models each provider supports.
//!
//! Provider and model metadata is loaded from YAML files in `resources/models/`.

use super::registry_data::{
    mock_provider_yaml, parse_provider_yaml, provider_yaml_files, ProviderYaml,
};
use derive_builder::Builder;
use std::cell::RefCell;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::OnceLock;

static MOCK_ENABLED: AtomicBool = AtomicBool::new(false);
thread_local! {
    /// Per-thread override for `is_mock_enabled` in tests. Allows tests to
    /// set deterministic mock state without interfering with parallel tests.
    static TEST_MOCK: RefCell<Option<bool>> = const { RefCell::new(None) };
}
static PROVIDER_CACHE: OnceLock<Vec<ProviderMeta>> = OnceLock::new();

/// Returns true when dev flags enable the mock provider. Without this,
/// the app is production-ready: no silent mock fallback, the mock provider
/// is NOT listed in the picker, and `ConfigState` does NOT default to it.
///
/// `dev.sh` sets `RUNIE_MOCK=1`. `RUNIE_MOCK_DELAY=1` is also accepted as
/// a back-compat alias (it both enables the mock and adds streaming delays).
pub fn is_mock_enabled() -> bool {
    // Thread-local override takes precedence (set by `set_mock_enabled` in tests).
    if let Some(v) = TEST_MOCK.with(|cell| *cell.borrow()) {
        return v;
    }
    MOCK_ENABLED.load(Ordering::Relaxed)
        || std::env::var_os("RUNIE_MOCK").is_some()
        || std::env::var_os("RUNIE_MOCK_DELAY").is_some()
}

/// Override the mock-enabled state without touching environment variables.
/// Primarily useful in tests that need deterministic mock behavior.
///
/// The thread-local override takes precedence over the global atomic and
/// environment variables, ensuring test parallelism safety.
pub fn set_mock_enabled(enabled: bool) {
    TEST_MOCK.with(|cell| *cell.borrow_mut() = Some(enabled));
    MOCK_ENABLED.store(enabled, Ordering::Relaxed);
}

/// Metadata for a model supported by a provider.
#[derive(Debug, Clone, PartialEq, Builder)]
#[builder(setter(strip_option))]
pub struct ModelMeta {
    pub name: String,
    pub cost_prompt: Option<f64>,
    pub cost_completion: Option<f64>,
    pub supports_thinking: bool,
    pub supports_vision: bool,
    pub tokenizer: Option<String>,
    pub context_window: Option<usize>,
    pub streaming: bool,
    pub supports_tools: bool,
    pub supports_reasoning: bool,
    pub supports_system: bool,
    pub max_output_tokens: usize,
    pub cache_control: bool,
}

impl ModelMeta {
    pub fn from_yaml(yaml: &super::registry_data::ModelYaml) -> Self {
        Self {
            name: yaml.name.clone(),
            cost_prompt: yaml.cost_prompt,
            cost_completion: yaml.cost_completion,
            supports_thinking: yaml.supports_thinking,
            supports_vision: yaml.supports_vision,
            tokenizer: yaml.tokenizer.clone(),
            context_window: yaml.context_window,
            streaming: yaml.streaming,
            supports_tools: yaml.supports_tools,
            supports_reasoning: yaml.supports_reasoning,
            supports_system: yaml.supports_system,
            max_output_tokens: yaml.max_output_tokens,
            cache_control: yaml.cache_control,
        }
    }
}

/// Metadata for a known provider.
#[derive(Debug, Clone, PartialEq, Builder)]
#[builder(setter(strip_option))]
pub struct ProviderMeta {
    pub key: String,
    pub display_name: String,
    pub base_url: String,
    pub env_var: String,
    pub models: Vec<ModelMeta>,
}

impl ProviderMeta {
    pub fn from_yaml(yaml: &ProviderYaml) -> Self {
        Self {
            key: yaml.key.clone(),
            display_name: yaml.display_name.clone(),
            base_url: yaml.base_url.clone(),
            env_var: yaml.env_var.clone(),
            models: yaml.models.iter().map(ModelMeta::from_yaml).collect(),
        }
    }
}

/// Load and cache all providers from YAML files.
fn load_providers() -> Vec<ProviderMeta> {
    provider_yaml_files()
        .iter()
        .map(|(_, yaml)| {
            let parsed = parse_provider_yaml(yaml).expect("Failed to parse embedded YAML");
            ProviderMeta::from_yaml(&parsed)
        })
        .collect()
}

/// Get the cached list of all known providers.
fn get_providers() -> &'static [ProviderMeta] {
    PROVIDER_CACHE.get_or_init(load_providers).as_slice()
}

/// Mock provider (dev-only).
fn mock_provider() -> ProviderMeta {
    ProviderMeta::from_yaml(&mock_provider_yaml())
}

/// All known providers. In production (no `RUNIE_MOCK`), this is the
/// real provider list only. With dev flags, the mock provider is
/// appended at the end.
pub fn known_providers() -> Vec<ProviderMeta> {
    let mut providers = get_providers().to_vec();
    if is_mock_enabled() {
        providers.push(mock_provider());
    }
    providers
}

/// Find a provider by its key (e.g. "minimax").
pub fn find_provider(key: &str) -> Option<ProviderMeta> {
    let providers = get_providers();
    if key == "mock" && is_mock_enabled() {
        return Some(mock_provider());
    }
    providers.iter().find(|p| p.key == key).cloned()
}

/// Find a provider by its environment variable name.
pub fn find_provider_by_env_var(env_var: &str) -> Option<ProviderMeta> {
    get_providers()
        .iter()
        .find(|p| p.env_var == env_var)
        .cloned()
}

/// Find a model across all known providers by its canonical model name.
pub fn find_model(model: &str) -> Option<ModelMeta> {
    let providers = get_providers();
    providers
        .iter()
        .flat_map(|p| p.models.iter())
        .find(|m| m.name == model)
        .cloned()
        .or_else(|| {
            if is_mock_enabled() {
                let mock = mock_provider();
                mock.models.into_iter().find(|m| m.name == model)
            } else {
                None
            }
        })
}

/// Check if a provider key is known.
pub fn is_known_provider(key: &str) -> bool {
    find_provider(key).is_some()
}

/// Get the display name for a provider key, or the key itself if unknown.
pub fn display_name(key: &str) -> String {
    find_provider(key)
        .map(|p| p.display_name.clone())
        .unwrap_or_else(|| key.to_owned())
}

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
            p.models.iter().map(|m| m.name.as_str()).collect::<Vec<_>>(),
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
        for p in get_providers() {
            assert!(
                p.base_url.starts_with("http"),
                "Provider {} should have valid base URL",
                p.key
            );
        }
    }

    #[test]
    fn provider_registry_all_have_models() {
        for p in get_providers() {
            assert!(
                !p.models.is_empty(),
                "Provider {} should have models",
                p.key
            );
        }
    }

    #[test]
    fn provider_registry_model_names_unique_per_provider() {
        for p in get_providers() {
            let mut names: Vec<_> = p.models.iter().map(|m| m.name.clone()).collect();
            let before = names.len();
            names.sort_unstable();
            names.dedup();
            assert_eq!(names.len(), before, "Provider {} has duplicates", p.key);
        }
    }

    #[test]
    fn openrouter_model_matches_canonical() {
        let openrouter = find_provider("openrouter").expect("openrouter should exist");
        for model in &openrouter.models {
            if let Some((provider_key, base_name)) = model.name.split_once('/') {
                if let Some(provider) = find_provider(provider_key) {
                    if let Some(m) = provider.models.iter().find(|m| m.name == base_name) {
                        assert_eq!(
                            m.supports_thinking, model.supports_thinking,
                            "thinking mismatch for {}/{}",
                            provider_key, base_name
                        );
                        assert_eq!(
                            m.supports_vision, model.supports_vision,
                            "vision mismatch for {}/{}",
                            provider_key, base_name
                        );
                        assert_eq!(
                            m.context_window, model.context_window,
                            "context_window mismatch for {}/{}",
                            provider_key, base_name
                        );
                    }
                }
            }
        }
    }

    #[test]
    fn context_window_comes_from_registry() {
        for p in get_providers() {
            if p.key == "mock" {
                continue;
            }
            for model in &p.models {
                assert!(
                    model.context_window.is_some(),
                    "Provider {} model {} needs context window",
                    p.key,
                    model.name
                );
            }
        }
        assert_eq!(
            find_provider("openai")
                .and_then(|p| p.models.into_iter().find(|m| m.name == "gpt-4o"))
                .and_then(|m| m.context_window),
            Some(128_000)
        );
        assert_eq!(
            find_provider("anthropic")
                .and_then(|p| p.models.into_iter().find(|m| m.name == "claude-sonnet-4-6"))
                .and_then(|m| m.context_window),
            Some(200_000)
        );
    }

    #[test]
    fn mock_provider_available_when_enabled() {
        set_mock_enabled(true);
        let providers = known_providers();
        assert!(providers.iter().any(|p| p.key == "mock"));
        set_mock_enabled(false);
        let providers = known_providers();
        assert!(!providers.iter().any(|p| p.key == "mock"));
    }
}
