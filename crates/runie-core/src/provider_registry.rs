//! Provider registry — metadata for known LLM providers.
//!
//! This module is the single source of truth for provider names, display names,
//! base URLs, and default models shown in the login dialog.

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

/// API type for a provider — determines request format and validation endpoint.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProviderApiType {
    /// OpenAI Chat Completions API (most common)
    OpenAiCompatible,
}

/// Metadata for a known provider.
#[derive(Debug, Clone, PartialEq)]
pub struct ProviderMeta {
    pub key: &'static str,
    pub display_name: &'static str,
    pub base_url: &'static str,
    pub api_type: ProviderApiType,
    pub env_var: &'static str,
    pub default_models: &'static [&'static str],
}

impl ProviderMeta {
    pub const fn new(
        key: &'static str,
        display_name: &'static str,
        base_url: &'static str,
        env_var: &'static str,
        models: &'static [&'static str],
    ) -> Self {
        Self {
            key,
            display_name,
            base_url,
            api_type: ProviderApiType::OpenAiCompatible,
            env_var,
            default_models: models,
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
    &["echo"],
);

/// Real providers — the production-safe list. Mock is added conditionally
/// on top of this.
const REAL_PROVIDERS: &[ProviderMeta] = KNOWN_PROVIDERS;

/// Cached combined list (real + mock when enabled). The mock entry is
/// appended on first access if dev flags are set.
fn providers() -> &'static [ProviderMeta] {
    static CACHE: OnceLock<Vec<ProviderMeta>> = OnceLock::new();
    CACHE.get_or_init(|| {
        let mut v: Vec<ProviderMeta> = REAL_PROVIDERS.to_vec();
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

static KNOWN_PROVIDERS: &[ProviderMeta] = &[
    ProviderMeta::new(
        "anthropic",
        "Anthropic",
        "https://api.anthropic.com/v1",
        "ANTHROPIC_API_KEY",
        &["claude-sonnet-4-6", "claude-opus-4-7", "claude-haiku-4-5"],
    ),
    ProviderMeta::new(
        "openai",
        "OpenAI",
        "https://api.openai.com/v1",
        "OPENAI_API_KEY",
        &["gpt-4o", "gpt-4o-mini", "gpt-5", "o3-mini", "o4-mini"],
    ),
    ProviderMeta::new(
        "google",
        "Google Gemini",
        "https://generativelanguage.googleapis.com/v1beta",
        "GEMINI_API_KEY",
        &["gemini-2.5-pro", "gemini-2.5-flash", "gemini-2.0-flash"],
    ),
    ProviderMeta::new(
        "deepseek",
        "DeepSeek",
        "https://api.deepseek.com/v1",
        "DEEPSEEK_API_KEY",
        &["deepseek-v4-flash", "deepseek-v4-pro"],
    ),
    ProviderMeta::new(
        "openrouter",
        "OpenRouter",
        "https://openrouter.ai/api/v1",
        "OPENROUTER_API_KEY",
        &[
            "anthropic/claude-sonnet-4.6",
            "openai/gpt-4o",
            "google/gemini-2.5-pro",
        ],
    ),
    ProviderMeta::new(
        "groq",
        "Groq",
        "https://api.groq.com/openai/v1",
        "GROQ_API_KEY",
        &[
            "llama-3.3-70b-versatile",
            "gemma2-9b-it",
            "mixtral-8x7b-32768",
        ],
    ),
    ProviderMeta::new(
        "mistral",
        "Mistral",
        "https://api.mistral.ai/v1",
        "MISTRAL_API_KEY",
        &[
            "mistral-large-latest",
            "codestral-latest",
            "devstral-latest",
        ],
    ),
    ProviderMeta::new(
        "fireworks",
        "Fireworks",
        "https://api.fireworks.ai/inference/v1",
        "FIREWORKS_API_KEY",
        &[
            "accounts/fireworks/models/deepseek-v4-pro",
            "accounts/fireworks/models/kimi-k2p6",
        ],
    ),
    ProviderMeta::new(
        "together",
        "Together AI",
        "https://api.together.xyz/v1",
        "TOGETHER_API_KEY",
        &[
            "meta-llama/Llama-3.3-70B-Instruct-Turbo",
            "deepseek-ai/DeepSeek-V4-Pro",
        ],
    ),
    ProviderMeta::new(
        "minimax",
        "MiniMax",
        "https://api.minimaxi.chat/v1",
        "MINIMAX_API_KEY",
        &["MiniMax-M3", "MiniMax-M2.7"],
    ),
    ProviderMeta::new(
        "moonshotai",
        "Moonshot AI",
        "https://api.moonshot.cn/v1",
        "MOONSHOT_API_KEY",
        &["kimi-k2.5", "kimi-k2.6", "kimi-k2-thinking"],
    ),
    ProviderMeta::new(
        "xai",
        "xAI",
        "https://api.x.ai/v1",
        "XAI_API_KEY",
        &["grok-3", "grok-4.3"],
    ),
    ProviderMeta::new(
        "ollama",
        "Ollama (local)",
        "http://localhost:11434/v1",
        "OLLAMA_HOST",
        &["llama3.1", "qwen2.5-coder:7b", "mistral"],
    ),
];

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
        assert_eq!(p.default_models, &["MiniMax-M3", "MiniMax-M2.7"]);
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
                !p.default_models.is_empty(),
                "Provider {} should have default models",
                p.key
            );
        }
    }
}
