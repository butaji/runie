use std::collections::HashMap;

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
    pub selected_provider: Option<usize>,
    pub api_key_input: String,
    pub selected_model: Option<usize>,
    pub providers: Vec<ProviderOption>,
    pub models: Vec<ModelOption>,
    pub error_message: Option<String>,
}

// ============================================================================
// Settings (placeholder for integration with app settings)
// ============================================================================

#[derive(Debug, Clone)]
pub struct Settings {
    pub provider_id: String,
    pub provider_name: String,
    pub model_id: String,
    pub model_name: String,
    pub api_key: String,
}

// ============================================================================
// Model definitions per provider
// ============================================================================

fn get_openai_models() -> Vec<ModelOption> {
    vec![
        ModelOption {
            name: "GPT-4o".to_string(),
            id: "gpt-4o".to_string(),
            description: "Most capable, multimodal flagship model".to_string(),
        },
        ModelOption {
            name: "GPT-4o Mini".to_string(),
            id: "gpt-4o-mini".to_string(),
            description: "Fast, affordable small model".to_string(),
        },
        ModelOption {
            name: "O1 Mini".to_string(),
            id: "o1-mini".to_string(),
            description: "Reasoning model optimized for code".to_string(),
        },
    ]
}

fn get_anthropic_models() -> Vec<ModelOption> {
    vec![
        ModelOption {
            name: "Claude Sonnet 4".to_string(),
            id: "claude-sonnet-4".to_string(),
            description: "Balanced performance and intelligence".to_string(),
        },
        ModelOption {
            name: "Claude Haiku".to_string(),
            id: "claude-haiku".to_string(),
            description: "Fast, lightweight for simple tasks".to_string(),
        },
        ModelOption {
            name: "Claude Opus".to_string(),
            id: "claude-opus".to_string(),
            description: "Most capable model for complex tasks".to_string(),
        },
    ]
}

fn get_google_models() -> Vec<ModelOption> {
    vec![
        ModelOption {
            name: "Gemini Pro".to_string(),
            id: "gemini-pro".to_string(),
            description: "Balanced multimodal model".to_string(),
        },
        ModelOption {
            name: "Gemini Flash".to_string(),
            id: "gemini-flash".to_string(),
            description: "Fast, efficient for high-volume tasks".to_string(),
        },
    ]
}

// ============================================================================
// Onboarding Implementation
// ============================================================================

impl Onboarding {
    pub fn new() -> Self {
        let providers = vec![
            ProviderOption {
                name: "OpenAI".to_string(),
                id: "openai".to_string(),
                description: "GPT-4o family of models".to_string(),
                key_prefix: "sk-".to_string(),
            },
            ProviderOption {
                name: "Anthropic".to_string(),
                id: "anthropic".to_string(),
                description: "Claude family of models".to_string(),
                key_prefix: "sk-ant-".to_string(),
            },
            ProviderOption {
                name: "Google".to_string(),
                id: "google".to_string(),
                description: "Gemini family of models".to_string(),
                key_prefix: String::new(),
            },
        ];

        Self {
            step: OnboardingStep::Welcome,
            selected_provider: None,
            api_key_input: String::new(),
            selected_model: None,
            providers,
            models: Vec::new(),
            error_message: None,
        }
    }

    pub fn next_step(&mut self) {
        self.error_message = None;
        self.step = match &self.step {
            OnboardingStep::Welcome => OnboardingStep::ProviderSelect,
            OnboardingStep::ProviderSelect => {
                if self.selected_provider.is_some() {
                    OnboardingStep::KeyInput
                } else {
                    self.error_message = Some("Please select a provider".to_string());
                    OnboardingStep::ProviderSelect
                }
            }
            OnboardingStep::KeyInput => {
                if self.validate_key() {
                    OnboardingStep::ModelSelect
                } else {
                    self.error_message = Some("Invalid API key format".to_string());
                    OnboardingStep::KeyInput
                }
            }
            OnboardingStep::ModelSelect => {
                if self.selected_model.is_some() {
                    OnboardingStep::Complete
                } else {
                    self.error_message = Some("Please select a model".to_string());
                    OnboardingStep::ModelSelect
                }
            }
            OnboardingStep::Complete => OnboardingStep::Complete,
        };
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
    }

    pub fn select_provider(&mut self, index: usize) {
        if index < self.providers.len() {
            self.selected_provider = Some(index);
            self.selected_model = None;

            // Populate models based on selected provider
            let provider = &self.providers[index];
            self.models = match provider.id.as_str() {
                "openai" => get_openai_models(),
                "anthropic" => get_anthropic_models(),
                "google" => get_google_models(),
                _ => Vec::new(),
            };

            self.error_message = None;
        }
    }

    pub fn select_model(&mut self, index: usize) {
        if index < self.models.len() {
            self.selected_model = Some(index);
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

            // Google doesn't require prefix check
            if prefix.is_empty() {
                return true;
            }

            // Check if API key starts with the expected prefix
            return self.api_key_input.starts_with(prefix);
        }

        false
    }

    pub fn is_complete(&self) -> bool {
        self.step == OnboardingStep::Complete
            && self.selected_provider.is_some()
            && self.selected_model.is_some()
            && !self.api_key_input.trim().is_empty()
    }

    pub fn to_settings(&self) -> Option<Settings> {
        let provider_idx = self.selected_provider?;
        let model_idx = self.selected_model?;

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
        Self::new()
    }
}

pub mod render;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_onboarding_new() {
        let onboarding = Onboarding::new();
        assert_eq!(onboarding.step, OnboardingStep::Welcome);
        assert!(onboarding.selected_provider.is_none());
        assert!(onboarding.api_key_input.is_empty());
        assert!(onboarding.selected_model.is_none());
        assert_eq!(onboarding.providers.len(), 3);
    }

    #[test]
    fn test_select_provider() {
        let mut onboarding = Onboarding::new();
        onboarding.select_provider(0); // OpenAI

        assert_eq!(onboarding.selected_provider, Some(0));
        assert_eq!(onboarding.models.len(), 3);
        assert_eq!(onboarding.models[0].id, "gpt-4o");
    }

    #[test]
    fn test_validate_key_openai() {
        let mut onboarding = Onboarding::new();
        onboarding.select_provider(0); // OpenAI
        onboarding.api_key_input = "sk-abc123".to_string();

        assert!(onboarding.validate_key());
    }

    #[test]
    fn test_validate_key_openai_wrong_prefix() {
        let mut onboarding = Onboarding::new();
        onboarding.select_provider(0); // OpenAI
        onboarding.api_key_input = "pk-abc123".to_string();

        assert!(!onboarding.validate_key());
    }

    #[test]
    fn test_validate_key_anthropic() {
        let mut onboarding = Onboarding::new();
        onboarding.select_provider(1); // Anthropic
        onboarding.api_key_input = "sk-ant-api123".to_string();

        assert!(onboarding.validate_key());
    }

    #[test]
    fn test_validate_key_google() {
        let mut onboarding = Onboarding::new();
        onboarding.select_provider(2); // Google
        onboarding.api_key_input = "any-key-format".to_string();

        assert!(onboarding.validate_key());
    }

    #[test]
    fn test_to_settings() {
        let mut onboarding = Onboarding::new();
        onboarding.select_provider(0); // OpenAI
        onboarding.select_model(0); // GPT-4o
        onboarding.api_key_input = "sk-test123".to_string();
        onboarding.step = OnboardingStep::Complete;

        let settings = onboarding.to_settings().unwrap();
        assert_eq!(settings.provider_id, "openai");
        assert_eq!(settings.model_id, "gpt-4o");
        assert_eq!(settings.api_key, "sk-test123");
    }
}
