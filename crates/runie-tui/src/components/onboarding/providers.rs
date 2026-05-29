// ============================================================================
// Provider Definitions
// ============================================================================

use super::ProviderOption;

fn provider(name: &str, id: &str, description: &str, key_prefix: &str) -> ProviderOption {
    ProviderOption {
        name: name.to_string(),
        id: id.to_string(),
        description: description.to_string(),
        key_prefix: key_prefix.to_string(),
    }
}

fn with_prefix(p: ProviderOption, prefix: &str) -> ProviderOption {
    ProviderOption { key_prefix: prefix.to_string(), ..p }
}

fn providers_with_prefix() -> Vec<ProviderOption> {
    vec![
        with_prefix(provider("OpenAI", "openai", "GPT-4o family of models", ""), "sk-"),
        with_prefix(provider("Anthropic", "anthropic", "Claude family of models", ""), "sk-ant-"),
        provider("Google", "google", "Gemini family of models", ""),
        provider("Cohere", "cohere", "Command R family of models", ""),
        provider("Mistral", "mistral", "Mistral AI models", ""),
        provider("DeepSeek", "deepseek", "DeepSeek models", ""),
        provider("Groq", "groq", "Fast inference with Llama", ""),
        provider("OpenRouter", "openrouter", "Access multiple models via OpenRouter", ""),
        provider("HuggingFace", "huggingface", "Open-source models", ""),
        provider("xAI", "xai", "Grok models", ""),
        provider("Azure", "azure", "Microsoft Azure OpenAI", ""),
        provider("Moonshot", "moonshot", "Moonshot AI models", ""),
        provider("Perplexity", "perplexity", "Online search-augmented models", ""),
        provider("Ollama", "ollama", "Local model inference", ""),
        provider("Hyperbolic", "hyperbolic", "Open-source models at low cost", ""),
        provider("Together", "together", "Together AI models", ""),
        provider("ZAI", "zai", "ZAI models", ""),
        provider("MiniMax", "minimax", "MiniMax AI models", ""),
        provider("Mira", "mira", "Mira models", ""),
        provider("Galadriel", "galadriel", "Galadriel models", ""),
        provider("Llamafile", "llamafile", "Local llamafile models", ""),
    ]
}

/// Returns the default list of providers, sorted alphabetically by name.
pub fn get_default_providers() -> Vec<ProviderOption> {
    let mut providers = providers_with_prefix();
    providers.sort_by(|a, b| a.name.cmp(&b.name));
    providers
}
