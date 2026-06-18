//! Model/provider switching helpers.

use crate::event::TransientLevel;
use crate::model::AppState;

impl AppState {
    pub(crate) fn switch_model(&mut self, provider: String, model: String) {
        if self.config.current_provider == provider && self.config.current_model == model {
            return;
        }
        self.config.current_provider = provider.clone();
        self.config.current_model = model.clone();
        self.config.config_provider = provider.clone();
        self.config.config_model = model.clone();
        self.configure_token_tracker();
        self.record_model_usage(&provider, &model);
        self.config.telemetry.track_event("model_switch", {
            let mut m = std::collections::HashMap::new();
            m.insert("provider".into(), provider.clone());
            m.insert("model".into(), model.clone());
            m
        });
        self.persist_current_model();
        self.notify(
            format!("Switched to {}/{}", provider, model),
            TransientLevel::Success,
        );
    }

    fn persist_current_model(&self) {
        #[cfg(not(test))]
        {
            let provider = self.config.current_provider.clone();
            let model = self.config.current_model.clone();
            let mut config = crate::config::Config::load(None);
            config.provider = Some(provider);
            config.model = None;
            config.models.default = Some(model);
            let _ = config.save();
        }
    }

    pub(crate) fn set_provider(&mut self, provider: &str) {
        if self.config.current_provider == provider {
            return;
        }
        let provider = provider.to_string();
        let model = first_model_for_provider(&provider)
            .unwrap_or_else(|| self.config.current_model.clone());
        self.switch_model(provider, model);
    }

    pub(crate) fn set_model(&mut self, model: &str) {
        if self.config.current_model == model {
            return;
        }
        let model = model.to_string();
        self.switch_model(self.config.current_provider.clone(), model);
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
            .position(|&i| i == self.config.scoped_index)
            .unwrap_or(0);
        let len = enabled.len() as isize;
        let new_pos = ((current_pos as isize + delta).rem_euclid(len)) as usize;
        self.config.scoped_index = enabled[new_pos];
        let model = &self.config.scoped_models[self.config.scoped_index];
        self.switch_model(model.provider.clone(), model.name.clone());
    }

    pub(crate) fn cycle_thinking_level(&mut self) {
        self.config.thinking_level = self.config.thinking_level.cycle();
        self.notify(
            format!("Thinking level: {}", self.config.thinking_level.as_str()),
            TransientLevel::Info,
        );
    }

    pub(crate) fn set_thinking_level(&mut self, level: crate::model::ThinkingLevel) {
        self.config.thinking_level = level;
        self.notify(
            format!(
                "Thinking level set to: {}",
                self.config.thinking_level.as_str()
            ),
            TransientLevel::Info,
        );
    }
}

fn first_model_for_provider(provider: &str) -> Option<String> {
    let configured = crate::login_config::list_configured_providers();
    configured
        .iter()
        .find(|(p, _, _)| p == provider)
        .and_then(|(_, _, models)| models.first().cloned())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn switch_model_updates_config_defaults() {
        let mut state = AppState::default();
        state.config.current_provider = "openai".into();
        state.config.current_model = "gpt-4o".into();
        state.switch_model("anthropic".into(), "claude-3".into());
        assert_eq!(state.config.current_provider, "anthropic");
        assert_eq!(state.config.current_model, "claude-3");
        assert_eq!(state.config.config_provider, "anthropic");
        assert_eq!(state.config.config_model, "claude-3");
    }
}
