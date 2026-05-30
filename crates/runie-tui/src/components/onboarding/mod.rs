// ============================================================================
// Onboarding Types & Public API
// ============================================================================

pub mod builder;
pub use builder::*;

pub mod events;
pub mod models;
pub mod providers;
pub mod state;

mod matrix_bg;
pub use matrix_bg::{render_onboarding_screen, MatrixRain};

pub mod render;

// Re-export from submodules for backwards compatibility
#[allow(unused_imports)]
pub use events::*;
#[allow(unused_imports)]
pub use models::*;
#[allow(unused_imports)]
pub use providers::*;
#[allow(unused_imports)]
pub use state::*;

// ─── OnboardingStep ─────────────────────────────────────────────────────────

#[derive(Debug, Clone, PartialEq)]
pub enum OnboardingStep {
    Welcome,
    ProviderSelect,
    KeyInput,
    ModelSelect,
    Complete,
}

// ─── Provider & Model Options ────────────────────────────────────────────────

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

// ─── Onboarding ─────────────────────────────────────────────────────────────

#[derive(Debug, Clone)]
pub struct Onboarding {
    pub step: OnboardingStep,
    pub selected_item: usize,
    pub selected_provider: Option<usize>,
    pub api_key_input: String,
    pub selected_model: Option<usize>,
    pub selected_models: Vec<usize>,
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

// ─── Settings ────────────────────────────────────────────────────────────────

#[derive(Debug, Clone)]
pub struct Settings {
    pub provider_id: String,
    pub provider_name: String,
    pub model_id: String,
    pub model_name: String,
    pub api_key: String,
}

// ─── Fuzzy Match ─────────────────────────────────────────────────────────────

pub fn fuzzy_match(query: &str, target: &str) -> bool {
    if query.is_empty() {
        return true;
    }
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

#[cfg(test)]
mod comprehensive_tests;
