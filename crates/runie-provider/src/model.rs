//! Model identification and registry
//!
//! Provider/model list derived from pi (https://pi.codes).
//! 35 providers, ~968 models in the upstream catalog.
//! This registry keeps a curated subset of headline models.





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





pub fn builtin_providers() -> Vec<ProviderMeta> {
    vec![

        ProviderMeta::new("openai", "OPENAI_API_KEY"),
        ProviderMeta::new("openai-codex", "OPENAI_API_KEY"),
        ProviderMeta::new("anthropic", "ANTHROPIC_API_KEY"),
        ProviderMeta::new("google", "GEMINI_API_KEY"),
        ProviderMeta::new("google-vertex", "GOOGLE_API_KEY"),
        ProviderMeta::new("xai", "XAI_API_KEY"),
        ProviderMeta::new("github-copilot", "GITHUB_TOKEN"),

        ProviderMeta::new("openrouter", "OPENROUTER_API_KEY"),
        ProviderMeta::new("groq", "GROQ_API_KEY"),
        ProviderMeta::new("deepseek", "DEEPSEEK_API_KEY"),
        ProviderMeta::new("mistral", "MISTRAL_API_KEY"),
        ProviderMeta::new("fireworks", "FIREWORKS_API_KEY"),
        ProviderMeta::new("together", "TOGETHER_API_KEY"),
        ProviderMeta::new("nvidia", "NVIDIA_API_KEY"),
        ProviderMeta::new("cerebras", "CEREBRAS_API_KEY"),
        ProviderMeta::new("huggingface", "HF_TOKEN"),

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

        ProviderMeta::with_url("ollama", "OLLAMA_HOST", "http://localhost:11434/v1"),

        ProviderMeta::new("mock", ""),
    ]
}





pub struct ModelRegistry {
    models: Vec<ModelId>,
}

impl Default for ModelRegistry {
    fn default() -> Self {
        let catalog: &[(&str, &[&str])] = &[
            ("amazon-bedrock", &[
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
            ]),
            ("ant-ling", &["Ling-2.6-1T", "Ling-2.6-flash", "Ring-2.6-1T"]),
            ("anthropic", &[
                "claude-sonnet-4-6", "claude-opus-4-6", "claude-opus-4-7",
                "claude-opus-4-8", "claude-haiku-4-5", "claude-3-7-sonnet-20250219",
                "claude-3-5-sonnet-20241022", "claude-3-5-haiku-20241022",
                "claude-3-opus-20240229", "claude-sonnet-4-5-20250929",
            ]),
            ("azure-openai-responses", &["gpt-4o", "gpt-4.1", "gpt-4.1-mini", "gpt-4.1-nano", "o3-mini"]),
            ("cerebras", &["gpt-oss-120b", "llama3.1-8b", "zai-glm-4.7"]),
            ("cloudflare-ai-gateway", &["claude-sonnet-4-5", "gpt-5.1", "gemini-2.5-pro"]),
            ("cloudflare-workers-ai", &[
                "@cf/moonshotai/kimi-k2.6",
                "@cf/meta/llama-4-scout-17b-16e-instruct",
                "@cf/mistralai/mistral-small-3.1-24b-instruct",
            ]),
            ("deepseek", &["deepseek-v4-flash", "deepseek-v4-pro"]),
            ("fireworks", &[
                "accounts/fireworks/models/deepseek-v4-pro",
                "accounts/fireworks/models/kimi-k2p6",
                "accounts/fireworks/models/qwen3p6-plus",
                "accounts/fireworks/models/gpt-oss-120b",
            ]),
            ("github-copilot", &[
                "claude-sonnet-4-6", "claude-opus-4-7", "gemini-2.5-pro", "gpt-5.4", "gpt-5.2-codex",
            ]),
            ("google", &[
                "gemini-2.5-pro", "gemini-2.5-flash", "gemini-2.5-flash-lite",
                "gemini-2.0-flash", "gemini-3-flash-preview", "gemini-3.1-pro-preview", "gemma-4-31b-it",
            ]),
            ("google-vertex", &["gemini-1.5-pro", "gemini-1.5-flash", "gemini-2.0-flash", "gemini-2.5-pro"]),
            ("groq", &[
                "llama-3.3-70b-versatile", "llama-3.1-8b-instant",
                "gemma2-9b-it", "mixtral-8x7b-32768", "qwen-qwq-32b",
            ]),
            ("huggingface", &[
                "meta-llama/Llama-3.3-70B-Instruct",
                "Qwen/Qwen3-Coder-480B-A35B-Instruct",
                "mistralai/Mistral-Large-Instruct-2411",
            ]),
            ("kimi-coding", &["kimi-for-coding", "kimi-k2-thinking"]),
            ("minimax", &["MiniMax-M2.7", "MiniMax-M3"]),
            ("minimax-cn", &["MiniMax-M2.7", "MiniMax-M3"]),
            ("mistral", &[
                "mistral-large-latest", "codestral-latest", "ministral-8b-latest",
                "ministral-3b-latest", "pixtral-large-latest", "devstral-latest", "mistral-medium-latest",
            ]),
            ("moonshotai", &[
                "kimi-k2.5", "kimi-k2.6", "kimi-k2-thinking", "kimi-k2-turbo-preview",
            ]),
            ("moonshotai-cn", &["kimi-k2.5", "kimi-k2-thinking"]),
            ("nvidia", &[
                "meta/llama-3.3-70b-instruct",
                "nvidia/nemotron-3-super-120b-a12b",
                "nvidia/nemotron-3-nano-30b-a3b",
                "mistralai/mistral-large-3-675b-instruct-2512",
            ]),
            ("openai", &[
                "gpt-4o", "gpt-4o-mini", "gpt-5", "gpt-5.1", "gpt-5.1-codex",
                "gpt-5.2", "gpt-5.4", "gpt-5.5", "o3-mini", "o4-mini", "o1", "o3",
            ]),
            ("openai-codex", &["gpt-5.3-codex-spark", "gpt-5.4", "gpt-5.5"]),
            ("opencode", &[
                "claude-sonnet-4-6", "gpt-5.1", "gpt-5.2-codex", "kimi-k2.6", "glm-5", "deepseek-v4-flash",
            ]),
            ("opencode-go", &["glm-5", "kimi-k2.6", "deepseek-v4-pro", "qwen3.7-max"]),
            ("openrouter", &[
                "anthropic/claude-sonnet-4.6", "anthropic/claude-opus-4.7", "anthropic/claude-opus-4.8",
                "anthropic/claude-haiku-4.5", "openai/gpt-5", "openai/gpt-5.1", "openai/gpt-4o",
                "openai/o3-mini", "google/gemini-2.5-pro", "google/gemini-2.5-flash",
                "meta-llama/llama-4-maverick", "meta-llama/llama-4-scout",
                "deepseek/deepseek-chat", "deepseek/deepseek-r1", "mistralai/mistral-large",
                "moonshotai/kimi-k2.6", "nvidia/nemotron-3-super-120b-a12b",
                "x-ai/grok-4.3", "qwen/qwen3-235b-a22b", "z-ai/glm-5",
                "~anthropic/claude_sonnet-latest", "~openai/gpt-latest",
            ]),
            ("together", &[
                "meta-llama/Llama-3.3-70B-Instruct-Turbo", "deepseek-ai/DeepSeek-V4-Pro",
                "Qwen/Qwen3.7-Max", "moonshotai/Kimi-K2.6", "google/gemma-4-31B-it",
            ]),
            ("vercel-ai-gateway", &["moonshotai/kimi-k2.5", "anthropic/claude_sonnet-4", "openai/gpt-5"]),
            ("xai", &["grok-3", "grok-3-fast", "grok-4.3", "grok-build-0.1"]),
            ("xiaomi", &["mimo-v2.5", "mimo-v2.5-pro", "mimo-v2-flash"]),
            ("xiaomi-token-plan-ams", &["mimo-v2.5", "mimo-v2.5-pro"]),
            ("xiaomi-token-plan-cn", &["mimo-v2.5", "mimo-v2.5-pro"]),
            ("xiaomi-token-plan-sgp", &["mimo-v2.5", "mimo-v2.5-pro"]),
            ("zai", &["glm-4.7", "glm-5", "glm-5-turbo", "glm-5.1", "glm-4.5-air"]),
            ("zai-coding-cn", &["glm-4.7", "glm-5", "glm-5-turbo", "glm-5.1"]),
            ("ollama", &[
                "llama3.1", "llama3.1:8b", "qwen2.5-coder:7b",
                "gpt-oss:20b", "llama3.2", "mistral",
            ]),
            ("mock", &["echo"]),
        ];

        let mut models = Vec::new();
        for &(provider, names) in catalog {
            for name in names {
                models.push(ModelId::new(provider, *name));
            }
        }
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
