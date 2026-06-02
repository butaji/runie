// ============================================================================
// State Management
// ============================================================================

use super::{
    fuzzy_match, get_default_providers, Onboarding, OnboardingStep,
    ProviderOption, ModelOption, Settings,
};

// Lookup table for provider → models function
type ModelProviderFn = fn() -> Vec<ModelOption>;

const MODEL_LOOKUP: &[(&str, ModelProviderFn)] = &[
    ("openai", super::get_openai_models),
    ("anthropic", super::get_anthropic_models),
    ("google", super::get_google_models),
    ("cohere", super::get_cohere_models),
    ("mistral", super::get_mistral_models),
    ("deepseek", super::get_deepseek_models),
    ("groq", super::get_groq_models),
    ("openrouter", super::get_openrouter_models),
    ("huggingface", super::get_huggingface_models),
    ("xai", super::get_xai_models),
    ("azure", super::get_azure_models),
    ("moonshot", super::get_moonshot_models),
    ("perplexity", super::get_perplexity_models),
    ("ollama", super::get_ollama_models),
    ("hyperbolic", super::get_hyperbolic_models),
    ("together", super::get_together_models),
    ("zai", super::get_zai_models),
    ("minimax", super::get_minimax_models),
    ("mira", super::get_mira_models),
    ("galadriel", super::get_galadriel_models),
    ("llamafile", super::get_llamafile_models),
];

impl Onboarding {

    #[must_use]
    
    pub fn new(mock_mode: bool) -> Self {
        let mut providers = get_default_providers();
        if mock_mode {
            providers.insert(
                0,
                ProviderOption {
                    name: "Mock".to_string(),
                    id: "mock".to_string(),
                    description: "Mock provider for testing (no API calls)".to_string(),
                    key_prefix: "mock-".to_string(),
                },
            );
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
        let (next_step, err) = Self::transition_step(
            &self.step,
            self.selected_provider,
            self.selected_model,
            || self.validate_key_detailed(),
        );
        if let Some(e) = err {
            self.error_message = Some(e);
        }
        self.step = next_step;
        self.enter_step();
    }

    /// Computes the next step based on current step and selections.
    /// Returns (next_step, optional_error_message).
    fn transition_step<F>(
        step: &OnboardingStep,
        selected_provider: Option<usize>,
        selected_model: Option<usize>,
        validate_key: F,
    ) -> (OnboardingStep, Option<String>)
    where
        F: FnOnce() -> Result<(), String>,
    {
        match step {
            OnboardingStep::Welcome => (OnboardingStep::ProviderSelect, None),
            OnboardingStep::ProviderSelect => {
                if selected_provider.is_some() {
                    (OnboardingStep::KeyInput, None)
                } else {
                    (OnboardingStep::ProviderSelect, Some("Please select a provider".to_string()))
                }
            }
            OnboardingStep::KeyInput => match validate_key() {
                Ok(()) => (OnboardingStep::ModelSelect, None),
                Err(specific_error) => (OnboardingStep::KeyInput, Some(specific_error)),
            },
            OnboardingStep::ModelSelect => {
                if selected_model.is_some() {
                    (OnboardingStep::Complete, None)
                } else {
                    (OnboardingStep::ModelSelect, Some("Please select a model".to_string()))
                }
            }
            OnboardingStep::Complete => (OnboardingStep::Complete, None),
        }
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
            OnboardingStep::ProviderSelect => self.enter_provider_select(),
            OnboardingStep::ModelSelect => self.enter_model_select(),
            _ => {}
        }
    }

    fn enter_provider_select(&mut self) {
        if self.search_query.is_empty() {
            if self.filtered_provider_indices.is_empty() {
                self.filtered_provider_indices = (0..self.providers.len()).collect();
            }
        } else {
            self.filtered_provider_indices = self
                .providers
                .iter()
                .enumerate()
                .filter(|(_, p)| fuzzy_match(&self.search_query, &p.name))
                .map(|(i, _)| i)
                .collect();
        }
        self.selected_item = self
            .selected_item
            .min(self.filtered_provider_indices.len().saturating_sub(1));
    }

    fn enter_model_select(&mut self) {
        if self.search_query.is_empty() {
            if self.filtered_model_indices.is_empty() {
                self.filtered_model_indices = (0..self.models.len()).collect();
            }
        } else {
            self.filtered_model_indices = self
                .models
                .iter()
                .enumerate()
                .filter(|(_, m)| fuzzy_match(&self.search_query, &m.name))
                .map(|(i, _)| i)
                .collect();
        }
        self.selected_item = self
            .selected_item
            .min(self.filtered_model_indices.len().saturating_sub(1));
    }

    pub fn update_search(&mut self, query: &str) {
        self.search_query = query.to_lowercase();
        match self.step {
            OnboardingStep::ProviderSelect => {
                self.filtered_provider_indices = self
                    .providers
                    .iter()
                    .enumerate()
                    .filter(|(_, p)| fuzzy_match(&self.search_query, &p.name))
                    .map(|(i, _)| i)
                    .collect();
                self.selected_item = 0;
            }
            OnboardingStep::ModelSelect => {
                self.filtered_model_indices = self
                    .models
                    .iter()
                    .enumerate()
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

    pub fn set_fetch_error(&mut self, err: String) {
        self.fetch_error = Some(err);
        self.is_fetching_models = false;
    }

    pub fn clear_fetch_error(&mut self) {
        self.fetch_error = None;
    }

    pub fn select_provider(&mut self, index: usize) {
        if let Some(&real_index) = self.filtered_provider_indices.get(index) {
            self.selected_provider = Some(real_index);
            self.selected_model = None;
            self.selected_models.clear();
            let provider = &self.providers[real_index];
            if provider.id == "mock" {
                self.api_key_input = "mock-api-key-for-testing".to_string();
            }
            self.models = self.get_models_for_provider(&provider.id);
            self.error_message = None;
        }
    }

    fn get_models_for_provider(&self, provider_id: &str) -> Vec<ModelOption> {
        MODEL_LOOKUP
            .iter()
            .find(|(id, _)| *id == provider_id)
            .map(|(_, f)| f())
            .unwrap_or_else(|| self.get_mock_models_for_provider(provider_id))
    }

    fn get_mock_models_for_provider(&self, provider_id: &str) -> Vec<ModelOption> {
        if provider_id == "mock" {
            vec![
                ModelOption {
                    name: "Mock GPT-4".to_string(),
                    id: "mock-gpt-4".to_string(),
                    description: "Mock model for testing".to_string(),
                },
                ModelOption {
                    name: "Mock Claude".to_string(),
                    id: "mock-claude".to_string(),
                    description: "Mock model for testing".to_string(),
                },
            ]
        } else {
            Vec::new()
        }
    }

    pub fn select_model(&mut self, index: usize) {
        if let Some(&real_index) = self.filtered_model_indices.get(index) {
            if let Some(pos) = self.selected_models.iter().position(|&x| x == real_index) {
                self.selected_models.remove(pos);
            } else {
                self.selected_models.push(real_index);
            }
            self.selected_model = self.selected_models.first().copied();
            self.error_message = None;
        }
    }

    pub fn validate_key(&self) -> bool {
        if self.api_key_input.trim().is_empty() {
            return false;
        }
        if let Some(provider_idx) = self.selected_provider {
            let provider = &self.providers[provider_idx];
            let prefix = &provider.key_prefix;
            if prefix.is_empty() {
                return true;
            }
            return self.api_key_input.starts_with(prefix);
        }
        false
    }

    fn validate_key_empty(&self) -> Result<(), String> {
        if self.api_key_input.trim().is_empty() {
            return Err("API key cannot be empty.".to_string());
        }
        Ok(())
    }

    fn validate_key_format(&self) -> Result<(), String> {
        let key = self.api_key_input.trim();
        let provider_idx = self
            .selected_provider
            .ok_or_else(|| "No provider selected".to_string())?;
        let provider = &self.providers[provider_idx];

        self.validate_key_length(key, provider)?;
        self.validate_key_prefix(key, provider)
    }

    fn validate_key_length(&self, key: &str, provider: &ProviderOption) -> Result<(), String> {
        let min_len = if provider.key_prefix.is_empty() {
            1
        } else {
            provider.key_prefix.len().max(1)
        };
        if key.len() < min_len {
            return Err(format!("Key for {} appears too short.", provider.name));
        }
        Ok(())
    }

    fn validate_key_prefix(&self, key: &str, provider: &ProviderOption) -> Result<(), String> {
        if provider.key_prefix.is_empty() {
            return Ok(());
        }
        if !key.starts_with(&provider.key_prefix) {
            let preview = if key.len() >= 8 { &key[..8] } else { key };
            return Err(format!(
                "Key for {} must start with '{}' (yours: '{}...')",
                provider.name, provider.key_prefix, preview
            ));
        }
        Ok(())
    }

    pub fn validate_key_detailed(&self) -> Result<(), String> {
        self.validate_key_empty()?;
        self.validate_key_format()
    }

    pub fn is_complete(&self) -> bool {
        self.step == OnboardingStep::Complete
            && self.selected_provider.is_some()
            && (!self.selected_models.is_empty() || self.selected_model.is_some())
            && !self.api_key_input.trim().is_empty()
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
    fn default() -> Self {
        Self::new(false)
    }
}
