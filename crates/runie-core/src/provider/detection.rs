//! Provider and model detection utilities.
//!
//! Provides utilities for parsing provider/model strings and detecting the
//! provider from API base URLs.
#![allow(clippy::too_many_lines)]

/// Known API base URLs for supported providers.
///
/// Maps provider keys to their canonical API base URLs.
pub const KNOWN_API_BASES: &[(&str, &str)] = &[
    ("anthropic", "https://api.anthropic.com/v1"),
    ("openai", "https://api.openai.com/v1"),
    ("deepseek", "https://api.deepseek.com/v1"),
    (
        "google",
        "https://generativelanguage.googleapis.com/v1beta/openai",
    ),
    ("minimax", "https://api.minimaxi.chat/v1"),
    ("moonshotai", "https://api.moonshotai.com/v1"),
    ("mistral", "https://api.mistral.ai/v1"),
    ("groq", "https://api.groq.com/openai/v1"),
    ("fireworks", "https://api.fireworks.ai/v1"),
    ("together", "https://api.together.xyz/v1"),
    ("kimi-code", "https://api.kimi.com/coding/v1"),
    ("opencode", "https://opencode.ai/zen/v1"),
    ("xai", "https://api.x.ai/v1"),
    ("ollama", "http://localhost:11434/v1"),
    ("openrouter", "https://openrouter.ai/api/v1"),
    ("azure_openai", "https://{resource}.openai.azure.com/v1"),
];

/// Parses a model string that may contain a provider prefix.
///
/// The format is `provider/model` where the slash separates the provider key
/// from the model name.
///
/// # Examples
///
/// ```
/// use runie_core::provider::detection::parse_model_prefix;
///
/// // With prefix
/// assert_eq!(parse_model_prefix("azure/gpt-4"), (Some("azure"), "gpt-4"));
/// assert_eq!(parse_model_prefix("openai/gpt-4o"), (Some("openai"), "gpt-4o"));
/// assert_eq!(parse_model_prefix("anthropic/claude-sonnet-4-6"), (Some("anthropic"), "claude-sonnet-4-6"));
///
/// // Without prefix
/// assert_eq!(parse_model_prefix("gpt-4o"), (None, "gpt-4o"));
/// assert_eq!(parse_model_prefix("claude-sonnet-4-6"), (None, "claude-sonnet-4-6"));
///
/// // OpenRouter-style (model names contain slashes)
/// assert_eq!(parse_model_prefix("anthropic/claude-sonnet-4-6"), (Some("anthropic"), "claude-sonnet-4-6"));
/// ```
pub fn parse_model_prefix(model: &str) -> (Option<&str>, &str) {
    if let Some((provider, model_name)) = model.split_once('/') {
        // Only treat as provider prefix if the first part looks like a known provider key
        // (alphanumeric with underscores, typical provider names)
        if is_valid_provider_key(provider) {
            return (Some(provider), model_name);
        }
    }
    (None, model)
}

/// Checks if a string looks like a valid provider key.
/// Provider keys are typically lowercase alphanumeric with underscores.
fn is_valid_provider_key(s: &str) -> bool {
    !s.is_empty()
        && s.len() <= 32
        && s.chars()
            .all(|c| c.is_alphanumeric() || c == '_' || c == '-')
}

/// Detects the provider key from an API base URL.
///
/// Returns the provider key if the URL matches a known provider pattern,
/// or `None` if no match is found.
///
/// # Examples
///
/// ```
/// use runie_core::provider::detection::detect_provider_from_api_base;
///
/// assert_eq!(detect_provider_from_api_base("https://api.anthropic.com/v1"), Some("anthropic"));
/// assert_eq!(detect_provider_from_api_base("https://api.openai.com/v1"), Some("openai"));
/// assert_eq!(detect_provider_from_api_base("https://api.deepseek.com/v1"), Some("deepseek"));
/// assert_eq!(detect_provider_from_api_base("https://generativelanguage.googleapis.com/v1beta/openai"), Some("google"));
/// assert_eq!(detect_provider_from_api_base("https://api.minimaxi.chat/v1"), Some("minimax"));
/// assert_eq!(detect_provider_from_api_base("http://localhost:11434/v1"), Some("ollama"));
///
/// // Azure OpenAI
/// assert_eq!(detect_provider_from_api_base("https://my-resource.openai.azure.com"), Some("azure_openai"));
/// assert_eq!(detect_provider_from_api_base("https://my-resource.openai.azure.com/v1"), Some("azure_openai"));
/// ```
#[allow(clippy::cognitive_complexity)]
pub fn detect_provider_from_api_base(api_base: &str) -> Option<&'static str> {
    let normalized = api_base.trim_end_matches('/').to_lowercase();

    // Check for Azure OpenAI first (more specific pattern)
    if normalized.contains("openai.azure.com")
        || normalized.contains(".azure.com/openai")
        || normalized.contains(".microsoft.com/openai")
    {
        return Some("azure_openai");
    }

    // Check each known provider pattern
    if normalized.contains("api.anthropic.com") {
        Some("anthropic")
    } else if normalized.contains("api.openai.com") && !normalized.contains("azure") {
        Some("openai")
    } else if normalized.contains("api.deepseek.com") {
        Some("deepseek")
    } else if normalized.contains("generativelanguage.googleapis.com") {
        Some("google")
    } else if normalized.contains("api.minimaxi.chat") {
        Some("minimax")
    } else if normalized.contains("api.moonshot.cn") || normalized.contains("api.moonshotai.com") {
        Some("moonshotai")
    } else if normalized.contains("api.mistral.ai") {
        Some("mistral")
    } else if normalized.contains("api.groq.com") {
        Some("groq")
    } else if normalized.contains("api.fireworks.ai") {
        Some("fireworks")
    } else if normalized.contains("api.together.xyz") {
        Some("together")
    } else if normalized.contains("api.kimi.com") || normalized.contains("api.kimi.cn") {
        Some("kimi-code")
    } else if normalized.contains("opencode.ai") {
        Some("opencode")
    } else if normalized.contains("api.x.ai") {
        Some("xai")
    } else if normalized.contains("localhost:11434") || normalized.contains("ollama") {
        Some("ollama")
    } else if normalized.contains("openrouter.ai") {
        Some("openrouter")
    } else {
        None
    }
}

/// Detects the provider from a URL.
///
/// Short alias for [`detect_provider_from_api_base`].
///
/// # Examples
///
/// ```
/// use runie_core::provider::detection::detect_provider;
///
/// assert_eq!(detect_provider("https://api.anthropic.com/v1"), Some("anthropic"));
/// assert_eq!(detect_provider("https://api.openai.com/v1"), Some("openai"));
/// assert_eq!(detect_provider("https://my-resource.openai.azure.com"), Some("azure_openai"));
/// ```
pub fn detect_provider(url: &str) -> Option<&'static str> {
    detect_provider_from_api_base(url)
}

/// Check if a provider name is a known/registered provider.
pub fn is_known_provider(name: &str) -> bool {
    matches!(
        name,
        "openai"
            | "anthropic"
            | "google"
            | "azure"
            | "groq"
            | "together"
            | "deepinfra"
            | "mistral"
            | "cohere"
            | "fireworks"
            | "perplexity"
            | "ollama"
            | "mock"
    )
}

/// Normalize a model name to its canonical form with provider prefix.
///
/// If the model already has a provider prefix, returns it unchanged.
/// If the model has no prefix and a default provider is given, adds the prefix.
/// If the model has no prefix and no default is given, returns it unchanged.
pub fn normalize_model_name(model: &str, default_provider: Option<&str>) -> String {
    let (prefix, rest) = parse_model_prefix(model);
    match (prefix, default_provider) {
        (Some(p), _) => format!("{p}/{rest}"),
        (None, Some(dp)) => format!("{dp}/{rest}"),
        (None, None) => rest.to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // ========================================================================
    // parse_model_prefix tests
    // ========================================================================

    #[test]
    fn parse_model_prefix_with_provider_prefix() {
        assert_eq!(parse_model_prefix("azure/gpt-4"), (Some("azure"), "gpt-4"));
        assert_eq!(
            parse_model_prefix("openai/gpt-4o"),
            (Some("openai"), "gpt-4o")
        );
        assert_eq!(
            parse_model_prefix("anthropic/claude-sonnet-4-6"),
            (Some("anthropic"), "claude-sonnet-4-6")
        );
        assert_eq!(
            parse_model_prefix("deepseek/deepseek-chat"),
            (Some("deepseek"), "deepseek-chat")
        );
    }

    #[test]
    fn parse_model_prefix_without_prefix() {
        assert_eq!(parse_model_prefix("gpt-4o"), (None, "gpt-4o"));
        assert_eq!(
            parse_model_prefix("claude-sonnet-4-6"),
            (None, "claude-sonnet-4-6")
        );
        assert_eq!(parse_model_prefix("deepseek-chat"), (None, "deepseek-chat"));
    }

    #[test]
    fn parse_model_prefix_openrouter_style() {
        // OpenRouter models use provider/model format
        assert_eq!(
            parse_model_prefix("anthropic/claude-sonnet-4-6"),
            (Some("anthropic"), "claude-sonnet-4-6")
        );
        assert_eq!(
            parse_model_prefix("openai/gpt-4o"),
            (Some("openai"), "gpt-4o")
        );
        assert_eq!(
            parse_model_prefix("google/gemini-2.5-pro"),
            (Some("google"), "gemini-2.5-pro")
        );
    }

    #[test]
    fn parse_model_prefix_with_underscore_provider() {
        assert_eq!(
            parse_model_prefix("kimi_code/model-name"),
            (Some("kimi_code"), "model-name")
        );
    }

    #[test]
    fn parse_model_prefix_edge_cases() {
        // Empty string
        assert_eq!(parse_model_prefix(""), (None, ""));

        // Single character (valid provider key)
        assert_eq!(parse_model_prefix("a/model"), (Some("a"), "model"));

        // Provider with numbers
        assert_eq!(
            parse_model_prefix("openai2/test"),
            (Some("openai2"), "test")
        );

        // Dashes in provider name
        assert_eq!(
            parse_model_prefix("my-provider/model"),
            (Some("my-provider"), "model")
        );
    }

    #[test]
    fn is_valid_provider_key_valid() {
        assert!(is_valid_provider_key("openai"));
        assert!(is_valid_provider_key("anthropic"));
        assert!(is_valid_provider_key("kimi_code"));
        assert!(is_valid_provider_key("a"));
        assert!(is_valid_provider_key("my-provider"));
    }

    #[test]
    fn is_valid_provider_key_invalid() {
        assert!(!is_valid_provider_key(""));
        assert!(!is_valid_provider_key("has space"));
        assert!(!is_valid_provider_key("has.dot"));
        assert!(!is_valid_provider_key("has/slash"));
    }

    // ========================================================================
    // detect_provider_from_api_base tests
    // ========================================================================

    #[test]
    fn detect_anthropic() {
        assert_eq!(
            detect_provider_from_api_base("https://api.anthropic.com/v1"),
            Some("anthropic")
        );
        assert_eq!(
            detect_provider_from_api_base("http://api.anthropic.com/v1"),
            Some("anthropic")
        );
    }

    #[test]
    fn detect_openai() {
        assert_eq!(
            detect_provider_from_api_base("https://api.openai.com/v1"),
            Some("openai")
        );
        assert_eq!(
            detect_provider_from_api_base("https://api.openai.com/v1/chat/completions"),
            Some("openai")
        );
    }

    #[test]
    fn detect_azure_openai() {
        assert_eq!(
            detect_provider_from_api_base("https://my-resource.openai.azure.com"),
            Some("azure_openai")
        );
        assert_eq!(
            detect_provider_from_api_base("https://my-resource.openai.azure.com/v1"),
            Some("azure_openai")
        );
        assert_eq!(
            detect_provider_from_api_base("https://account.openai.azure.com/openai/deployments/gpt-4"),
            Some("azure_openai")
        );
    }

    #[test]
    fn detect_deepseek() {
        assert_eq!(
            detect_provider_from_api_base("https://api.deepseek.com/v1"),
            Some("deepseek")
        );
    }

    #[test]
    fn detect_google() {
        assert_eq!(
            detect_provider_from_api_base("https://generativelanguage.googleapis.com/v1beta/openai"),
            Some("google")
        );
    }

    #[test]
    fn detect_minimax() {
        assert_eq!(
            detect_provider_from_api_base("https://api.minimaxi.chat/v1"),
            Some("minimax")
        );
    }

    #[test]
    fn detect_moonshotai() {
        assert_eq!(
            detect_provider_from_api_base("https://api.moonshot.cn/v1"),
            Some("moonshotai")
        );
        assert_eq!(
            detect_provider_from_api_base("https://api.moonshotai.com/v1"),
            Some("moonshotai")
        );
    }

    #[test]
    fn detect_mistral() {
        assert_eq!(
            detect_provider_from_api_base("https://api.mistral.ai/v1"),
            Some("mistral")
        );
    }

    #[test]
    fn detect_groq() {
        assert_eq!(
            detect_provider_from_api_base("https://api.groq.com/openai/v1"),
            Some("groq")
        );
    }

    #[test]
    fn detect_fireworks() {
        assert_eq!(
            detect_provider_from_api_base("https://api.fireworks.ai/v1"),
            Some("fireworks")
        );
    }

    #[test]
    fn detect_together() {
        assert_eq!(
            detect_provider_from_api_base("https://api.together.xyz/v1"),
            Some("together")
        );
    }

    #[test]
    fn detect_kimi_code() {
        assert_eq!(
            detect_provider_from_api_base("https://api.kimi.com/coding/v1"),
            Some("kimi-code")
        );
        assert_eq!(
            detect_provider_from_api_base("https://api.kimi.cn/coding/v1"),
            Some("kimi-code")
        );
    }

    #[test]
    fn detect_opencode() {
        assert_eq!(
            detect_provider_from_api_base("https://opencode.ai/zen/v1"),
            Some("opencode")
        );
    }

    #[test]
    fn detect_xai() {
        assert_eq!(
            detect_provider_from_api_base("https://api.x.ai/v1"),
            Some("xai")
        );
    }

    #[test]
    fn detect_ollama() {
        assert_eq!(
            detect_provider_from_api_base("http://localhost:11434/v1"),
            Some("ollama")
        );
        assert_eq!(
            detect_provider_from_api_base("https://ollama.example.com/v1"),
            Some("ollama")
        );
    }

    #[test]
    fn detect_openrouter() {
        assert_eq!(
            detect_provider_from_api_base("https://openrouter.ai/api/v1"),
            Some("openrouter")
        );
    }

    #[test]
    fn detect_provider_trailing_slash() {
        assert_eq!(
            detect_provider_from_api_base("https://api.openai.com/v1/"),
            Some("openai")
        );
        assert_eq!(
            detect_provider_from_api_base("https://api.anthropic.com/v1/"),
            Some("anthropic")
        );
    }

    #[test]
    fn detect_provider_unknown() {
        assert_eq!(
            detect_provider_from_api_base("https://unknown.api.example.com/v1"),
            None
        );
        assert_eq!(
            detect_provider_from_api_base("https://my-custom-endpoint.com"),
            None
        );
    }

    #[test]
    fn detect_provider_case_insensitive() {
        assert_eq!(
            detect_provider_from_api_base("https://API.OPENAI.COM/v1"),
            Some("openai")
        );
        assert_eq!(
            detect_provider_from_api_base("https://Api.Anthropic.Com/v1"),
            Some("anthropic")
        );
    }

    #[test]
    fn roundtrip_parse_and_detect() {
        // Test that parsing a provider/model string and then detecting the
        // provider gives consistent results for common patterns.
        let cases =
            vec![("azure/gpt-4", "azure"), ("openai/gpt-4o", "openai"), ("anthropic/claude-sonnet-4-6", "anthropic")];

        for (model, expected_provider) in cases {
            let (prefix, _) = parse_model_prefix(model);
            assert_eq!(
                prefix,
                Some(expected_provider),
                "parse_model_prefix failed for {model}"
            );
        }
    }

    // ========================================================================
    // detect_provider tests (alias function)
    // ========================================================================

    #[test]
    fn detect_provider_alias_works() {
        // Test that detect_provider is an alias for detect_provider_from_api_base
        assert_eq!(
            detect_provider("https://api.anthropic.com/v1"),
            detect_provider_from_api_base("https://api.anthropic.com/v1")
        );
        assert_eq!(
            detect_provider("https://api.openai.com/v1"),
            detect_provider_from_api_base("https://api.openai.com/v1")
        );
        assert_eq!(
            detect_provider("https://my-resource.openai.azure.com"),
            detect_provider_from_api_base("https://my-resource.openai.azure.com")
        );
    }

    #[test]
    fn detect_provider_azure_gpt4_url() {
        // Specific test for azure gpt-4 URL detection
        assert_eq!(
            detect_provider("https://my-resource.openai.azure.com/openai/deployments/gpt-4"),
            Some("azure_openai")
        );
        assert_eq!(
            detect_provider("https://eastus.api.cognitive.microsoft.com/openai/deployments/gpt-4o"),
            Some("azure_openai")
        );
    }

    #[test]
    fn detect_provider_anthropic_url() {
        // Specific test for anthropic URL detection
        assert_eq!(
            detect_provider("https://api.anthropic.com"),
            Some("anthropic")
        );
        assert_eq!(
            detect_provider("https://api.anthropic.com/v1/messages"),
            Some("anthropic")
        );
    }

    // ========================================================================
    // parse_model_prefix azure/anthropic specific tests
    // ========================================================================

    #[test]
    fn parse_model_prefix_azure_gpt4() {
        // Azure uses azure/ prefix for explicit provider specification
        assert_eq!(parse_model_prefix("azure/gpt-4"), (Some("azure"), "gpt-4"));
        assert_eq!(
            parse_model_prefix("azure/gpt-4o"),
            (Some("azure"), "gpt-4o")
        );
        assert_eq!(
            parse_model_prefix("azure/gpt-4-turbo"),
            (Some("azure"), "gpt-4-turbo")
        );
    }

    #[test]
    fn parse_model_prefix_anthropic_claude() {
        // Anthropic models with explicit provider prefix
        assert_eq!(
            parse_model_prefix("anthropic/claude-3-5-sonnet"),
            (Some("anthropic"), "claude-3-5-sonnet")
        );
        assert_eq!(
            parse_model_prefix("anthropic/claude-opus-4"),
            (Some("anthropic"), "claude-opus-4")
        );
        assert_eq!(
            parse_model_prefix("anthropic/claude-3-haiku"),
            (Some("anthropic"), "claude-3-haiku")
        );
    }

    // ========================================================================
    // KNOWN_API_BASES tests
    // ========================================================================

    #[test]
    #[allow(clippy::cognitive_complexity)]
    fn known_api_bases_contains_expected_providers() {
        let providers: Vec<_> = KNOWN_API_BASES.iter().map(|(k, _)| *k).collect();

        // Check all expected providers are present
        assert!(providers.contains(&"anthropic"));
        assert!(providers.contains(&"openai"));
        assert!(providers.contains(&"azure_openai"));
        assert!(providers.contains(&"deepseek"));
        assert!(providers.contains(&"google"));
        assert!(providers.contains(&"minimax"));
        assert!(providers.contains(&"moonshotai"));
        assert!(providers.contains(&"mistral"));
        assert!(providers.contains(&"groq"));
        assert!(providers.contains(&"fireworks"));
        assert!(providers.contains(&"together"));
        assert!(providers.contains(&"kimi-code"));
        assert!(providers.contains(&"opencode"));
        assert!(providers.contains(&"xai"));
        assert!(providers.contains(&"ollama"));
        assert!(providers.contains(&"openrouter"));
    }

    #[test]
    fn known_api_bases_format() {
        // Verify each entry has a valid URL
        for (provider, url) in KNOWN_API_BASES {
            assert!(
                url.starts_with("http://") || url.starts_with("https://"),
                "Invalid URL for provider {}: {}",
                provider,
                url
            );
        }
    }

    // ========================================================================
    // is_known_provider tests
    // ========================================================================

    #[test]
    fn is_known_provider_positive() {
        for p in ["openai", "anthropic", "google", "azure", "groq", "mock"] {
            assert!(is_known_provider(p), "{p} should be known");
        }
    }

    #[test]
    fn is_known_provider_negative() {
        assert!(!is_known_provider("unknown"));
        assert!(!is_known_provider(""));
        assert!(!is_known_provider("custom"));
    }

    // ========================================================================
    // normalize_model_name tests
    // ========================================================================

    #[test]
    fn normalize_with_provider_prefix() {
        assert_eq!(
            normalize_model_name("azure/gpt-4", Some("openai")),
            "azure/gpt-4"
        );
    }

    #[test]
    fn normalize_adds_default_provider() {
        assert_eq!(
            normalize_model_name("gpt-4", Some("openai")),
            "openai/gpt-4"
        );
    }

    #[test]
    fn normalize_no_provider_no_default() {
        assert_eq!(normalize_model_name("gpt-4", None), "gpt-4");
    }
}
