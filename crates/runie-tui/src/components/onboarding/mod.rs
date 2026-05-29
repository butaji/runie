mod matrix_bg;
pub use matrix_bg::{render_onboarding_screen, MatrixRain};

pub mod builder;
pub use builder::*;

// ============================================================================
// Onboarding Types
// ============================================================================

#[derive(Debug, Clone, PartialEq)]
pub enum OnboardingStep {
    Welcome,
    ProviderSelect,
    KeyInput,
    ModelSelect,
    Complete,
}

#[derive(Debug, Clone)]
pub struct ProviderOption {
    pub name: String,
    pub id: String,
    pub description: String,
    pub key_prefix: String,
}

#[derive(Debug, Clone)]
pub struct ModelOption {
    pub name: String,
    pub id: String,
    pub description: String,
}

#[derive(Debug, Clone)]
pub struct Onboarding {
    pub step: OnboardingStep,
    pub selected_item: usize,
    pub selected_provider: Option<usize>,
    pub api_key_input: String,
    pub selected_model: Option<usize>,
    pub selected_models: Vec<usize>,  // multi-select for models
    pub providers: Vec<ProviderOption>,
    pub models: Vec<ModelOption>,
    pub error_message: Option<String>,
    pub fetch_error: Option<String>,
    pub search_query: String,
    pub filtered_provider_indices: Vec<usize>,
    pub filtered_model_indices: Vec<usize>,
    pub is_fetching_models: bool,
    pub matrix_rain: Option<MatrixRain>,
}

// ============================================================================
// Settings - provider, model, and API key configuration from onboarding
// ============================================================================

#[derive(Debug, Clone)]
pub struct Settings {
    pub provider_id: String,
    pub provider_name: String,
    pub model_id: String,
    pub model_name: String,
    pub api_key: String,
}

fn fuzzy_match(query: &str, target: &str) -> bool {
    if query.is_empty() { return true; }
    let target_lower = target.to_lowercase();
    let mut target_chars = target_lower.chars();
    for q in query.chars() {
        loop {
            match target_chars.next() {
                Some(t) if t == q => break,
                None => return false,
                _ => {}
            }
        }
    }
    true
}

// ============================================================================
// Model definitions per provider
// ============================================================================

pub fn get_openai_models() -> Vec<ModelOption> {
    let mut models = vec![
        ModelOption { name: "GPT-4o".to_string(), id: "gpt-4o".to_string(), description: "Most capable, multimodal flagship model".to_string() },
        ModelOption { name: "GPT-4o Mini".to_string(), id: "gpt-4o-mini".to_string(), description: "Fast, affordable small model".to_string() },
        ModelOption { name: "O1 Mini".to_string(), id: "o1-mini".to_string(), description: "Reasoning model optimized for code".to_string() },
    ];
    models.sort_by(|a, b| a.name.cmp(&b.name));
    models
}

pub fn get_anthropic_models() -> Vec<ModelOption> {
    let mut models = vec![
        ModelOption { name: "Claude Sonnet 4".to_string(), id: "claude-sonnet-4".to_string(), description: "Balanced performance and intelligence".to_string() },
        ModelOption { name: "Claude Haiku".to_string(), id: "claude-haiku".to_string(), description: "Fast, lightweight for simple tasks".to_string() },
        ModelOption { name: "Claude Opus".to_string(), id: "claude-opus".to_string(), description: "Most capable model for complex tasks".to_string() },
    ];
    models.sort_by(|a, b| a.name.cmp(&b.name));
    models
}

pub fn get_google_models() -> Vec<ModelOption> {
    let mut models = vec![
        ModelOption { name: "Gemini Pro".to_string(), id: "gemini-pro".to_string(), description: "Balanced multimodal model".to_string() },
        ModelOption { name: "Gemini Flash".to_string(), id: "gemini-flash".to_string(), description: "Fast, efficient for high-volume tasks".to_string() },
    ];
    models.sort_by(|a, b| a.name.cmp(&b.name));
    models
}

pub fn get_cohere_models() -> Vec<ModelOption> {
    let mut models = vec![
        ModelOption { name: "Command R".to_string(), id: "command-r".to_string(), description: "High quality RAG-optimized model".to_string() },
        ModelOption { name: "Command R Plus".to_string(), id: "command-r-plus".to_string(), description: "Most capable model for complex tasks".to_string() },
    ];
    models.sort_by(|a, b| a.name.cmp(&b.name));
    models
}

pub fn get_mistral_models() -> Vec<ModelOption> {
    let mut models = vec![
        ModelOption { name: "Mistral Large".to_string(), id: "mistral-large-latest".to_string(), description: "Flagship model for complex reasoning".to_string() },
    ];
    models.sort_by(|a, b| a.name.cmp(&b.name));
    models
}

pub fn get_deepseek_models() -> Vec<ModelOption> {
    let mut models = vec![
        ModelOption { name: "DeepSeek Chat".to_string(), id: "deepseek-chat".to_string(), description: "Efficient conversational model".to_string() },
    ];
    models.sort_by(|a, b| a.name.cmp(&b.name));
    models
}

pub fn get_groq_models() -> Vec<ModelOption> {
    let mut models = vec![
        ModelOption { name: "Llama 3.1 8B Instant".to_string(), id: "llama-3.1-8b-instant".to_string(), description: "Ultra-fast inference at low cost".to_string() },
    ];
    models.sort_by(|a, b| a.name.cmp(&b.name));
    models
}

pub fn get_openrouter_models() -> Vec<ModelOption> {
    let mut models = vec![
        ModelOption { name: "GPT-4o".to_string(), id: "openai/gpt-4o".to_string(), description: "Most capable multimodal model".to_string() },
    ];
    models.sort_by(|a, b| a.name.cmp(&b.name));
    models
}

pub fn get_huggingface_models() -> Vec<ModelOption> {
    let mut models = vec![
        ModelOption { name: "Llama 2 70B".to_string(), id: "meta-llama/Llama-2-70b-chat-hf".to_string(), description: "70B parameter open model".to_string() },
    ];
    models.sort_by(|a, b| a.name.cmp(&b.name));
    models
}

pub fn get_xai_models() -> Vec<ModelOption> {
    let mut models = vec![
        ModelOption { name: "Grok Beta".to_string(), id: "grok-beta".to_string(), description: "Real-time knowledge and reasoning".to_string() },
    ];
    models.sort_by(|a, b| a.name.cmp(&b.name));
    models
}

pub fn get_azure_models() -> Vec<ModelOption> {
    let mut models = vec![
        ModelOption { name: "GPT-4o".to_string(), id: "gpt-4o".to_string(), description: "Most capable multimodal model".to_string() },
    ];
    models.sort_by(|a, b| a.name.cmp(&b.name));
    models
}

pub fn get_moonshot_models() -> Vec<ModelOption> {
    let mut models = vec![
        ModelOption { name: "Moonshot V1 8K".to_string(), id: "moonshot-v1-8k".to_string(), description: "Long context conversational model".to_string() },
    ];
    models.sort_by(|a, b| a.name.cmp(&b.name));
    models
}

pub fn get_perplexity_models() -> Vec<ModelOption> {
    let mut models = vec![
        ModelOption { name: "Llama 3.1 Sonar Large".to_string(), id: "llama-3.1-sonar-large-128k-online".to_string(), description: "Online search-augmented model".to_string() },
    ];
    models.sort_by(|a, b| a.name.cmp(&b.name));
    models
}

pub fn get_ollama_models() -> Vec<ModelOption> {
    let mut models = vec![
        ModelOption { name: "Llama 3.2".to_string(), id: "llama3.2".to_string(), description: "Latest open-source model".to_string() },
    ];
    models.sort_by(|a, b| a.name.cmp(&b.name));
    models
}

pub fn get_hyperbolic_models() -> Vec<ModelOption> {
    let mut models = vec![
        ModelOption { name: "Llama 3.1 70B".to_string(), id: "meta-llama/Meta-Llama-3.1-70B-Instruct".to_string(), description: "High-quality open-source model".to_string() },
    ];
    models.sort_by(|a, b| a.name.cmp(&b.name));
    models
}

pub fn get_together_models() -> Vec<ModelOption> {
    let mut models = vec![
        ModelOption { name: "Llama 3.2 3B Turbo".to_string(), id: "meta-llama/Llama-3.2-3B-Instruct-Turbo".to_string(), description: "Fast, efficient instruction model".to_string() },
    ];
    models.sort_by(|a, b| a.name.cmp(&b.name));
    models
}

pub fn get_zai_models() -> Vec<ModelOption> {
    let mut models = vec![
        ModelOption { name: "Default".to_string(), id: "default-model".to_string(), description: "Default model for Zai".to_string() },
    ];
    models.sort_by(|a, b| a.name.cmp(&b.name));
    models
}

/// Returns all model names from all providers, sorted alphabetically
pub fn get_all_model_names() -> Vec<String> {
    let all_models: Vec<String> = [
        get_openai_models(),
        get_anthropic_models(),
        get_google_models(),
        get_cohere_models(),
        get_mistral_models(),
        get_deepseek_models(),
        get_groq_models(),
        get_openrouter_models(),
        get_huggingface_models(),
        get_xai_models(),
        get_azure_models(),
        get_moonshot_models(),
        get_perplexity_models(),
        get_ollama_models(),
        get_hyperbolic_models(),
        get_together_models(),
        get_zai_models(),
        get_minimax_models(),
        get_mira_models(),
        get_galadriel_models(),
        get_llamafile_models(),
    ].iter()
    .flat_map(|models| models.iter().map(|m| m.name.clone()))
    .collect();
    
    let mut sorted = all_models;
    sorted.sort();
    sorted.dedup();
    sorted
}

pub fn get_minimax_models() -> Vec<ModelOption> {
    let mut models = vec![
        ModelOption { name: "ABAB 6.5".to_string(), id: "abab6.5-chat".to_string(), description: "MiniMax chat model".to_string() },
    ];
    models.sort_by(|a, b| a.name.cmp(&b.name));
    models
}

pub fn get_mira_models() -> Vec<ModelOption> {
    let mut models = vec![
        ModelOption { name: "Default".to_string(), id: "default-model".to_string(), description: "Default model for Mira".to_string() },
    ];
    models.sort_by(|a, b| a.name.cmp(&b.name));
    models
}

pub fn get_galadriel_models() -> Vec<ModelOption> {
    let mut models = vec![
        ModelOption { name: "Default".to_string(), id: "default-model".to_string(), description: "Default model for Galadriel".to_string() },
    ];
    models.sort_by(|a, b| a.name.cmp(&b.name));
    models
}

pub fn get_llamafile_models() -> Vec<ModelOption> {
    let mut models = vec![
        ModelOption { name: "Llamafile".to_string(), id: "llamafile".to_string(), description: "Local llamafile model".to_string() },
    ];
    models.sort_by(|a, b| a.name.cmp(&b.name));
    models
}

// ============================================================================
// Onboarding Implementation
// ============================================================================

impl Onboarding {
    pub fn new(mock_mode: bool) -> Self {
        let mut providers = get_default_providers();
        if mock_mode {
            providers.insert(0, ProviderOption {
                name: "Mock".to_string(),
                id: "mock".to_string(),
                description: "Mock provider for testing (no API calls)".to_string(),
                key_prefix: "mock-".to_string(),
            });
        }
        Self {
            step: OnboardingStep::Welcome,
            selected_item: 0,
            selected_provider: None,
            api_key_input: String::new(),
            selected_model: None,
            selected_models: Vec::new(),
            providers,
            models: Vec::new(),
            error_message: None,
            fetch_error: None,
            search_query: String::new(),
            filtered_provider_indices: Vec::new(),
            filtered_model_indices: Vec::new(),
            is_fetching_models: false,
            matrix_rain: None,
        }
    }

    pub fn next_step(&mut self) {
        self.error_message = None;
        self.step = match &self.step {
            OnboardingStep::Welcome => OnboardingStep::ProviderSelect,
            OnboardingStep::ProviderSelect => {
                if self.selected_provider.is_some() { OnboardingStep::KeyInput }
                else { self.error_message = Some("Please select a provider".to_string()); OnboardingStep::ProviderSelect }
            }
            OnboardingStep::KeyInput => {
                // P0-3 FIX: Provide specific validation errors
                match self.validate_key_detailed() {
                    Ok(()) => OnboardingStep::ModelSelect,
                    Err(specific_error) => {
                        self.error_message = Some(specific_error);
                        OnboardingStep::KeyInput
                    }
                }
            }
            OnboardingStep::ModelSelect => {
                if self.selected_model.is_some() { OnboardingStep::Complete }
                else { self.error_message = Some("Please select a model".to_string()); OnboardingStep::ModelSelect }
            }
            OnboardingStep::Complete => OnboardingStep::Complete,
        };
        self.enter_step();
    }

    pub fn prev_step(&mut self) {
        self.error_message = None;
        self.step = match &self.step {
            OnboardingStep::Welcome => OnboardingStep::Welcome,
            OnboardingStep::ProviderSelect => OnboardingStep::Welcome,
            OnboardingStep::KeyInput => OnboardingStep::ProviderSelect,
            OnboardingStep::ModelSelect => OnboardingStep::KeyInput,
            OnboardingStep::Complete => OnboardingStep::ModelSelect,
        };
        self.enter_step();
    }

    pub fn enter_step(&mut self) {
        match self.step {
            OnboardingStep::ProviderSelect => {
                if self.search_query.is_empty() {
                    if self.filtered_provider_indices.is_empty() {
                        self.filtered_provider_indices = (0..self.providers.len()).collect();
                    }
                } else {
                    // Restore filter when returning with an active search
                    self.filtered_provider_indices = self.providers.iter().enumerate()
                        .filter(|(_, p)| fuzzy_match(&self.search_query, &p.name))
                        .map(|(i, _)| i)
                        .collect();
                }
                self.selected_item = self.selected_item.min(self.filtered_provider_indices.len().saturating_sub(1));
            }
            OnboardingStep::ModelSelect => {
                if self.search_query.is_empty() {
                    if self.filtered_model_indices.is_empty() {
                        self.filtered_model_indices = (0..self.models.len()).collect();
                    }
                } else {
                    // Restore filter when returning with an active search
                    self.filtered_model_indices = self.models.iter().enumerate()
                        .filter(|(_, m)| fuzzy_match(&self.search_query, &m.name))
                        .map(|(i, _)| i)
                        .collect();
                }
                self.selected_item = self.selected_item.min(self.filtered_model_indices.len().saturating_sub(1));
            }
            _ => {}
        }
    }

    pub fn update_search(&mut self, query: &str) {
        self.search_query = query.to_lowercase();
        match self.step {
            OnboardingStep::ProviderSelect => {
                self.filtered_provider_indices = self.providers.iter().enumerate()
                    .filter(|(_, p)| fuzzy_match(&self.search_query, &p.name))
                    .map(|(i, _)| i)
                    .collect();
                self.selected_item = 0;
            }
            OnboardingStep::ModelSelect => {
                self.filtered_model_indices = self.models.iter().enumerate()
                    .filter(|(_, m)| fuzzy_match(&self.search_query, &m.name))
                    .map(|(i, _)| i)
                    .collect();
                self.selected_item = 0;
            }
            _ => {}
        }
    }

    pub fn clear_search(&mut self) {
        self.search_query.clear();
        self.filtered_provider_indices.clear();
        self.filtered_model_indices.clear();
    }

    // P1-1 FIX: Set fetch error when model listing fails
    pub fn set_fetch_error(&mut self, err: String) {
        self.fetch_error = Some(err);
        self.is_fetching_models = false;
    }

    // P1-1 FIX: Clear fetch error
    pub fn clear_fetch_error(&mut self) {
        self.fetch_error = None;
    }

    pub fn select_provider(&mut self, index: usize) {
        if let Some(&real_index) = self.filtered_provider_indices.get(index) {
            self.selected_provider = Some(real_index);
            self.selected_model = None;
            self.selected_models.clear();
            let provider = &self.providers[real_index];
            // Pre-fill mock API key for mock provider
            if provider.id == "mock" {
                self.api_key_input = "mock-api-key-for-testing".to_string();
            }
            self.models = match provider.id.as_str() {
                "openai" => get_openai_models(),
                "anthropic" => get_anthropic_models(),
                "google" => get_google_models(),
                "cohere" => get_cohere_models(),
                "mistral" => get_mistral_models(),
                "deepseek" => get_deepseek_models(),
                "groq" => get_groq_models(),
                "openrouter" => get_openrouter_models(),
                "huggingface" => get_huggingface_models(),
                "xai" => get_xai_models(),
                "azure" => get_azure_models(),
                "moonshot" => get_moonshot_models(),
                "perplexity" => get_perplexity_models(),
                "ollama" => get_ollama_models(),
                "hyperbolic" => get_hyperbolic_models(),
                "together" => get_together_models(),
                "zai" => get_zai_models(),
                "minimax" => get_minimax_models(),
                "mira" => get_mira_models(),
                "galadriel" => get_galadriel_models(),
                "llamafile" => get_llamafile_models(),
                "mock" => vec![
                    ModelOption { name: "Mock GPT-4".to_string(), id: "mock-gpt-4".to_string(), description: "Mock model for testing".to_string() },
                    ModelOption { name: "Mock Claude".to_string(), id: "mock-claude".to_string(), description: "Mock model for testing".to_string() },
                ],
                _ => Vec::new(),
            };
            self.error_message = None;
        }
    }

    pub fn select_model(&mut self, index: usize) {
        if let Some(&real_index) = self.filtered_model_indices.get(index) {
            // Toggle in multi-select list
            if let Some(pos) = self.selected_models.iter().position(|&x| x == real_index) {
                self.selected_models.remove(pos);
            } else {
                self.selected_models.push(real_index);
            }
            // Keep selected_model in sync with first selected (or None if empty)
            self.selected_model = self.selected_models.first().copied();
            self.error_message = None;
        }
    }

    pub fn navigate_up(&mut self) {
        let max = match &self.step {
            OnboardingStep::Welcome => 0,
            OnboardingStep::ProviderSelect => self.get_filtered_provider_count().saturating_sub(1),
            OnboardingStep::ModelSelect => self.get_filtered_model_count().saturating_sub(1),
            OnboardingStep::KeyInput => 0,
            OnboardingStep::Complete => 1,  // yes/no = 2 options, max index = 1
        };
        if self.selected_item > 0 {
            self.selected_item -= 1;
            self.selected_item = self.selected_item.min(max);
        }
    }

    pub fn navigate_down(&mut self) {
        let max = match &self.step {
            OnboardingStep::Welcome => 0,
            OnboardingStep::ProviderSelect => self.get_filtered_provider_count().saturating_sub(1),
            OnboardingStep::ModelSelect => self.get_filtered_model_count().saturating_sub(1),
            OnboardingStep::KeyInput => 0,
            OnboardingStep::Complete => 1,  // yes/no = 2 options, max index = 1
        };
        if self.selected_item < max {
            self.selected_item += 1;
        }
    }

    pub fn get_filtered_provider_count(&self) -> usize {
        if self.filtered_provider_indices.is_empty() && self.search_query.is_empty() {
            // Not initialized yet - return all providers
            self.providers.len()
        } else {
            // Either has matches or actively searched with no results
            self.filtered_provider_indices.len()
        }
    }

    pub fn get_filtered_model_count(&self) -> usize {
        if self.filtered_model_indices.is_empty() && self.search_query.is_empty() {
            // Not initialized yet - return all models
            self.models.len()
        } else {
            // Either has matches or actively searched with no results
            self.filtered_model_indices.len()
        }
    }

    pub fn get_selected_provider_index(&self) -> Option<usize> {
        self.selected_provider
    }

    pub fn get_selected_model_index(&self) -> Option<usize> {
        self.selected_model
    }

    pub fn get_selected_item(&self) -> usize {
        self.selected_item
    }

    pub fn validate_key(&self) -> bool {
        if self.api_key_input.trim().is_empty() { return false; }
        if let Some(provider_idx) = self.selected_provider {
            let provider = &self.providers[provider_idx];
            let prefix = &provider.key_prefix;
            if prefix.is_empty() { return true; }
            return self.api_key_input.starts_with(prefix);
        }
        false
    }
    
    /// P0-3 FIX: Detailed validation - checks empty key
    fn validate_key_empty(&self) -> Result<(), String> {
        if self.api_key_input.trim().is_empty() {
            return Err("API key cannot be empty.".to_string());
        }
        Ok(())
    }
    
    /// P0-3 FIX: Detailed validation - checks provider and key format
    fn validate_key_format(&self) -> Result<(), String> {
        let key = self.api_key_input.trim();
        let provider_idx = self.selected_provider
            .ok_or_else(|| "No provider selected".to_string())?;
        let provider = &self.providers[provider_idx];
        // Length check - be lenient for short test keys
        let min_len = if provider.key_prefix.is_empty() { 1 } else { 1.max(provider.key_prefix.len()) };
        if key.len() < min_len {
            return Err(format!("Key for {} appears too short.", provider.name));
        }
        // Prefix check (if required)
        if !provider.key_prefix.is_empty() && !key.starts_with(&provider.key_prefix) {
            let preview = if key.len() >= 8 { &key[..8] } else { key };
            return Err(format!(
                "Key for {} must start with '{}' (yours: '{}...')",
                provider.name, provider.key_prefix, preview
            ));
        }
        Ok(())
    }
    
    /// P0-3 FIX: Detailed validation with specific error messages
    pub fn validate_key_detailed(&self) -> Result<(), String> {
        self.validate_key_empty()?;
        self.validate_key_format()
    }

    pub fn is_complete(&self) -> bool {
        self.step == OnboardingStep::Complete && self.selected_provider.is_some() && (!self.selected_models.is_empty() || self.selected_model.is_some()) && !self.api_key_input.trim().is_empty()
    }

    pub fn to_settings(&self) -> Option<Settings> {
        let provider_idx = self.selected_provider?;
        let model_idx = self.selected_models.first().copied().or(self.selected_model)?;
        let provider = self.providers.get(provider_idx)?;
        let model = self.models.get(model_idx)?;
        Some(Settings {
            provider_id: provider.id.clone(),
            provider_name: provider.name.clone(),
            model_id: model.id.clone(),
            model_name: model.name.clone(),
            api_key: self.api_key_input.clone(),
        })
    }

    pub fn get_current_provider(&self) -> Option<&ProviderOption> {
        self.selected_provider.and_then(|i| self.providers.get(i))
    }

    pub fn get_current_model(&self) -> Option<&ModelOption> {
        self.selected_model.and_then(|i| self.models.get(i))
    }
}

impl Default for Onboarding {
    fn default() -> Self { Self::new(false) }
}

/// Returns the default list of providers, sorted alphabetically by name.
fn get_default_providers() -> Vec<ProviderOption> {
    let mut providers = vec![
        ProviderOption { name: "OpenAI".to_string(), id: "openai".to_string(), description: "GPT-4o family of models".to_string(), key_prefix: "sk-".to_string() },
        ProviderOption { name: "Anthropic".to_string(), id: "anthropic".to_string(), description: "Claude family of models".to_string(), key_prefix: "sk-ant-".to_string() },
        ProviderOption { name: "Google".to_string(), id: "google".to_string(), description: "Gemini family of models".to_string(), key_prefix: String::new() },
        ProviderOption { name: "Cohere".to_string(), id: "cohere".to_string(), description: "Command R family of models".to_string(), key_prefix: String::new() },
        ProviderOption { name: "Mistral".to_string(), id: "mistral".to_string(), description: "Mistral AI models".to_string(), key_prefix: String::new() },
        ProviderOption { name: "DeepSeek".to_string(), id: "deepseek".to_string(), description: "DeepSeek models".to_string(), key_prefix: String::new() },
        ProviderOption { name: "Groq".to_string(), id: "groq".to_string(), description: "Fast inference with Llama".to_string(), key_prefix: String::new() },
        ProviderOption { name: "OpenRouter".to_string(), id: "openrouter".to_string(), description: "Access multiple models via OpenRouter".to_string(), key_prefix: String::new() },
        ProviderOption { name: "HuggingFace".to_string(), id: "huggingface".to_string(), description: "Open-source models".to_string(), key_prefix: String::new() },
        ProviderOption { name: "xAI".to_string(), id: "xai".to_string(), description: "Grok models".to_string(), key_prefix: String::new() },
        ProviderOption { name: "Azure".to_string(), id: "azure".to_string(), description: "Microsoft Azure OpenAI".to_string(), key_prefix: String::new() },
        ProviderOption { name: "Moonshot".to_string(), id: "moonshot".to_string(), description: "Moonshot AI models".to_string(), key_prefix: String::new() },
        ProviderOption { name: "Perplexity".to_string(), id: "perplexity".to_string(), description: "Online search-augmented models".to_string(), key_prefix: String::new() },
        ProviderOption { name: "Ollama".to_string(), id: "ollama".to_string(), description: "Local model inference".to_string(), key_prefix: String::new() },
        ProviderOption { name: "Hyperbolic".to_string(), id: "hyperbolic".to_string(), description: "Open-source models at low cost".to_string(), key_prefix: String::new() },
        ProviderOption { name: "Together".to_string(), id: "together".to_string(), description: "Together AI models".to_string(), key_prefix: String::new() },
        ProviderOption { name: "ZAI".to_string(), id: "zai".to_string(), description: "ZAI models".to_string(), key_prefix: String::new() },
        ProviderOption { name: "MiniMax".to_string(), id: "minimax".to_string(), description: "MiniMax AI models".to_string(), key_prefix: String::new() },
        ProviderOption { name: "Mira".to_string(), id: "mira".to_string(), description: "Mira models".to_string(), key_prefix: String::new() },
        ProviderOption { name: "Galadriel".to_string(), id: "galadriel".to_string(), description: "Galadriel models".to_string(), key_prefix: String::new() },
        ProviderOption { name: "Llamafile".to_string(), id: "llamafile".to_string(), description: "Local llamafile models".to_string(), key_prefix: String::new() },
    ];
    providers.sort_by(|a, b| a.name.cmp(&b.name));
    providers
}

pub mod render;
#[cfg(test)]
mod tests;
#[cfg(test)]
mod comprehensive_tests;
