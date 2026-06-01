use crate::tui::view_models::{OnboardingStep, OnboardingViewModel};

pub(crate) struct OnboardingBuilder {
    step: OnboardingStep,
    selected_item: usize,
    selected_provider: Option<usize>,
    api_key_input: String,
    selected_model: Option<usize>,
    providers: Vec<String>,
    models: Vec<String>,
    error_message: Option<String>,
}

impl OnboardingBuilder {
    pub(crate) fn new() -> Self {
        Self {
            step: OnboardingStep::Welcome,
            selected_item: 0,
            selected_provider: None,
            api_key_input: String::new(),
            selected_model: None,
            providers: Vec::new(),
            models: Vec::new(),
            error_message: None,
        }
    }

    pub(crate) fn welcome(mut self) -> Self {
        self.step = OnboardingStep::Welcome;
        self
    }

    pub(crate) fn provider(self, _name: &str, _id: &str) -> Self {
        // Provider selection is done via index in ViewModel
        // Caller should use providers() method to set the list
        self
    }

    pub(crate) fn model(self, _name: &str) -> Self {
        // Model selection is done via index in ViewModel
        // Caller should use models() method to set the list
        self
    }

    pub(crate) fn step(mut self, step: OnboardingStep) -> Self {
        self.step = step;
        self
    }

    pub(crate) fn selected_item(mut self, item: usize) -> Self {
        self.selected_item = item;
        self
    }

    pub(crate) fn selected_provider(mut self, provider: Option<usize>) -> Self {
        self.selected_provider = provider;
        self
    }

    pub(crate) fn selected_model(mut self, model: Option<usize>) -> Self {
        self.selected_model = model;
        self
    }

    pub(crate) fn key(mut self, key: &str) -> Self {
        self.api_key_input = key.to_string();
        self
    }

    pub(crate) fn providers(mut self, providers: Vec<String>) -> Self {
        self.providers = providers;
        self
    }

    pub(crate) fn models(mut self, models: Vec<String>) -> Self {
        self.models = models;
        self
    }

    pub(crate) fn error_message(mut self, msg: &str) -> Self {
        self.error_message = Some(msg.to_string());
        self
    }

    pub(crate) fn build(self) -> OnboardingViewModel {
        OnboardingViewModel {
            step: self.step,
            selected_item: self.selected_item,
            selected_provider: self.selected_provider,
            api_key_input: self.api_key_input,
            selected_model: self.selected_model,
            providers: self.providers,
            models: self.models,
            error_message: self.error_message,
        }
    }
}

impl Default for OnboardingBuilder {
    fn default() -> Self {
        Self::new()
    }
}