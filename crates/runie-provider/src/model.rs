//! Model identification and registry
//!
//! Provider/model list derived from pi (https://pi.codes).
//! Covers cloud APIs, local runners, and proxy gateways.

// ============================================================================
// ModelId
// ============================================================================

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct ModelId {
    pub provider: String,
    pub name: String,
}

impl ModelId {
    pub fn new(provider: impl Into<String>, name: impl Into<String>) -> Self {
        Self {
            provider: provider.into(),
            name: name.into(),
        }
    }

    pub fn full(&self) -> String {
        format!("{}/{}", self.provider, self.name)
    }
}

// ============================================================================
// Provider Metadata
// ============================================================================

#[derive(Debug, Clone)]
pub struct ProviderMeta {
    pub key: &'static str,
    pub env_var: &'static str,
    pub base_url_hint: Option<&'static str>,
}

impl ProviderMeta {
    pub const fn new(key: &'static str, env_var: &'static str) -> Self {
        Self {
            key,
            env_var,
            base_url_hint: None,
        }
    }

    pub const fn with_url(
        key: &'static str,
        env_var: &'static str,
        url: &'static str,
    ) -> Self {
        Self {
            key,
            env_var,
            base_url_hint: Some(url),
        }
    }
}

// ============================================================================
// Built-in Provider Catalog
// ============================================================================

pub fn builtin_providers() -> Vec<ProviderMeta> {
    vec![
        // Subscriptions / first-party
        ProviderMeta::new("openai", "OPENAI_API_KEY"),
        ProviderMeta::new("anthropic", "ANTHROPIC_API_KEY"),
        ProviderMeta::new("google", "GEMINI_API_KEY"),
        ProviderMeta::new("xai", "XAI_API_KEY"),
        // API-key aggregators / routers
        ProviderMeta::new("openrouter", "OPENROUTER_API_KEY"),
        ProviderMeta::new("groq", "GROQ_API_KEY"),
        ProviderMeta::new("deepseek", "DEEPSEEK_API_KEY"),
        ProviderMeta::new("mistral", "MISTRAL_API_KEY"),
        ProviderMeta::new("fireworks", "FIREWORKS_API_KEY"),
        ProviderMeta::new("together", "TOGETHER_API_KEY"),
        ProviderMeta::new("nvidia", "NVIDIA_API_KEY"),
        ProviderMeta::new("cerebras", "CEREBRAS_API_KEY"),
        // Cloud platforms
        ProviderMeta::new("azure-openai", "AZURE_OPENAI_API_KEY"),
        ProviderMeta::new("amazon-bedrock", "AWS_ACCESS_KEY_ID"),
        ProviderMeta::with_url(
            "cloudflare-ai-gateway",
            "CLOUDFLARE_API_KEY",
            "https://gateway.ai.cloudflare.com",
        ),
        ProviderMeta::with_url(
            "cloudflare-workers-ai",
            "CLOUDFLARE_API_KEY",
            "https://api.cloudflare.com",
        ),
        // China / regional
        ProviderMeta::new("kimi-coding", "KIMI_API_KEY"),
        ProviderMeta::new("minimax", "MINIMAX_API_KEY"),
        ProviderMeta::new("xiaomi", "XIAOMI_API_KEY"),
        ProviderMeta::new("zai", "ZAI_API_KEY"),
        ProviderMeta::new("opencode", "OPENCODE_API_KEY"),
        // Local / custom
        ProviderMeta::with_url("ollama", "OLLAMA_HOST", "http://localhost:11434/v1"),
        // Mock
        ProviderMeta::new("mock", ""),
    ]
}

// ============================================================================
// Model Registry
// ============================================================================

pub struct ModelRegistry {
    models: Vec<ModelId>,
}

impl Default for ModelRegistry {
    fn default() -> Self {
        let mut models = Vec::new();

        // OpenAI
        for m in ["gpt-4o", "gpt-4o-mini", "gpt-5.1", "o3-mini", "o4-mini"] {
            models.push(ModelId::new("openai", m));
        }

        // Anthropic
        for m in [
            "claude-sonnet-4",
            "claude-opus-4",
            "claude-sonnet-4-5",
            "claude-3-5-sonnet",
            "claude-3-5-haiku",
        ] {
            models.push(ModelId::new("anthropic", m));
        }

        // Google Gemini
        for m in [
            "gemini-2.5-pro",
            "gemini-2.5-flash",
            "gemini-2.0-flash",
            "gemma-4-31b-it",
        ] {
            models.push(ModelId::new("google", m));
        }

        // xAI
        for m in ["grok-3", "grok-3-mini"] {
            models.push(ModelId::new("xai", m));
        }

        // DeepSeek
        for m in ["deepseek-chat", "deepseek-reasoner"] {
            models.push(ModelId::new("deepseek", m));
        }

        // Groq
        for m in ["llama-3.3-70b", "mixtral-8x7b", "gemma2-9b-it"] {
            models.push(ModelId::new("groq", m));
        }

        // Mistral
        for m in ["mistral-large", "codestral", "ministral-8b"] {
            models.push(ModelId::new("mistral", m));
        }

        // OpenRouter (popular routes)
        for m in [
            "anthropic/claude-sonnet-4",
            "openai/gpt-4o",
            "google/gemini-2.5-pro",
            "deepseek/deepseek-chat",
        ] {
            models.push(ModelId::new("openrouter", m));
        }

        // Fireworks
        for m in ["llama-v3p1-70b-instruct", "qwen2p5-72b-instruct"] {
            models.push(ModelId::new("fireworks", m));
        }

        // Together
        for m in ["meta-llama/Llama-3.3-70B-Instruct-Turbo"] {
            models.push(ModelId::new("together", m));
        }

        // NVIDIA NIM
        for m in ["meta/llama-3.3-70b-instruct"] {
            models.push(ModelId::new("nvidia", m));
        }

        // Cerebras
        for m in ["llama-3.3-70b", "llama-4-scout-17b-16e"] {
            models.push(ModelId::new("cerebras", m));
        }

        // Azure OpenAI
        for m in ["gpt-4o", "gpt-4", "o3-mini"] {
            models.push(ModelId::new("azure-openai", m));
        }

        // Amazon Bedrock
        for m in [
            "us.anthropic.claude-sonnet-4-20250514-v1:0",
            "us.meta.llama3-3-70b-instruct-v1:0",
        ] {
            models.push(ModelId::new("amazon-bedrock", m));
        }

        // Cloudflare
        models.push(ModelId::new(
            "cloudflare-ai-gateway",
            "claude-sonnet-4-5",
        ));
        models.push(ModelId::new(
            "cloudflare-workers-ai",
            "@cf/moonshotai/kimi-k2.6",
        ));

        // China / regional
        models.push(ModelId::new("kimi-coding", "kimi-k2.5"));
        models.push(ModelId::new("minimax", "minimax-text-01"));
        models.push(ModelId::new("xiaomi", "mimo-7b"));
        models.push(ModelId::new("zai", "zai-coder"));
        models.push(ModelId::new("opencode", "opencodesky-v1"));

        // Ollama (local)
        for m in ["llama3.1", "llama3.1:8b", "qwen2.5-coder:7b", "gpt-oss:20b"] {
            models.push(ModelId::new("ollama", m));
        }

        // Mock
        models.push(ModelId::new("mock", "echo"));

        Self { models }
    }
}

impl ModelRegistry {
    pub fn list(&self) -> &[ModelId] {
        &self.models
    }

    pub fn find(&self, full: &str) -> Option<&ModelId> {
        self.models.iter().find(|m| m.full() == full)
    }

    pub fn by_provider(&self, provider: &str) -> Vec<&ModelId> {
        self.models
            .iter()
            .filter(|m| m.provider == provider)
            .collect()
    }

    pub fn register(&mut self, model: ModelId) {
        self.models.push(model);
    }
}
