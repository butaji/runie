//! Navigation handlers: select, session tree, top bar.

use crate::components::model_picker::ModelPicker;
use crate::tui::state::{AppState, Msg};

/// Handle select/model picker messages - delegates to specific handlers.
pub fn handle_select(msg: &Msg, state: &mut AppState) -> Vec<crate::UiCmd> {
    use crate::tui::state::Msg;
    match msg {
        Msg::SelectUp => handle_select_nav(state, true),
        Msg::SelectDown => handle_select_nav(state, false),
        Msg::SelectConfirm => handle_select_confirm(state),
        Msg::SelectToggleDetails => handle_select_toggle_details(state),
        Msg::SwitchModel => handle_switch_model(state),
        _ => vec![],
    }
}

/// Handle session tree messages - delegates to specific handlers.
pub fn handle_session_tree(msg: &Msg, state: &mut AppState) -> Vec<crate::UiCmd> {
    use crate::tui::state::Msg;
    match msg {
        Msg::ToggleSessionTree => handle_toggle_session_tree(state),
        Msg::SessionTreeUp => handle_session_tree_up(state),
        Msg::SessionTreeDown => handle_session_tree_down(state),
        Msg::SessionTreeConfirm => handle_session_tree_confirm(state),
        _ => vec![],
    }
}

/// Handle top bar update messages - delegates to specific handlers.
pub fn handle_top_bar(msg: &Msg, state: &mut AppState) -> Vec<crate::UiCmd> {
    use crate::tui::state::Msg;
    match msg {
        Msg::SetTopBarMockChecks { checks_passed, checks_total, percentage, context_badges } =>
            handle_set_top_bar_mock_checks(state, *checks_passed, *checks_total, *percentage, context_badges.clone()),
        Msg::SetTopBarRealChecks { context_badges } =>
            handle_set_top_bar_real_checks(state, context_badges.clone()),
        Msg::SetInputRightInfo(info) => handle_set_input_right_info(state, info.clone()),
        Msg::UpdateTopBarContext { model, context_window, estimated_tokens } =>
            handle_update_top_bar_context(state, model.clone(), *context_window, *estimated_tokens),
        _ => vec![],
    }
}

/// Handle model/mode state messages - delegates to specific handlers.
pub fn handle_model_mode(msg: &Msg, state: &mut AppState) -> Vec<crate::UiCmd> {
    use crate::tui::state::Msg;
    match msg {
        Msg::SetCurrentModel(model) => handle_set_current_model(state, model.clone()),
        Msg::SetMockMode(mock) => handle_set_mock_mode(state, *mock),
        Msg::ResetAgentState => handle_reset_agent_state(state),
        _ => vec![],
    }
}

// ─── Select/Model Picker ─────────────────────────────────────────────────

fn handle_select_nav(state: &mut AppState, up: bool) -> Vec<crate::UiCmd> {
    if let Some(ref mut picker) = state.model_picker {
        if up { picker.prev(); } else { picker.next(); }
    }
    vec![]
}

fn handle_select_confirm(state: &mut AppState) -> Vec<crate::UiCmd> {
    if let Some(ref mut picker) = state.model_picker {
        if let Some((_provider_id, model_id)) = picker.selected_model() {
            state.current_model = Some(model_id.to_string());
            state.mode = crate::tui::state::TuiMode::Chat;
            state.model_picker = None;
        }
    }
    vec![]
}

fn handle_select_toggle_details(state: &mut AppState) -> Vec<crate::UiCmd> {
    if let Some(ref mut picker) = state.model_picker {
        picker.toggle_details();
    }
    vec![]
}

fn handle_switch_model(state: &mut AppState) -> Vec<crate::UiCmd> {
    let picker = ModelPicker::with_default_models();
    state.model_picker = Some(picker);
    state.mode = crate::tui::state::TuiMode::Overlay;
    vec![]
}

// ─── Session Tree ─────────────────────────────────────────────────────────

fn handle_toggle_session_tree(state: &mut AppState) -> Vec<crate::UiCmd> {
    super::slash::handle_tree(state);
    vec![]
}

fn handle_session_tree_up(state: &mut AppState) -> Vec<crate::UiCmd> {
    state.session_tree.move_up();
    vec![]
}

fn handle_session_tree_down(state: &mut AppState) -> Vec<crate::UiCmd> {
    state.session_tree.move_down();
    vec![]
}

fn handle_session_tree_confirm(state: &mut AppState) -> Vec<crate::UiCmd> {
    super::tree::handle_tree_confirm(state);
    vec![]
}

// ─── Top Bar ─────────────────────────────────────────────────────────────

pub fn handle_set_git_info(state: &mut AppState, repo: String, branch: String, path: String) -> Vec<crate::UiCmd> {
    state.top_bar.repo = repo;
    state.top_bar.branch = branch;
    state.top_bar.path = path;
    vec![]
}

fn handle_set_top_bar_mock_checks(
    state: &mut AppState,
    checks_passed: Option<usize>,
    checks_total: Option<usize>,
    percentage: Option<f32>,
    context_badges: Vec<String>,
) -> Vec<crate::UiCmd> {
    state.top_bar.checks_passed = checks_passed;
    state.top_bar.checks_total = checks_total;
    state.top_bar.percentage = percentage;
    state.top_bar.context_badges = context_badges;
    state.top_bar.context_pct = None;
    state.top_bar.context_bar_pct = None;
    vec![]
}

fn handle_set_top_bar_real_checks(state: &mut AppState, context_badges: Vec<String>) -> Vec<crate::UiCmd> {
    state.top_bar.checks_passed = None;
    state.top_bar.checks_total = None;
    state.top_bar.percentage = None;
    state.top_bar.context_badges = context_badges;
    state.top_bar.context_pct = None;
    state.top_bar.context_bar_pct = None;
    vec![]
}

fn handle_set_input_right_info(state: &mut AppState, info: String) -> Vec<crate::UiCmd> {
    state.input_right_info = info;
    vec![]
}

fn handle_update_top_bar_context(
    state: &mut AppState,
    model: String,
    context_window: Option<usize>,
    estimated_tokens: Option<usize>,
) -> Vec<crate::UiCmd> {
    state.top_bar.model = model;
    state.top_bar.context_window = context_window;
    state.top_bar.estimated_tokens = estimated_tokens;
    vec![]
}

// ─── Model/Mode State ──────────────────────────────────────────────────────

pub fn handle_enter_onboarding(state: &mut AppState) -> Vec<crate::UiCmd> {
    state.mode = crate::tui::state::TuiMode::Onboarding;
    state.onboarding = Some(crate::components::onboarding::Onboarding::new(state.mock_mode));
    vec![]
}

fn handle_set_current_model(state: &mut AppState, model: Option<String>) -> Vec<crate::UiCmd> {
    state.current_model = model.clone();
    // Extract just the model name (after "/") for top_bar.model
    if let Some(ref m) = model {
        if let Some(slash_idx) = m.rfind('/') {
            state.top_bar.model = m[slash_idx + 1..].to_string();
        } else {
            state.top_bar.model = m.clone();
        }
    } else {
        state.top_bar.model = String::new();
    }
    vec![]
}

fn handle_set_mock_mode(state: &mut AppState, mock: bool) -> Vec<crate::UiCmd> {
    state.mock_mode = mock;
    vec![]
}

fn handle_reset_agent_state(state: &mut AppState) -> Vec<crate::UiCmd> {
    state.agent_running = false;
    state.agent_start_time = None;
    state.session_token_usage = runie_ai::TokenUsage::default();
    vec![]
}
