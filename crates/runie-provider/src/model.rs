//! Model identification and registry
//!
//! Provider/model list derived from pi (https://pi.codes).
//! 35 providers, ~968 models in the upstream catalog.
//! This registry keeps a curated subset of headline models.

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
// Built-in Provider Catalog (35 providers from pi)
// ============================================================================

pub fn builtin_providers() -> Vec<ProviderMeta> {
    vec![
        // First-party cloud
        ProviderMeta::new("openai", "OPENAI_API_KEY"),
        ProviderMeta::new("openai-codex", "OPENAI_API_KEY"),
        ProviderMeta::new("anthropic", "ANTHROPIC_API_KEY"),
        ProviderMeta::new("google", "GEMINI_API_KEY"),
        ProviderMeta::new("google-vertex", "GOOGLE_API_KEY"),
        ProviderMeta::new("xai", "XAI_API_KEY"),
        ProviderMeta::new("github-copilot", "GITHUB_TOKEN"),
        // Aggregators / routers
        ProviderMeta::new("openrouter", "OPENROUTER_API_KEY"),
        ProviderMeta::new("groq", "GROQ_API_KEY"),
        ProviderMeta::new("deepseek", "DEEPSEEK_API_KEY"),
        ProviderMeta::new("mistral", "MISTRAL_API_KEY"),
        ProviderMeta::new("fireworks", "FIREWORKS_API_KEY"),
        ProviderMeta::new("together", "TOGETHER_API_KEY"),
        ProviderMeta::new("nvidia", "NVIDIA_API_KEY"),
        ProviderMeta::new("cerebras", "CEREBRAS_API_KEY"),
        ProviderMeta::new("huggingface", "HF_TOKEN"),
        // Cloud platforms
        ProviderMeta::new("azure-openai-responses", "AZURE_OPENAI_API_KEY"),
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
        ProviderMeta::new("vercel-ai-gateway", "AI_GATEWAY_API_KEY"),
        // China / regional
        ProviderMeta::new("ant-ling", "ANT_LING_API_KEY"),
        ProviderMeta::new("kimi-coding", "KIMI_API_KEY"),
        ProviderMeta::new("minimax", "MINIMAX_API_KEY"),
        ProviderMeta::new("minimax-cn", "MINIMAX_CN_API_KEY"),
        ProviderMeta::new("moonshotai", "MOONSHOT_API_KEY"),
        ProviderMeta::new("moonshotai-cn", "MOONSHOT_CN_API_KEY"),
        ProviderMeta::new("xiaomi", "XIAOMI_API_KEY"),
        ProviderMeta::new("xiaomi-token-plan-ams", "XIAOMI_TOKEN_PLAN_AMS_API_KEY"),
        ProviderMeta::new("xiaomi-token-plan-cn", "XIAOMI_TOKEN_PLAN_CN_API_KEY"),
        ProviderMeta::new("xiaomi-token-plan-sgp", "XIAOMI_TOKEN_PLAN_SGP_API_KEY"),
        ProviderMeta::new("zai", "ZAI_API_KEY"),
        ProviderMeta::new("zai-coding-cn", "ZAI_CODING_CN_API_KEY"),
        ProviderMeta::new("opencode", "OPENCODE_API_KEY"),
        ProviderMeta::new("opencode-go", "OPENCODE_API_KEY"),
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
        let mut m = Vec::new();

        // amazon-bedrock (curated: US + key cross-region)
        for name in [
            "us.anthropic.claude-sonnet-4-6",
            "us.anthropic.claude-opus-4-6-v1",
            "us.anthropic.claude-opus-4-7",
            "us.anthropic.claude-opus-4-8",
            "us.meta.llama4-scout-17b-instruct-v1:0",
            "us.meta.llama4-maverick-17b-instruct-v1:0",
            "us.deepseek.r1-v1:0",
            "amazon.nova-pro-v1:0",
            "amazon.nova-lite-v1:0",
            "mistral.mistral-large-3-675b-instruct",
        ] {
            m.push(ModelId::new("amazon-bedrock", name));
        }

        // ant-ling
        for name in ["Ling-2.6-1T", "Ling-2.6-flash", "Ring-2.6-1T"] {
            m.push(ModelId::new("ant-ling", name));
        }

        // anthropic
        for name in [
            "claude-sonnet-4-6",
            "claude-opus-4-6",
            "claude-opus-4-7",
            "claude-opus-4-8",
            "claude-haiku-4-5",
            "claude-3-7-sonnet-20250219",
            "claude-3-5-sonnet-20241022",
            "claude-3-5-haiku-20241022",
            "claude-3-opus-20240229",
            "claude-sonnet-4-5-20250929",
        ] {
            m.push(ModelId::new("anthropic", name));
        }

        // azure-openai-responses
        for name in ["gpt-4o", "gpt-4.1", "gpt-4.1-mini", "gpt-4.1-nano", "o3-mini"] {
            m.push(ModelId::new("azure-openai-responses", name));
        }

        // cerebras
        for name in ["gpt-oss-120b", "llama3.1-8b", "zai-glm-4.7"] {
            m.push(ModelId::new("cerebras", name));
        }

        // cloudflare-ai-gateway
        for name in ["claude-sonnet-4-5", "gpt-5.1", "gemini-2.5-pro"] {
            m.push(ModelId::new("cloudflare-ai-gateway", name));
        }

        // cloudflare-workers-ai
        for name in [
            "@cf/moonshotai/kimi-k2.6",
            "@cf/meta/llama-4-scout-17b-16e-instruct",
            "@cf/mistralai/mistral-small-3.1-24b-instruct",
        ] {
            m.push(ModelId::new("cloudflare-workers-ai", name));
        }

        // deepseek
        for name in ["deepseek-v4-flash", "deepseek-v4-pro"] {
            m.push(ModelId::new("deepseek", name));
        }

        // fireworks
        for name in [
            "accounts/fireworks/models/deepseek-v4-pro",
            "accounts/fireworks/models/kimi-k2p6",
            "accounts/fireworks/models/qwen3p6-plus",
            "accounts/fireworks/models/gpt-oss-120b",
        ] {
            m.push(ModelId::new("fireworks", name));
        }

        // github-copilot
        for name in [
            "claude-sonnet-4-6",
            "claude-opus-4-7",
            "gemini-2.5-pro",
            "gpt-5.4",
            "gpt-5.2-codex",
        ] {
            m.push(ModelId::new("github-copilot", name));
        }

        // google
        for name in [
            "gemini-2.5-pro",
            "gemini-2.5-flash",
            "gemini-2.5-flash-lite",
            "gemini-2.0-flash",
            "gemini-3-flash-preview",
            "gemini-3.1-pro-preview",
            "gemma-4-31b-it",
        ] {
            m.push(ModelId::new("google", name));
        }

        // google-vertex
        for name in [
            "gemini-1.5-pro",
            "gemini-1.5-flash",
            "gemini-2.0-flash",
            "gemini-2.5-pro",
        ] {
            m.push(ModelId::new("google-vertex", name));
        }

        // groq
        for name in [
            "llama-3.3-70b-versatile",
            "llama-3.1-8b-instant",
            "gemma2-9b-it",
            "mixtral-8x7b-32768",
            "qwen-qwq-32b",
        ] {
            m.push(ModelId::new("groq", name));
        }

        // huggingface
        for name in [
            "meta-llama/Llama-3.3-70B-Instruct",
            "Qwen/Qwen3-Coder-480B-A35B-Instruct",
            "mistralai/Mistral-Large-Instruct-2411",
        ] {
            m.push(ModelId::new("huggingface", name));
        }

        // kimi-coding
        for name in ["kimi-for-coding", "kimi-k2-thinking"] {
            m.push(ModelId::new("kimi-coding", name));
        }

        // minimax
        for name in ["MiniMax-M2.7", "MiniMax-M3"] {
            m.push(ModelId::new("minimax", name));
        }

        // minimax-cn
        for name in ["MiniMax-M2.7", "MiniMax-M3"] {
            m.push(ModelId::new("minimax-cn", name));
        }

        // mistral
        for name in [
            "mistral-large-latest",
            "codestral-latest",
            "ministral-8b-latest",
            "ministral-3b-latest",
            "pixtral-large-latest",
            "devstral-latest",
            "mistral-medium-latest",
        ] {
            m.push(ModelId::new("mistral", name));
        }

        // moonshotai
        for name in [
            "kimi-k2.5",
            "kimi-k2.6",
            "kimi-k2-thinking",
            "kimi-k2-turbo-preview",
        ] {
            m.push(ModelId::new("moonshotai", name));
        }

        // moonshotai-cn
        for name in ["kimi-k2.5", "kimi-k2-thinking"] {
            m.push(ModelId::new("moonshotai-cn", name));
        }

        // nvidia
        for name in [
            "meta/llama-3.3-70b-instruct",
            "nvidia/nemotron-3-super-120b-a12b",
            "nvidia/nemotron-3-nano-30b-a3b",
            "mistralai/mistral-large-3-675b-instruct-2512",
        ] {
            m.push(ModelId::new("nvidia", name));
        }

        // openai
        for name in [
            "gpt-4o",
            "gpt-4o-mini",
            "gpt-5",
            "gpt-5.1",
            "gpt-5.1-codex",
            "gpt-5.2",
            "gpt-5.4",
            "gpt-5.5",
            "o3-mini",
            "o4-mini",
            "o1",
            "o3",
        ] {
            m.push(ModelId::new("openai", name));
        }

        // openai-codex
        for name in ["gpt-5.3-codex-spark", "gpt-5.4", "gpt-5.5"] {
            m.push(ModelId::new("openai-codex", name));
        }

        // opencode
        for name in [
            "claude-sonnet-4-6",
            "gpt-5.1",
            "gpt-5.2-codex",
            "kimi-k2.6",
            "glm-5",
            "deepseek-v4-flash",
        ] {
            m.push(ModelId::new("opencode", name));
        }

        // opencode-go
        for name in ["glm-5", "kimi-k2.6", "deepseek-v4-pro", "qwen3.7-max"] {
            m.push(ModelId::new("opencode-go", name));
        }

        // openrouter (curated headline routes)
        for name in [
            "anthropic/claude-sonnet-4.6",
            "anthropic/claude-opus-4.7",
            "anthropic/claude-opus-4.8",
            "anthropic/claude-haiku-4.5",
            "openai/gpt-5",
            "openai/gpt-5.1",
            "openai/gpt-4o",
            "openai/o3-mini",
            "google/gemini-2.5-pro",
            "google/gemini-2.5-flash",
            "meta-llama/llama-4-maverick",
            "meta-llama/llama-4-scout",
            "deepseek/deepseek-chat",
            "deepseek/deepseek-r1",
            "mistralai/mistral-large",
            "moonshotai/kimi-k2.6",
            "nvidia/nemotron-3-super-120b-a12b",
            "x-ai/grok-4.3",
            "qwen/qwen3-235b-a22b",
            "z-ai/glm-5",
            "~anthropic/claude-sonnet-latest",
            "~openai/gpt-latest",
        ] {
            m.push(ModelId::new("openrouter", name));
        }

        // together
        for name in [
            "meta-llama/Llama-3.3-70B-Instruct-Turbo",
            "deepseek-ai/DeepSeek-V4-Pro",
            "Qwen/Qwen3.7-Max",
            "moonshotai/Kimi-K2.6",
            "google/gemma-4-31B-it",
        ] {
            m.push(ModelId::new("together", name));
        }

        // vercel-ai-gateway
        for name in ["moonshotai/kimi-k2.5", "anthropic/claude-sonnet-4", "openai/gpt-5"] {
            m.push(ModelId::new("vercel-ai-gateway", name));
        }

        // xai
        for name in ["grok-3", "grok-3-fast", "grok-4.3", "grok-build-0.1"] {
            m.push(ModelId::new("xai", name));
        }

        // xiaomi
        for name in ["mimo-v2.5", "mimo-v2.5-pro", "mimo-v2-flash"] {
            m.push(ModelId::new("xiaomi", name));
        }

        // xiaomi-token-plan-ams / cn / sgp
        for name in ["mimo-v2.5", "mimo-v2.5-pro"] {
            m.push(ModelId::new("xiaomi-token-plan-ams", name));
            m.push(ModelId::new("xiaomi-token-plan-cn", name));
            m.push(ModelId::new("xiaomi-token-plan-sgp", name));
        }

        // zai
        for name in ["glm-4.7", "glm-5", "glm-5-turbo", "glm-5.1", "glm-4.5-air"] {
            m.push(ModelId::new("zai", name));
        }

        // zai-coding-cn
        for name in ["glm-4.7", "glm-5", "glm-5-turbo", "glm-5.1"] {
            m.push(ModelId::new("zai-coding-cn", name));
        }

        // ollama (local)
        for name in [
            "llama3.1",
            "llama3.1:8b",
            "qwen2.5-coder:7b",
            "gpt-oss:20b",
            "llama3.2",
            "mistral",
        ] {
            m.push(ModelId::new("ollama", name));
        }

        // mock
        m.push(ModelId::new("mock", "echo"));

        Self { models: m }
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

    pub fn providers_with_models(&self) -> Vec<&str> {
        let mut providers: Vec<&str> = self
            .models
            .iter()
            .map(|m| m.provider.as_str())
            .collect::<std::collections::HashSet<_>>()
            .into_iter()
            .collect();
        providers.sort();
        providers
    }

    pub fn register(&mut self, model: ModelId) {
        self.models.push(model);
    }
}
