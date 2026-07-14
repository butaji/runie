//! Provider detection utilities.
//!
//! Parses model names with provider prefixes and infers providers from API base URLs.

use once_cell::sync::Lazy;
use std::collections::HashMap;

/// Parse a model name with optional provider prefix.
///
/// Supports the `provider/model` syntax used throughout litellm:
/// - `"azure/gpt-4"` → `(Some("azure"), "gpt-4")`
/// - `"anthropic/claude-3-opus"` → `(Some("anthropic"), "claude-3-opus")`
/// - `"gpt-4"` → `(None, "gpt-4")`
/// - `"claude-3-opus"` → `(None, "claude-3-opus")`
///
/// # Example
/// ```
/// let (provider, model) = parse_model_prefix("azure/gpt-4");
/// assert_eq!(provider, Some("azure"));
/// assert_eq!(model, "gpt-4");
/// ```
pub fn parse_model_prefix(model: &str) -> (Option<&str>, &str) {
    if let Some((provider, rest)) = model.split_once('/') {
        // Validate that provider looks like a provider name (not a path)
        if !rest.is_empty() && !provider.contains('\\') && !rest.contains('/') {
            return (Some(provider), rest);
        }
    }
    (None, model)
}

/// Known API base URL patterns mapped to their provider names.
///
/// Used by [`detect_provider_from_url`] to infer the provider from an API base URL.
static KNOWN_API_BASES: Lazy<HashMap<&str, &str>> = Lazy::new(|| {
    let mut m = HashMap::new();

    // OpenAI-compatible endpoints
    m.insert("api.openai.com", "openai");
    m.insert("https://api.openai.com", "openai");
    m.insert("http://api.openai.com", "openai");

    // Anthropic
    m.insert("api.anthropic.com", "anthropic");
    m.insert("https://api.anthropic.com", "anthropic");

    // Google AI / Gemini
    m.insert("generativelanguage.googleapis.com", "google");
    m.insert("https://generativelanguage.googleapis.com", "google");
    m.insert("aistudio.googleapis.com", "google");
    m.insert("https://aistudio.googleapis.com", "google");

    // Azure OpenAI
    m.insert("openai.azure.com", "azure");
    m.insert("https://openai.azure.com", "azure");
    m.insert(".openai.azure.com", "azure");
    m.insert("https://.openai.azure.com", "azure");

    // Groq
    m.insert("api.groq.com", "groq");
    m.insert("https://api.groq.com", "groq");

    // Together AI
    m.insert("api.together.xyz", "together");
    m.insert("https://api.together.xyz", "together");

    // DeepInfra
    m.insert("api.deepinfra.com", "deepinfra");
    m.insert("https://api.deepinfra.com", "deepinfra");

    // Mistral
    m.insert("api.mistral.ai", "mistral");
    m.insert("https://api.mistral.ai", "mistral");

    // Cohere
    m.insert("api.cohere.ai", "cohere");
    m.insert("https://api.cohere.ai", "cohere");

    // Fireworks AI
    m.insert("api.fireworks.ai", "fireworks");
    m.insert("https://api.fireworks.ai", "fireworks");

    // Perplexity
    m.insert("api.perplexity.ai", "perplexity");
    m.insert("https://api.perplexity.ai", "perplexity");

    // Ollama (local)
    m.insert("localhost:11434", "ollama");
    m.insert("http://localhost:11434", "ollama");
    m.insert("http://127.0.0.1:11434", "ollama");

    m
});

/// Detect the provider name from an API base URL.
///
/// Returns `None` if the URL doesn't match any known provider pattern.
///
/// # Example
/// ```
/// let provider = detect_provider_from_url("https://api.openai.com/v1");
/// assert_eq!(provider, Some("openai"));
/// ```
pub fn detect_provider_from_url(url: &str) -> Option<&'static str> {
    let url = url.trim_end_matches('/');

    // Try exact match first
    if let Some(provider) = KNOWN_API_BASES.get(url) {
        return Some(provider);
    }

    // Try host-only match (strip path)
    if let Ok(parsed) = url::Url::parse(url) {
        if let Some(host) = parsed.host_str() {
            let host_lower = host.to_lowercase();

            // Check for Azure-style wildcard subdomains
            if host_lower.ends_with(".openai.azure.com") {
                return KNOWN_API_BASES.get(".openai.azure.com").copied();
            }

            // Try exact host match
            if let Some(provider) = KNOWN_API_BASES.get(host_lower.as_str()) {
                return Some(provider);
            }

            // Try with https:// prefix
            let with_scheme = format!("https://{host_lower}");
            if let Some(provider) = KNOWN_API_BASES.get(with_scheme.as_str()) {
                return Some(provider);
            }
        }
    }

    None
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

    // ── parse_model_prefix ───────────────────────────────────────────────────────

    #[test]
    fn parse_model_prefix_with_provider() {
        let (provider, model) = parse_model_prefix("azure/gpt-4");
        assert_eq!(provider, Some("azure"));
        assert_eq!(model, "gpt-4");
    }

    #[test]
    fn parse_model_prefix_anthropic() {
        let (provider, model) = parse_model_prefix("anthropic/claude-3-opus-20240229");
        assert_eq!(provider, Some("anthropic"));
        assert_eq!(model, "claude-3-opus-20240229");
    }

    #[test]
    fn parse_model_prefix_no_provider() {
        let (provider, model) = parse_model_prefix("gpt-4");
        assert_eq!(provider, None);
        assert_eq!(model, "gpt-4");
    }

    #[test]
    fn parse_model_prefix_bare_anthropic() {
        let (provider, model) = parse_model_prefix("claude-3-opus");
        assert_eq!(provider, None);
        assert_eq!(model, "claude-3-opus");
    }

    #[test]
    fn parse_model_prefix_slash_in_path_not_provider() {
        // A path like "folder/model" should not be split as provider/model
        let (provider, model) = parse_model_prefix("folder/model-name");
        assert_eq!(provider, None);
        assert_eq!(model, "folder/model-name");
    }

    #[test]
    fn parse_model_prefix_double_slash() {
        let (provider, model) = parse_model_prefix("azure//gpt-4");
        // Empty segment after provider means no valid prefix
        assert_eq!(provider, None);
        assert_eq!(model, "azure//gpt-4");
    }

    // ── detect_provider_from_url ─────────────────────────────────────────────────

    #[test]
    fn detect_provider_from_openai_url() {
        assert_eq!(detect_provider_from_url("https://api.openai.com/v1"), Some("openai"));
        assert_eq!(detect_provider_from_url("api.openai.com/v1"), Some("openai"));
    }

    #[test]
    fn detect_provider_from_anthropic_url() {
        assert_eq!(detect_provider_from_url("https://api.anthropic.com"), Some("anthropic"));
    }

    #[test]
    fn detect_provider_from_azure_url() {
        assert_eq!(detect_provider_from_url("https://my-resource.openai.azure.com"), Some("azure"));
        assert_eq!(detect_provider_from_url("https://test.openai.azure.com/"), Some("azure"));
    }

    #[test]
    fn detect_provider_from_groq_url() {
        assert_eq!(detect_provider_from_url("https://api.groq.com/openai/v1"), Some("groq"));
    }

    #[test]
    fn detect_provider_from_ollama_local() {
        assert_eq!(detect_provider_from_url("http://localhost:11434"), Some("ollama"));
        assert_eq!(detect_provider_from_url("http://127.0.0.1:11434"), Some("ollama"));
    }

    #[test]
    fn detect_provider_unknown_url() {
        assert_eq!(detect_provider_from_url("https://my-custom-api.example.com"), None);
    }

    // ── is_known_provider ────────────────────────────────────────────────────────

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

    // ── normalize_model_name ─────────────────────────────────────────────────────

    #[test]
    fn normalize_with_provider_prefix() {
        assert_eq!(normalize_model_name("azure/gpt-4", Some("openai")), "azure/gpt-4");
    }

    #[test]
    fn normalize_adds_default_provider() {
        assert_eq!(normalize_model_name("gpt-4", Some("openai")), "openai/gpt-4");
    }

    #[test]
    fn normalize_no_provider_no_default() {
        assert_eq!(normalize_model_name("gpt-4", None), "gpt-4");
    }
}
