use crate::tui::state::{AppState, Msg, Cmd, TuiMode, OnboardingStep};

pub fn handle_onboarding_msg(state: &mut AppState, msg: Msg) -> Vec<Cmd> {
    if state.onboarding.is_none() {
        return vec![];
    }

    match msg {
        Msg::OnboardingNext => handle_onboarding_next(state),
        Msg::OnboardingBack => handle_onboarding_back(state),
        Msg::OnboardingNavigateUp => handle_onboarding_navigate_up(state),
        Msg::OnboardingNavigateDown => handle_onboarding_navigate_down(state),
        Msg::OnboardingSelectProvider(idx) => handle_onboarding_select_provider(state, idx),
        Msg::OnboardingSelectModel(idx) => handle_onboarding_select_model(state, idx),
        Msg::OnboardingKeyInput(c) => handle_onboarding_key_input(state, c),
        Msg::OnboardingKeyBackspace => handle_onboarding_key_backspace(state),
        Msg::OnboardingSubmit => handle_onboarding_submit(state),
        Msg::OnboardingSkip => handle_onboarding_skip(state),
        _ => vec![],
    }
}

fn handle_onboarding_next(state: &mut AppState) -> Vec<Cmd> {
    if let Some(o) = state.onboarding.as_mut() {
        match &o.step {
            OnboardingStep::ProviderSelect => {
                let idx = o.get_selected_item();
                o.select_provider(idx);
                o.next_step();
            }
            OnboardingStep::ModelSelect => {
                let idx = o.get_selected_item();
                o.select_model(idx);
                o.next_step();
            }
            OnboardingStep::Complete => {
                if let Some(settings) = o.to_settings() {
                    state.onboarding = None;
                    state.mode = TuiMode::Chat;
                    return vec![Cmd::SaveSettings {
                        provider: settings.provider_id,
                        model: settings.model_id,
                        api_key: settings.api_key,
                    }];
                }
            }
            _ => {
                o.next_step();
            }
        }
    }
    vec![]
}

fn handle_onboarding_back(state: &mut AppState) -> Vec<Cmd> {
    if let Some(o) = state.onboarding.as_mut() {
        o.prev_step();
    }
    vec![]
}

fn handle_onboarding_navigate_up(state: &mut AppState) -> Vec<Cmd> {
    if let Some(o) = state.onboarding.as_mut() {
        o.navigate_up();
    }
    vec![]
}

fn handle_onboarding_navigate_down(state: &mut AppState) -> Vec<Cmd> {
    if let Some(o) = state.onboarding.as_mut() {
        o.navigate_down();
    }
    vec![]
}

fn handle_onboarding_select_provider(state: &mut AppState, idx: usize) -> Vec<Cmd> {
    if let Some(o) = state.onboarding.as_mut() {
        o.select_provider(idx);
        o.selected_item = idx;
    }
    vec![]
}

fn handle_onboarding_select_model(state: &mut AppState, idx: usize) -> Vec<Cmd> {
    if let Some(o) = state.onboarding.as_mut() {
        o.select_model(idx);
        o.selected_item = idx;
    }
    vec![]
}

fn handle_onboarding_key_input(state: &mut AppState, c: char) -> Vec<Cmd> {
    if let Some(o) = state.onboarding.as_mut() {
        o.api_key_input.push(c);
    }
    vec![]
}

fn handle_onboarding_key_backspace(state: &mut AppState) -> Vec<Cmd> {
    if let Some(o) = state.onboarding.as_mut() {
        o.api_key_input.pop();
    }
    vec![]
}

fn handle_onboarding_submit(state: &mut AppState) -> Vec<Cmd> {
    if let Some(o) = state.onboarding.clone() {
        if let Some(settings) = o.to_settings() {
            state.onboarding = None;
            state.mode = TuiMode::Chat;
            return vec![Cmd::SaveSettings {
                provider: settings.provider_id,
                model: settings.model_id,
                api_key: settings.api_key,
            }];
        }
    }
    vec![]
}

fn handle_onboarding_skip(state: &mut AppState) -> Vec<Cmd> {
    state.onboarding = None;
    state.mode = TuiMode::Chat;
    vec![]
}
