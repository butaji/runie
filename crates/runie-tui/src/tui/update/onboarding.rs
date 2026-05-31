use crate::tui::state::{AppState, Msg, Cmd, TuiMode, OnboardingStep};
use crate::components::onboarding::ModelOption;

fn is_onboarding_nav_msg(msg: &Msg) -> bool {
    matches!(msg, Msg::OnboardingNext | Msg::OnboardingBack | Msg::OnboardingNavigateUp | Msg::OnboardingNavigateDown)
}

fn is_onboarding_selection_msg(msg: &Msg) -> bool {
    matches!(msg, Msg::OnboardingSelectProvider(_) | Msg::OnboardingSelectModel(_))
}

fn is_onboarding_key_msg(msg: &Msg) -> bool {
    matches!(msg, Msg::OnboardingKeyInput(_) | Msg::OnboardingKeyBackspace)
}

fn is_onboarding_search_msg(msg: &Msg) -> bool {
    matches!(msg, Msg::OnboardingSearchInput(_) | Msg::OnboardingSearchBackspace)
}

fn is_onboarding_submit_skip_msg(msg: &Msg) -> bool {
    matches!(msg, Msg::OnboardingSubmit | Msg::OnboardingSkip)
}

fn is_onboarding_model_fetch_msg(msg: &Msg) -> bool {
    matches!(msg, Msg::ModelsFetched(_) | Msg::ModelsFetchFailed(_))
}

fn log_onboarding_msg(msg: &Msg) {
    if !matches!(msg, Msg::Tick | Msg::CursorBlink) {
        tracing::debug!("[Onboarding] Received message: {:?}", msg);
    }
}

fn check_onboarding_active(state: &AppState) -> bool {
    state.onboarding.is_some()
}

fn route_onboarding_msg(state: &mut AppState, msg: Msg) -> Vec<Cmd> {
    if is_onboarding_nav_msg(&msg) {
        return handle_nav_msg(state, msg);
    }
    if is_onboarding_selection_msg(&msg) {
        return handle_selection_msg(state, msg);
    }
    if is_onboarding_key_msg(&msg) {
        return handle_key_msg(state, msg);
    }
    if is_onboarding_search_msg(&msg) {
        return handle_search_msg(state, msg);
    }
    if is_onboarding_submit_skip_msg(&msg) {
        return handle_submit_skip_msg(state, msg);
    }
    if is_onboarding_model_fetch_msg(&msg) {
        return handle_model_fetch_msg(state, msg);
    }
    if let Msg::Paste(text) = msg { return handle_onboarding_action(state, text); }
    vec![]
}

pub fn handle_onboarding_msg(state: &mut AppState, msg: Msg) -> Vec<Cmd> {
    log_onboarding_msg(&msg);
    if !check_onboarding_active(state) {
        tracing::debug!("[Onboarding] No onboarding active, ignoring message");
        return vec![];
    }
    route_onboarding_msg(state, msg)
}

fn handle_nav_msg(state: &mut AppState, msg: Msg) -> Vec<Cmd> {
    match msg {
        Msg::OnboardingNext => handle_onboarding_next(state),
        Msg::OnboardingBack => handle_onboarding_back(state),
        Msg::OnboardingNavigateUp => handle_onboarding_navigate(state, true),
        Msg::OnboardingNavigateDown => handle_onboarding_navigate(state, false),
        _ => vec![],
    }
}

fn handle_selection_msg(state: &mut AppState, msg: Msg) -> Vec<Cmd> {
    match msg {
        Msg::OnboardingSelectProvider(idx) => handle_onboarding_select_provider(state, idx),
        Msg::OnboardingSelectModel(idx) => handle_onboarding_select_model(state, idx),
        _ => vec![],
    }
}

fn handle_key_msg(state: &mut AppState, msg: Msg) -> Vec<Cmd> {
    match msg {
        Msg::OnboardingKeyInput(c) => handle_onboarding_key(state, c),
        Msg::OnboardingKeyBackspace => handle_onboarding_key_backspace(state),
        _ => vec![],
    }
}

fn handle_search_msg(state: &mut AppState, msg: Msg) -> Vec<Cmd> {
    match msg {
        Msg::OnboardingSearchInput(c) => handle_search_input(state, c),
        Msg::OnboardingSearchBackspace => handle_search_backspace(state),
        _ => vec![],
    }
}

fn handle_submit_skip_msg(state: &mut AppState, msg: Msg) -> Vec<Cmd> {
    match msg {
        Msg::OnboardingSubmit => handle_onboarding_submit(state),
        Msg::OnboardingSkip => handle_onboarding_skip(state),
        _ => vec![],
    }
}

fn handle_model_fetch_msg(state: &mut AppState, msg: Msg) -> Vec<Cmd> {
    match msg {
        Msg::ModelsFetched(models) => handle_models_fetched(state, models),
        Msg::ModelsFetchFailed(error) => handle_models_fetch_failed(state, error),
        _ => vec![],
    }
}

/// Handle navigation (up/down) in one function
fn handle_onboarding_navigate(state: &mut AppState, up: bool) -> Vec<Cmd> {
    if let Some(o) = state.onboarding.as_mut() {
        if up {
            o.navigate_up();
        } else {
            o.navigate_down();
        }
    }
    vec![]
}

/// Handles key input during API key entry phase.
fn handle_onboarding_key(state: &mut AppState, c: char) -> Vec<Cmd> {
    if let Some(o) = state.onboarding.as_mut() {
        o.api_key_input.push(c);
    }
    vec![]
}

/// Handles paste action, routing to appropriate step handler.
fn handle_onboarding_action(state: &mut AppState, text: String) -> Vec<Cmd> {
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

fn handle_onboarding_next(state: &mut AppState) -> Vec<Cmd> {
    let step = state.onboarding.as_ref().map(|o| o.step.clone());
    match step {
        Some(OnboardingStep::ProviderSelect) => {
            if let Some(o) = state.onboarding.as_mut() {
                handle_provider_next(o);
            }
        }
        Some(OnboardingStep::ModelSelect) => {
            if let Some(o) = state.onboarding.as_mut() {
                handle_model_next(o);
            }
        }
        Some(OnboardingStep::Complete) => {
            return handle_onboarding_complete_next(state);
        }
        Some(OnboardingStep::KeyInput) => {
            return handle_onboarding_key_input(state);
        }
        _ => {
            if let Some(o) = state.onboarding.as_mut() {
                o.next_step();
            }
        }
    }
    vec![]
}

fn handle_provider_next(o: &mut crate::components::onboarding::Onboarding) {
    if o.filtered_provider_indices.is_empty() {
        o.error_message = Some("No providers match your search".to_string());
        return;
    }
    let idx = o.get_selected_item();
    o.select_provider(idx);
    o.next_step();
}

fn handle_model_next(o: &mut crate::components::onboarding::Onboarding) {
    let idx = o.get_selected_item();
    if o.selected_models.is_empty() && o.selected_model.is_none() {
        o.select_model(idx);
    }
    if o.selected_models.is_empty() && o.selected_model.is_none() {
        o.error_message = Some("Please select at least one model".to_string());
        return;
    }
    o.next_step();
}

fn handle_onboarding_complete_next(state: &mut AppState) -> Vec<Cmd> {
    if let Some(o) = state.onboarding.as_mut() {
        if o.selected_item == 0 {
            // Yes - add another provider
            o.step = OnboardingStep::ProviderSelect;
            o.selected_provider = None;
            o.selected_model = None;
            o.selected_models.clear();
            o.api_key_input.clear();
            o.search_query.clear();
            o.filtered_provider_indices.clear();
            o.filtered_model_indices.clear();
            o.error_message = None;
            o.enter_step();
            return vec![];
        }
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

fn handle_onboarding_key_backspace(state: &mut AppState) -> Vec<Cmd> {
    if let Some(o) = state.onboarding.as_mut() {
        // Remove the last token (word) instead of just one character
        // A token is separated by whitespace, dashes, underscores, dots, or slashes
        let s = &o.api_key_input;
        if s.is_empty() {
            return vec![];
        }

        // Find the last non-separator character
        let mut end = s.len();
        let bytes = s.as_bytes();

        // Skip trailing separators
        while end > 0 && is_token_separator(bytes[end - 1]) {
            end -= 1;
        }

        // Now skip the token characters
        while end > 0 && !is_token_separator(bytes[end - 1]) {
            end -= 1;
        }

        o.api_key_input.truncate(end);
    }
    vec![]
}

fn is_token_separator(b: u8) -> bool {
    matches!(b, b' ' | b'\t' | b'\n' | b'-' | b'_' | b'.' | b'/' | b':' | b'=')
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

type ModelProviderFn = fn() -> Vec<ModelOption>;

fn get_models_for_provider(provider_id: &str) -> Vec<ModelOption> {
    use crate::components::onboarding::*;

    static MODEL_LOOKUP: &[(&str, ModelProviderFn)] = &[
        ("openai", get_openai_models),
        ("anthropic", get_anthropic_models),
        ("google", get_google_models),
        ("cohere", get_cohere_models),
        ("mistral", get_mistral_models),
        ("deepseek", get_deepseek_models),
        ("groq", get_groq_models),
        ("openrouter", get_openrouter_models),
        ("huggingface", get_huggingface_models),
        ("xai", get_xai_models),
        ("azure", get_azure_models),
        ("moonshot", get_moonshot_models),
        ("perplexity", get_perplexity_models),
        ("ollama", get_ollama_models),
        ("hyperbolic", get_hyperbolic_models),
        ("together", get_together_models),
        ("zai", get_zai_models),
        ("minimax", get_minimax_models),
        ("mira", get_mira_models),
        ("galadriel", get_galadriel_models),
        ("llamafile", get_llamafile_models),
    ];

    MODEL_LOOKUP
        .iter()
        .find(|(id, _)| *id == provider_id)
        .map(|(_, f)| f())
        .unwrap_or_default()
}
