//! Model catalog for the model selector dialog.

/// Information about a model for the model selector dialog.
#[derive(Clone, Debug, PartialEq)]
pub struct ModelInfo {
    pub name: String,
    pub provider: String,
    pub display_name: String,
    pub cost_prompt: Option<f64>,
    pub cost_completion: Option<f64>,
    pub supports_thinking: bool,
    pub supports_vision: bool,
}

impl ModelInfo {
    pub fn new(provider: impl Into<String>, name: impl Into<String>) -> Self {
        let provider = provider.into();
        let name = name.into();
        Self {
            display_name: name.clone(),
            name,
            provider,
            cost_prompt: None,
            cost_completion: None,
            supports_thinking: false,
            supports_vision: false,
        }
    }

    pub fn with_cost(mut self, prompt: f64, completion: f64) -> Self {
        self.cost_prompt = Some(prompt);
        self.cost_completion = Some(completion);
        self
    }

    pub fn with_thinking(mut self) -> Self {
        self.supports_thinking = true;
        self
    }

    pub fn with_vision(mut self) -> Self {
        self.supports_vision = true;
        self
    }

    pub fn full(&self) -> String {
        format!("{}/{}", self.provider, self.name)
    }
}

/// Static catalog of known models for the model selector.
/// Derived from the provider registry but kept in core to avoid cross-crate deps.
#[allow(clippy::vec_init_then_push)]
pub fn model_catalog() -> Vec<ModelInfo> {
    let mut models = Vec::new();

    // Anthropic
    models.push(ModelInfo::new("anthropic", "claude-sonnet-4-6").with_cost(3.0, 15.0).with_thinking().with_vision());
    models.push(ModelInfo::new("anthropic", "claude-opus-4-7").with_cost(15.0, 75.0).with_thinking().with_vision());
    models.push(ModelInfo::new("anthropic", "claude-haiku-4-5").with_cost(0.25, 1.25).with_vision());

    // OpenAI
    models.push(ModelInfo::new("openai", "gpt-4o").with_cost(5.0, 15.0).with_vision());
    models.push(ModelInfo::new("openai", "gpt-4o-mini").with_cost(0.15, 0.6).with_vision());
    models.push(ModelInfo::new("openai", "gpt-5").with_cost(2.5, 10.0));
    models.push(ModelInfo::new("openai", "o3-mini").with_cost(1.1, 4.4).with_thinking());
    models.push(ModelInfo::new("openai", "o4-mini").with_cost(1.1, 4.4).with_thinking().with_vision());
    models.push(ModelInfo::new("openai", "o1").with_cost(15.0, 60.0).with_thinking().with_vision());
    models.push(ModelInfo::new("openai", "o3").with_cost(10.0, 40.0).with_thinking().with_vision());

    // Google
    models.push(ModelInfo::new("google", "gemini-2.5-pro").with_cost(1.25, 10.0).with_thinking().with_vision());
    models.push(ModelInfo::new("google", "gemini-2.5-flash").with_cost(0.15, 0.6).with_thinking().with_vision());
    models.push(ModelInfo::new("google", "gemini-2.0-flash").with_cost(0.1, 0.4).with_vision());

    // DeepSeek
    models.push(ModelInfo::new("deepseek", "deepseek-v4-flash").with_cost(0.27, 1.1).with_thinking());
    models.push(ModelInfo::new("deepseek", "deepseek-v4-pro").with_cost(1.54, 6.16).with_thinking());

    // Mistral
    models.push(ModelInfo::new("mistral", "mistral-large-latest").with_cost(2.0, 6.0));
    models.push(ModelInfo::new("mistral", "codestral-latest").with_cost(2.0, 6.0));
    models.push(ModelInfo::new("mistral", "devstral-latest").with_cost(2.0, 6.0));

    // Groq
    models.push(ModelInfo::new("groq", "llama-3.3-70b-versatile").with_cost(0.59, 0.79));
    models.push(ModelInfo::new("groq", "gemma2-9b-it").with_cost(0.2, 0.2));
    models.push(ModelInfo::new("groq", "mixtral-8x7b-32768").with_cost(0.24, 0.24));

    // Together
    models.push(ModelInfo::new("together", "meta-llama/Llama-3.3-70B-Instruct-Turbo").with_cost(0.88, 0.88));
    models.push(ModelInfo::new("together", "deepseek-ai/DeepSeek-V4-Pro").with_cost(1.25, 1.25).with_thinking());

    // Fireworks
    models.push(ModelInfo::new("fireworks", "accounts/fireworks/models/deepseek-v4-pro").with_cost(1.25, 1.25).with_thinking());
    models.push(ModelInfo::new("fireworks", "accounts/fireworks/models/kimi-k2p6").with_cost(2.0, 8.0));

    // OpenRouter
    models.push(ModelInfo::new("openrouter", "anthropic/claude-sonnet-4.6").with_cost(3.0, 15.0).with_thinking().with_vision());
    models.push(ModelInfo::new("openrouter", "openai/gpt-4o").with_cost(5.0, 15.0).with_vision());
    models.push(ModelInfo::new("openrouter", "google/gemini-2.5-pro").with_cost(1.25, 10.0).with_thinking().with_vision());
    models.push(ModelInfo::new("openrouter", "deepseek/deepseek-chat").with_cost(0.5, 2.0).with_thinking());
    models.push(ModelInfo::new("openrouter", "deepseek/deepseek-r1").with_cost(0.55, 2.19).with_thinking());

    // xAI
    models.push(ModelInfo::new("xai", "grok-3").with_cost(3.0, 15.0).with_vision());
    models.push(ModelInfo::new("xai", "grok-4.3").with_cost(5.0, 25.0).with_vision());

    // Ollama
    models.push(ModelInfo::new("ollama", "llama3.1"));
    models.push(ModelInfo::new("ollama", "qwen2.5-coder:7b"));
    models.push(ModelInfo::new("ollama", "mistral"));

    // Mock
    models.push(ModelInfo::new("mock", "echo"));

    models
}

/// Filter models by name, provider, or display name (case-insensitive).
pub fn filter_models(models: &[ModelInfo], query: &str) -> Vec<usize> {
    let q = query.to_lowercase();
    models
        .iter()
        .enumerate()
        .filter(|(_, m)| {
            m.name.to_lowercase().contains(&q)
                || m.provider.to_lowercase().contains(&q)
                || m.display_name.to_lowercase().contains(&q)
        })
        .map(|(i, _)| i)
        .collect()
}

/// Build grouped model selector items for the snapshot.
/// Returns (provider_header, display_name, cost_str, is_selected, is_current) tuples.
pub fn build_model_selector_items(
    models: &[ModelInfo],
    recent: &[String],
    filter: &str,
    current_provider: &str,
    current_model: &str,
) -> Vec<(String, String, String, bool, bool)> {
    let indices = if filter.is_empty() {
        (0..models.len()).collect()
    } else {
        filter_models(models, filter)
    };

    let mut items: Vec<(String, String, String, bool, bool)> = Vec::new();

    // Show recent section when no filter
    if filter.is_empty() && !recent.is_empty() {
        for r in recent.iter().rev().take(5) {
            if let Some(idx) = models.iter().position(|m| m.full() == *r) {
                let m = &models[idx];
                let cost = format_cost(m.cost_prompt, m.cost_completion);
                let is_current = m.provider == current_provider && m.name == current_model;
                items.push(("Recent".to_string(), m.full(), cost, false, is_current));
            }
        }
    }

    let mut last_provider = String::new();
    for &idx in &indices {
        let m = &models[idx];
        // Skip recent items in the main list to avoid duplication
        if filter.is_empty() && recent.contains(&m.full()) {
            continue;
        }
        let header = if m.provider != last_provider {
            last_provider = m.provider.clone();
            m.provider.clone()
        } else {
            String::new()
        };
        let cost = format_cost(m.cost_prompt, m.cost_completion);
        let is_current = m.provider == current_provider && m.name == current_model;
        items.push((header, m.full(), cost, false, is_current));
    }

    items
}

fn format_cost(prompt: Option<f64>, completion: Option<f64>) -> String {
    match (prompt, completion) {
        (Some(p), Some(c)) => format!("${:.2}/${:.2}", p, c),
        (Some(p), None) => format!("${:.2}/?", p),
        (None, Some(c)) => format!("?/${:.2}", c),
        (None, None) => String::new(),
    }
}
