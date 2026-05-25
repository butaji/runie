use crate::tui::state::{AppState, Msg, Cmd, TuiMode, OnboardingStep};
use crate::components::onboarding::ModelOption;

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
        Msg::OnboardingKeyInput(c) => handle_keypress(state, c),
        Msg::OnboardingKeyBackspace => handle_onboarding_key_backspace(state),
        Msg::OnboardingSearchInput(c) => {
            if let Some(o) = state.onboarding.as_mut() {
                o.search_query.push(c);
                let query = o.search_query.clone();
                o.update_search(&query);
            }
            vec![]
        }
        Msg::OnboardingSearchBackspace => {
            if let Some(o) = state.onboarding.as_mut() {
                o.search_query.pop();
                let query = o.search_query.clone();
                o.update_search(&query);
            }
            vec![]
        }
        Msg::OnboardingSubmit => handle_onboarding_submit(state),
        Msg::OnboardingSkip => handle_onboarding_skip(state),
        Msg::ModelsFetched(models) => handle_models_fetched(state, models),
        Msg::ModelsFetchFailed(error) => handle_models_fetch_failed(state, error),
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
                if o.selected_item == 0 {
                    // Yes - add another provider
                    o.step = OnboardingStep::ProviderSelect;
                    o.selected_provider = None;
                    o.selected_model = None;
                    o.api_key_input.clear();
                    o.search_query.clear();
                    o.filtered_provider_indices.clear();
                    o.filtered_model_indices.clear();
                    o.error_message = None;
                    o.enter_step();
                } else {
                    // No - finish
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
            }
            OnboardingStep::KeyInput => {
                return handle_onboarding_key_input(state);
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
        if let Some(provider_idx) = o.filtered_provider_indices.get(idx).copied() {
            o.select_provider(provider_idx);
            o.selected_item = idx;
        }
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

fn handle_keypress(state: &mut AppState, c: char) -> Vec<Cmd> {
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

fn handle_models_fetched(state: &mut AppState, models: Vec<crate::tui::state::ModelInfo>) -> Vec<Cmd> {
    if let Some(o) = state.onboarding.as_mut() {
        o.is_fetching_models = false;
        o.models = models.into_iter().map(|m| ModelOption {
            name: m.name,
            id: m.id,
            description: String::new(),
        }).collect();
        o.models.sort_by(|a, b| a.name.cmp(&b.name));
        o.filtered_model_indices = (0..o.models.len()).collect();
        o.selected_item = 0;
        o.step = OnboardingStep::ModelSelect;
        o.enter_step();
    }
    vec![]
}

fn handle_models_fetch_failed(state: &mut AppState, error: String) -> Vec<Cmd> {
    if let Some(o) = state.onboarding.as_mut() {
        o.is_fetching_models = false;
        o.error_message = Some(format!("Failed to fetch models: {}", error));
        if let Some(provider_idx) = o.selected_provider {
            let provider = &o.providers[provider_idx];
            o.models = get_models_for_provider(&provider.id);
            o.filtered_model_indices = (0..o.models.len()).collect();
            o.selected_item = 0;
            o.step = OnboardingStep::ModelSelect;
            o.enter_step();
        }
    }
    vec![]
}

fn handle_onboarding_key_input(state: &mut AppState) -> Vec<Cmd> {
    if let Some(o) = state.onboarding.as_mut() {
        if o.validate_key() {
            if let Some(provider_idx) = o.selected_provider {
                let provider = &o.providers[provider_idx];
                o.is_fetching_models = true;
                return vec![Cmd::FetchModels {
                    provider_id: provider.id.clone(),
                    api_key: o.api_key_input.clone(),
                }];
            }
        } else {
            o.error_message = Some("Invalid API key format".to_string());
        }
    }
    vec![]
}

fn get_models_for_provider(provider_id: &str) -> Vec<ModelOption> {
    use crate::components::onboarding::get_openai_models;
    use crate::components::onboarding::get_anthropic_models;
    use crate::components::onboarding::get_google_models;
    use crate::components::onboarding::get_cohere_models;
    use crate::components::onboarding::get_mistral_models;
    use crate::components::onboarding::get_deepseek_models;
    use crate::components::onboarding::get_groq_models;
    use crate::components::onboarding::get_openrouter_models;
    use crate::components::onboarding::get_huggingface_models;
    use crate::components::onboarding::get_xai_models;
    use crate::components::onboarding::get_azure_models;
    use crate::components::onboarding::get_moonshot_models;
    use crate::components::onboarding::get_perplexity_models;
    use crate::components::onboarding::get_ollama_models;
    use crate::components::onboarding::get_hyperbolic_models;
    use crate::components::onboarding::get_together_models;
    use crate::components::onboarding::get_zai_models;
    use crate::components::onboarding::get_minimax_models;
    use crate::components::onboarding::get_mira_models;
    use crate::components::onboarding::get_galadriel_models;
    use crate::components::onboarding::get_llamafile_models;

    match provider_id {
        "openai" => get_openai_models(),
        "anthropic" => get_anthropic_models(),
        "google" => get_google_models(),
        "cohere" => get_cohere_models(),
        "mistral" => get_mistral_models(),
        "deepseek" => get_deepseek_models(),
        "groq" => get_groq_models(),
        "openrouter" => get_openrouter_models(),
        "huggingface" => get_huggingface_models(),
        "xai" => get_xai_models(),
        "azure" => get_azure_models(),
        "moonshot" => get_moonshot_models(),
        "perplexity" => get_perplexity_models(),
        "ollama" => get_ollama_models(),
        "hyperbolic" => get_hyperbolic_models(),
        "together" => get_together_models(),
        "zai" => get_zai_models(),
        "minimax" => get_minimax_models(),
        "mira" => get_mira_models(),
        "galadriel" => get_galadriel_models(),
        "llamafile" => get_llamafile_models(),
        _ => vec![],
    }
}
