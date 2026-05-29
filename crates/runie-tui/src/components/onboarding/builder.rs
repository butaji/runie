use super::{Onboarding, OnboardingStep};

pub struct OnboardingBuilder {
    step: OnboardingStep,
    provider_id: Option<String>,
    model: Option<String>,
    api_key: Option<String>,
}

impl OnboardingBuilder {
    pub fn new() -> Self {
        Self {
            step: OnboardingStep::Welcome,
            provider_id: None,
            model: None,
            api_key: None,
        }
    }

    pub fn welcome(mut self) -> Self {
        self.step = OnboardingStep::Welcome;
        self
    }

    pub fn provider(mut self, _name: &str, id: &str) -> Self {
        self.provider_id = Some(id.to_string());
        self
    }

    pub fn model(mut self, name: &str) -> Self {
        self.model = Some(name.to_string());
        self
    }

    pub fn key(mut self, key: &str) -> Self {
        self.api_key = Some(key.to_string());
        self
    }

    pub fn build(self) -> Onboarding {
        let mut onboarding = Onboarding::new(false);
        onboarding.step = self.step.clone();
        apply_provider(&mut onboarding, &self.provider_id);
        if let Some(key) = self.api_key {
            onboarding.api_key_input = key;
        }
        if self.step == OnboardingStep::Complete {
            if let Some(model_name) = &self.model {
                if let Some(idx) = onboarding.models.iter().position(|m| m.name == *model_name) {
                    onboarding.select_model(idx);
                }
            }
        }
        onboarding
    }
}

fn apply_provider(onboarding: &mut Onboarding, provider_id: &Option<String>) {
    if let Some(id) = provider_id {
        if let Some(idx) = onboarding.providers.iter().position(|p| p.id == *id) {
            onboarding.select_provider(idx);
        }
    }
}

impl Default for OnboardingBuilder {
    fn default() -> Self {
        Self::new()
    }
}
