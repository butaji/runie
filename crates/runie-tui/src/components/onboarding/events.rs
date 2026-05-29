// ============================================================================
// Event Handling & Navigation
// ============================================================================

use super::{Onboarding, OnboardingStep};

impl Onboarding {
    pub fn navigate_up(&mut self) {
        let max = match &self.step {
            OnboardingStep::Welcome => 0,
            OnboardingStep::ProviderSelect => self.get_filtered_provider_count().saturating_sub(1),
            OnboardingStep::ModelSelect => self.get_filtered_model_count().saturating_sub(1),
            OnboardingStep::KeyInput => 0,
            OnboardingStep::Complete => 1,
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
            OnboardingStep::Complete => 1,
        };
        if self.selected_item < max {
            self.selected_item += 1;
        }
    }

    pub fn get_filtered_provider_count(&self) -> usize {
        if self.filtered_provider_indices.is_empty() && self.search_query.is_empty() {
            self.providers.len()
        } else {
            self.filtered_provider_indices.len()
        }
    }

    pub fn get_filtered_model_count(&self) -> usize {
        if self.filtered_model_indices.is_empty() && self.search_query.is_empty() {
            self.models.len()
        } else {
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
}
