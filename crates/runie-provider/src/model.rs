//! Model identification and registry
//!
//! Provider/model list derived from pi (https://pi.codes).
//! 35 providers, ~968 models in the upstream catalog.

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

    pub const fn with_url(key: &'static str, env_var: &'static str, url: &'static str) -> Self {
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
        ProviderMeta::new("anthropic", "ANTHROPIC_API_KEY"),
        ProviderMeta::new("google", "GEMINI_API_KEY"),
        ProviderMeta::new("openrouter", "OPENROUTER_API_KEY"),
        ProviderMeta::new("groq", "GROQ_API_KEY"),
        ProviderMeta::new("deepseek", "DEEPSEEK_API_KEY"),
        ProviderMeta::new("mistral", "MISTRAL_API_KEY"),
        ProviderMeta::new("fireworks", "FIREWORKS_API_KEY"),
        ProviderMeta::new("together", "TOGETHER_API_KEY"),
        ProviderMeta::new("ollama", "OLLAMA_HOST"),
        ProviderMeta::new("mock", ""),
    ]
}

pub struct ModelRegistry {
    models: Vec<ModelId>,
}

impl Default for ModelRegistry {
    fn default() -> Self {
        let mut models = Vec::new();
        Self::fill_ai_providers(&mut models);
        Self::fill_china_providers(&mut models);
        Self::fill_gateway_providers(&mut models);
        Self::fill_specialized(&mut models);
        Self { models }
    }
}

impl ModelRegistry {
    fn fill_ai_providers(models: &mut Vec<ModelId>) {
        let catalog: &[(&str, &[&str])] = &[
            ("anthropic", &["claude-sonnet-4-6", "claude-opus-4-7", "claude-haiku-4-5"]),
            ("openai", &["gpt-4o", "gpt-4o-mini", "gpt-5", "o3-mini", "o4-mini", "o1", "o3"]),
            ("google", &["gemini-2.5-pro", "gemini-2.5-flash", "gemini-2.0-flash"]),
            ("deepseek", &["deepseek-v4-flash", "deepseek-v4-pro"]),
            ("mistral", &["mistral-large-latest", "codestral-latest", "devstral-latest"]),
            ("groq", &["llama-3.3-70b-versatile", "gemma2-9b-it", "mixtral-8x7b-32768"]),
            ("together", &["meta-llama/Llama-3.3-70B-Instruct-Turbo", "deepseek-ai/DeepSeek-V4-Pro"]),
            ("fireworks", &["accounts/fireworks/models/deepseek-v4-pro", "accounts/fireworks/models/kimi-k2p6"]),
            ("openrouter", &[
                "anthropic/claude-sonnet-4.6", "openai/gpt-4o", "google/gemini-2.5-pro",
                "deepseek/deepseek-chat", "deepseek/deepseek-r1",
            ]),
        ];
        Self::extend_models(models, catalog);
    }

    fn fill_china_providers(models: &mut Vec<ModelId>) {
        let catalog: &[(&str, &[&str])] = &[
            ("kimi-coding", &["kimi-for-coding", "kimi-k2-thinking"]),
            ("moonshotai", &["kimi-k2.5", "kimi-k2.6", "kimi-k2-thinking"]),
            ("minimax", &["MiniMax-M2.7", "MiniMax-M3"]),
            ("xiaomi", &["mimo-v2.5", "mimo-v2.5-pro"]),
            ("zai", &["glm-4.7", "glm-5", "glm-5-turbo", "glm-5.1"]),
            ("opencode", &["claude-sonnet-4-6", "gpt-5.1", "kimi-k2.6", "glm-5"]),
        ];
        Self::extend_models(models, catalog);
    }

    fn fill_gateway_providers(models: &mut Vec<ModelId>) {
        let catalog: &[(&str, &[&str])] = &[
            ("cloudflare-ai-gateway", &["claude-sonnet-4-5", "gpt-5.1"]),
            ("cloudflare-workers-ai", &["@cf/moonshotai/kimi-k2.6", "@cf/meta/llama-4-scout-17b"]),
            ("vercel-ai-gateway", &["moonshotai/kimi-k2.5", "openai/gpt-5"]),
        ];
        Self::extend_models(models, catalog);
    }

    fn fill_specialized(models: &mut Vec<ModelId>) {
        let catalog: &[(&str, &[&str])] = &[
            ("amazon-bedrock", &["us.anthropic.claude-sonnet-4-6", "us.deepseek.r1-v1:0"]),
            ("azure-openai-responses", &["gpt-4o", "gpt-4.1", "o3-mini"]),
            ("cerebras", &["gpt-oss-120b", "llama3.1-8b"]),
            ("github-copilot", &["claude-sonnet-4-6", "gemini-2.5-pro"]),
            ("huggingface", &["meta-llama/Llama-3.3-70B-Instruct", "mistralai/Mistral-Large-Instruct-2411"]),
            ("nvidia", &["meta/llama-3.3-70b-instruct", "nvidia/nemotron-3-super-120b-a12b"]),
            ("ollama", &["llama3.1", "qwen2.5-coder:7b", "mistral"]),
            ("xai", &["grok-3", "grok-4.3"]),
            ("mock", &["echo"]),
        ];
        Self::extend_models(models, catalog);
    }

    fn extend_models(models: &mut Vec<ModelId>, catalog: &[(&str, &[&str])]) {
        for &(provider, names) in catalog {
            for name in names {
                models.push(ModelId::new(provider, *name));
            }
        }
    }

    pub fn list(&self) -> &[ModelId] {
        &self.models
    }

    pub fn find(&self, full: &str) -> Option<&ModelId> {
        self.models.iter().find(|m| m.full() == full)
    }

    pub fn by_provider(&self, provider: &str) -> Vec<&ModelId> {
        self.models.iter().filter(|m| m.provider == provider).collect()
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
