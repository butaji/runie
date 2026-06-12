//! Login flow event handling.

use crate::model::AppState;
use crate::login_flow::LoginStep;
use crate::Event;

pub(crate) fn update(state: &mut AppState, event: Event) {
    match event {
        Event::LoginFlowStart => state.login_flow_start(),
        Event::LoginFlowSelectProvider { provider } => {
            state.login_flow_select_provider(provider)
        }
        Event::LoginFlowSubmitKey { provider, key } => {
            state.login_flow_submit_key(provider, key)
        }
        Event::LoginFlowValidationDone { models, .. } => {
            state.login_flow_validation_done(models)
        }
        Event::LoginFlowValidationFailed { error, .. } => {
            state.login_flow_validation_failed(error)
        }
        Event::LoginFlowModelsFetched { models, .. } => {
            state.login_flow_models_fetched(models)
        }
        Event::LoginFlowToggleModel { model } => state.login_flow_toggle_model(model),
        Event::LoginFlowSave => state.login_flow_save(),
        Event::LoginFlowCancel => state.login_flow_cancel(),
        _ => {}
    }
}

impl AppState {
    fn login_flow_start(&mut self) {
        self.login_flow = Some(crate::login_flow::LoginFlowState::new());
        self.rebuild_login_dialog();
    }

    fn login_flow_select_provider(&mut self, provider: String) {
        if let Some(ref mut flow) = self.login_flow {
            *flow = flow.clone().with_provider(provider);
            self.rebuild_login_dialog();
        }
    }

    fn login_flow_submit_key(&mut self, provider: String, key: String) {
        if let Some(flow) = self.login_flow.clone() {
            let p = if provider.is_empty() { flow.provider.clone() } else { provider };
            let defaults = self.provider_defaults(&p);
            let mut new_flow = flow.with_key_and_defaults(key, defaults);
            new_flow.provider = p;
            self.login_flow = Some(new_flow);
            self.rebuild_login_dialog();
        }
    }

    pub(crate) fn provider_defaults(&self, provider: &str,
    ) -> Vec<String> {
        crate::provider_registry::find_provider(provider)
            .map(|meta| meta.default_models.iter().map(|s| s.to_string()).collect())
            .unwrap_or_default()
    }

    fn login_flow_validation_done(&mut self, models: Vec<String>) {
        if let Some(ref mut flow) = self.login_flow {
            // Non-blocking: enrich the model list without changing the step.
            *flow = flow.clone().with_fetched_models(models);
            self.rebuild_login_dialog();
        }
    }

    fn login_flow_models_fetched(&mut self, models: Vec<String>) {
        if let Some(ref mut flow) = self.login_flow {
            if flow.step == LoginStep::ModelSelect {
                *flow = flow.clone().with_fetched_models(models);
                self.rebuild_login_dialog();
            }
        }
    }

    fn login_flow_validation_failed(&mut self, error: String) {
        // Non-blocking: surface a transient warning, do NOT change the step.
        if let Some(ref flow) = self.login_flow {
            if flow.step == LoginStep::ModelSelect {
                self.set_transient(
                    format!("Could not verify key: {}", error),
                    crate::event::TransientLevel::Warning,
                );
                self.mark_dirty();
            }
        }
    }

    fn login_flow_toggle_model(&mut self, model: String) {
        if let Some(ref mut flow) = self.login_flow {
            flow.toggle_model(&model);
            self.rebuild_login_dialog();
        }
    }

    fn login_flow_save(&mut self) {
        if let Some(ref flow) = self.login_flow {
            let base_url = crate::provider_registry::find_provider(&flow.provider)
                .map(|p| p.base_url.to_string())
                .unwrap_or_default();
            let models: Vec<String> = flow.selected_models.iter().cloned().collect();
            let _ = crate::login_config::save_provider_config(
                &flow.provider,
                &base_url,
                &flow.key,
                &models,
            );
        }
        self.open_dialog = None;
        self.login_flow = None;
        self.mark_dirty();
    }

    fn login_flow_cancel(&mut self) {
        self.open_dialog = None;
        self.login_flow = None;
        self.mark_dirty();
    }

    fn rebuild_login_dialog(&mut self) {
        use crate::login_flow::build_login_stack;
        if let Some(ref flow) = self.login_flow {
            let stack = build_login_stack(flow);
            self.open_dialog = Some(crate::commands::DialogState::PanelStack(stack));
            self.mark_dirty();
        }
    }
}
