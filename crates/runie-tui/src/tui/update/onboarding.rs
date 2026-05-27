use crate::tui::state::{AppState, Msg, Cmd, TuiMode, OnboardingStep};
use crate::components::onboarding::ModelOption;

pub fn handle_onboarding_msg(state: &mut AppState, msg: Msg) -> Vec<Cmd> {
    // Only log non-tick messages to avoid log spam
    if !matches!(msg, Msg::Tick | Msg::CursorBlink) {
        tracing::debug!("[Onboarding] Received message: {:?}", msg);
    }
    if state.onboarding.is_none() {
        tracing::debug!("[Onboarding] No onboarding active, ignoring message");
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
        Msg::OnboardingSearchInput(c) => handle_search_input(state, c),
        Msg::OnboardingSearchBackspace => handle_search_backspace(state),
        Msg::OnboardingSubmit => handle_onboarding_submit(state),
        Msg::OnboardingSkip => handle_onboarding_skip(state),
        Msg::ModelsFetched(models) => handle_models_fetched(state, models),
        Msg::ModelsFetchFailed(error) => handle_models_fetch_failed(state, error),
        Msg::Paste(text) => handle_paste(state, text),
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
        // P0-2: Welcome has no previous step, so Esc on Welcome stays put.
        // P2-3 FIX: The Welcome render now shows "Press Enter to begin →" CTA.
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
    tracing::info!("[Onboarding] handle_models_fetched called with {} models", models.len());
    if let Some(o) = state.onboarding.as_mut() {
        tracing::debug!("[Onboarding] Setting is_fetching_models = false, step = ModelSelect");
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
        // P1-1 FIX: Use dedicated fetch_error field, fall back to hardcoded models
        o.set_fetch_error(error.clone());
        o.error_message = Some(format!("Failed to fetch models: {}", error));
        if let Some(provider_idx) = o.selected_provider {
            let provider = &o.providers[provider_idx];
            o.models = get_models_for_provider(&provider.id);
            o.filtered_model_indices = (0..o.models.len()).collect();
            o.selected_item = 0;
            // P1-1 FIX: Stay on KeyInput step when fetch fails so user can retry
            // o.step = OnboardingStep::ModelSelect;
            o.enter_step();
        }
    }
    vec![]
}

fn handle_onboarding_key_input(state: &mut AppState) -> Vec<Cmd> {
    tracing::debug!("[Onboarding] handle_onboarding_key_input called");
    if let Some(o) = state.onboarding.as_mut() {
        let is_valid = o.validate_key();
        tracing::debug!("[Onboarding] Key validation result: {}", is_valid);
        if is_valid {
            if let Some(provider_idx) = o.selected_provider {
                // Clone IDs before mutating state to avoid borrow conflict
                let provider_id = o.providers[provider_idx].id.clone();
                let api_key = o.api_key_input.clone();
                o.is_fetching_models = true;
                // P1-1 FIX: Clear previous fetch error on retry
                o.clear_fetch_error();
                return vec![Cmd::FetchModels {
                    provider_id,
                    api_key,
                }];
            }
        } else {
            tracing::warn!("[Onboarding] Key validation failed");
            o.error_message = Some("Invalid API key format".to_string());
        }
    }
    vec![]
}

fn handle_paste(state: &mut AppState, text: String) -> Vec<Cmd> {
    if let Some(o) = state.onboarding.as_mut() {
        match o.step {
            OnboardingStep::KeyInput => {
                o.api_key_input.push_str(&text);
            }
            OnboardingStep::ProviderSelect | OnboardingStep::ModelSelect => {
                for c in text.chars() {
                    o.search_query.push(c);
                }
                let query = o.search_query.clone();
                o.update_search(&query);
            }
            _ => {}
        }
    }
    vec![]
}

fn handle_search_input(state: &mut AppState, c: char) -> Vec<Cmd> {
    if let Some(o) = state.onboarding.as_mut() {
        o.search_query.push(c);
        let query = o.search_query.clone();
        o.update_search(&query);
    }
    vec![]
}

fn handle_search_backspace(state: &mut AppState) -> Vec<Cmd> {
    if let Some(o) = state.onboarding.as_mut() {
        o.search_query.pop();
        let query = o.search_query.clone();
        o.update_search(&query);
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
