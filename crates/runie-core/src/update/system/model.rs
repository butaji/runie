//! Model/provider switching helpers.

use crate::actors::ConfigMsg;
use crate::event::TransientLevel;
use crate::model::{AppState, ModelSource};

impl AppState {
    /// Switch the active provider/model and optionally persist it to config.
    pub(crate) fn switch_model(&mut self, provider: String, model: String, explicit: bool) {
        if explicit {
            self.config_mut().model_source = ModelSource::UserOverride;
        }
        if self.config().current_provider == provider && self.config().current_model == model {
            return;
        }
        let model_source = self.config().model_source;
        self.set_active_model(provider.clone(), model.clone(), model_source);
        self.persist_current_model();
        self.notify(
            format!("Switched to {}/{}", provider, model),
            TransientLevel::Success,
        );
    }

    /// Update the active model fields, token tracker, and usage history.
    pub fn set_active_model(&mut self, provider: String, model: String, source: ModelSource) {
        self.config_mut().current_provider = provider.clone();
        self.config_mut().current_model = model.clone();
        self.config_mut().model_source = source;
        self.configure_token_tracker();
        self.record_model_usage(&provider, &model);
        if self.config().telemetry_enabled() {
            tracing::info!(provider = %provider, model = %model, "model_switch");
        }
    }

    fn persist_current_model(&self) {
        let provider = self.config().current_provider.clone();
        let model = self.config().current_model.clone();
        let handles = self.actor_handles().cloned();
        if let Some(handles) = handles {
            if tokio::runtime::Handle::try_current().is_ok() {
                let _ = handles.config.try_send(ConfigMsg::SetDefaultModel { provider, model });
            }
        }
    }

    pub(crate) fn set_provider(&mut self, provider: &str) {
        if self.config().current_provider == provider {
            return;
        }
        let provider = provider.to_owned();
        let model = self
            .config()
            .model_providers()
            .get(&provider)
            .and_then(|p| p.models.first().cloned())
            .unwrap_or_else(|| self.config().current_model.clone());
        self.switch_model(provider, model, true);
    }

    pub(crate) fn set_model(&mut self, model: &str) {
        if self.config().current_model == model {
            return;
        }
        let model = model.to_owned();
        let provider = self.config().current_provider.clone();
        self.switch_model(provider, model, true);
    }

    pub(crate) fn cycle_model(&mut self, delta: isize) {
        let enabled: Vec<usize> = self
            .config
            .scoped_models
            .iter()
            .enumerate()
            .filter(|(_, m)| m.enabled)
            .map(|(i, _)| i)
            .collect();
        if enabled.is_empty() {
            return;
        }
        let current_pos = enabled
            .iter()
            .position(|&i| i == self.config().scoped_index)
            .unwrap_or(0);
        let len = enabled.len() as isize;
        let new_pos = ((current_pos as isize + delta).rem_euclid(len)) as usize;
        self.config_mut().scoped_index = enabled[new_pos];
        let scoped_index = self.config().scoped_index;
        let model = &self.config().scoped_models[scoped_index];
        self.switch_model(model.provider.clone(), model.name.clone(), true);
    }

    pub(crate) fn cycle_thinking_level(&mut self) {
        let new_level = self.config().thinking_level.cycle();
        self.set_thinking_level(new_level);
        self.notify(
            format!("Thinking level: {}", self.config().thinking_level.as_str()),
            TransientLevel::Info,
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn switch_model_updates_source() {
        let mut state = AppState::default();
        state.config_mut().current_provider = "openai".into();
        state.config_mut().current_model = "gpt-4o".into();
        state.switch_model("anthropic".into(), "claude-3".into(), true);
        assert_eq!(state.config_mut().current_provider, "anthropic");
        assert_eq!(state.config_mut().current_model, "claude-3");
        assert_eq!(state.config_mut().model_source, ModelSource::UserOverride);
    }
}
