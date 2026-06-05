//! ViewModel builders - functions to construct ViewModels from AppState.

use crate::components::MessageItem;
use crate::components::message_list::Feed;
use crate::components::message_list::PlanStatus;
use crate::components::command_palette::CommandPalette;

use super::{
    InputBarViewModel, StatusBarViewModel, AgentListViewModel, CommandPaletteViewModel,
    PermissionModalViewModel, OverlayViewModel, SessionTreeViewModel, DiffViewerViewModel,
    OnboardingViewModel, OnboardingStep, McpStatus,
    GlobalTagsViewModel, TopBarViewModel, MessageListViewModel,
    strip_thinking_from_assistant,
};

// ─── Build Helper Functions ─────────────────────────────────────────────────

pub fn build_top_bar_vm(state: &crate::tui::state::AppState) -> TopBarViewModel {
    TopBarViewModel::from_state(&state.top_bar, state.agent_running, state.animation.braille_frame, state.mode)
}

pub fn build_global_tags_vm(state: &crate::tui::state::AppState) -> GlobalTagsViewModel {
    if state.agent_running {
        let status = state.status_header.as_deref().unwrap_or("thinking");
        let elapsed = state.status_start_time
            .map(|t| format_duration_short(t.elapsed().as_secs()))
            .unwrap_or_default();
        let spinner = crate::glyphs::spinner_frame(state.animation.braille_frame);
        GlobalTagsViewModel::running(
            spinner, status, &elapsed, state.token_usage.total_tokens as u64,
            state.last_turn_duration_secs,
            state.last_turn_tokens,
            state.last_turn_tool_calls,
        )
    } else if let Some(ref header) = state.status_header {
        GlobalTagsViewModel::error(header)
    } else {
        GlobalTagsViewModel::idle()
    }
}

pub fn build_message_list_vm(
    state: &crate::tui::state::AppState,
    wrap_cache: crate::components::message_list::render::WrapCache,
) -> MessageListViewModel {
    let feed = if state.show_thoughts {
        Feed::from(&state.messages[..])
    } else {
        let messages_stripped = strip_and_clone_messages(state);
        Feed::from(messages_stripped)
    };
    MessageListViewModel::new(
        feed,
        state.scroll.feed_offset,
        state.agent_running,
        state.animation.clone(),
        wrap_cache,
        state.session_starting,
        state.thinking.as_ref().filter(|_| state.agent_running).map(|t| t.text.clone()),
    )
}

fn strip_and_clone_messages(state: &crate::tui::state::AppState) -> Vec<MessageItem> {
    state.messages.iter().map(|msg| match msg {
        MessageItem::Assistant { text, model, timestamp, expanded, thought_duration, turn_duration } => {
            MessageItem::Assistant {
                text: strip_thinking_from_assistant(text).into_owned(),
                model: model.clone(),
                timestamp: timestamp.clone(),
                expanded: *expanded,
                thought_duration: *thought_duration,
                turn_duration: *turn_duration,
            }
        }
        other => other.clone(),
    }).collect()
}

pub fn build_input_bar_vm(state: &crate::tui::state::AppState) -> InputBarViewModel {
    use crate::tui::state::PermissionMode;

    let mode_indicator = match state.permission_mode {
        PermissionMode::Normal => "Grok Build".to_string(),
        PermissionMode::Plan => "Grok Build · plan".to_string(),
        PermissionMode::AutoApprove => "Grok Build · always-approve".to_string(),
    };

    let char_count = {
        let text = state.textarea.lines().join("\n");
        let text_len = text.len();
        let ctx_window = state.top_bar.context_window.unwrap_or(512_000);
        let estimated_tokens = text_len / 4;
        if estimated_tokens > ctx_window / 2 { Some(text_len) } else { None }
    };

    InputBarViewModel {
        textarea: state.textarea.clone(),
        prompt: state.input_draft.clone(),
        right_info: state.input_right_info.clone(),
        placeholder: "Build anything...".to_string(),
        mode_indicator,
        attached_files: Vec::new(),
        char_count,
        context_window: state.top_bar.context_window,
    }
}

pub fn build_status_bar_vm(state: &crate::tui::state::AppState) -> StatusBarViewModel {
    StatusBarViewModel {
        mode: state.mode.clone(),
        current_model: state.current_model.clone(),
        session_token_usage: state.session_token_usage.clone(),
        status_header: state.status_header.clone(),
        status_details: state.status_details.clone(),
        status_start_time: state.status_start_time,
        mcp_status: McpStatus::None,
        agent_running: state.agent_running,
        input_has_text: !state.textarea.lines().join("").trim().is_empty(),
    }
}

pub fn build_agent_list_vm(state: &crate::tui::state::AppState) -> AgentListViewModel {
    AgentListViewModel {
        plan_steps: extract_plan_steps(&state.messages),
        running_jobs: state.background_jobs.clone(),
        active_count: state.background_jobs.iter()
            .filter(|j| matches!(j.status, crate::components::status_bar::JobStatus::Running))
            .count(),
        tokens: state.session_token_usage.total_tokens as u64,
        cost: 0.0,
        agent_running: state.agent_running,
        braille_frame: state.animation.braille_frame,
    }
}

pub fn build_command_palette_vm(
    state: &crate::tui::state::AppState,
    _palette: &CommandPalette,
) -> Option<CommandPaletteViewModel> {
    if state.command_palette.open {
        Some(CommandPaletteViewModel { show: true })
    } else {
        None
    }
}

pub fn build_permission_modal_vm(state: &crate::tui::state::AppState) -> Option<PermissionModalViewModel> {
    let pm = &state.permission_modal;
    if pm.tool.is_some() {
        Some(PermissionModalViewModel {
            tool: pm.tool.clone().unwrap_or_default(),
            args: pm.args.clone().unwrap_or_default(),
            desc: pm.desc.clone().unwrap_or_default(),
            selected: 0,
            visible: true,
            timeout_secs: pm.timeout_start.map(|t| {
                let elapsed = t.elapsed().as_secs();
                if elapsed < 60 { 60 - elapsed } else { 0 }
            }),
        })
    } else {
        None
    }
}

pub fn build_overlay_vm(_state: &crate::tui::state::AppState) -> Option<OverlayViewModel> {
    None
}

pub fn build_session_tree_vm(state: &crate::tui::state::AppState) -> Option<SessionTreeViewModel> {
    let nav = &state.session_tree;
    if nav.visible && !nav.entries.is_empty() {
        Some(SessionTreeViewModel {
            entries: nav.entries.clone(),
            selected: nav.selected,
            visible: true,
        })
    } else {
        None
    }
}

pub fn build_diff_viewer_vm(state: &crate::tui::state::AppState) -> Option<DiffViewerViewModel> {
    state.diff_viewer.as_ref().map(|dv| DiffViewerViewModel {
        filename: dv.filename.clone(),
        diff_lines: dv.compute_diff(),
        scroll_offset: dv.scroll_offset,
        visible: dv.visible,
    })
}

pub fn build_onboarding_vm(state: &crate::tui::state::AppState) -> Option<OnboardingViewModel> {
    state.onboarding.as_ref().map(|ob| OnboardingViewModel {
        step: map_onboarding_step(&ob.step),
        selected_item: ob.selected_item,
        selected_provider: ob.selected_provider,
        api_key_input: ob.api_key_input.clone(),
        selected_model: ob.selected_model,
        providers: ob.providers.iter().map(|p| p.name.clone()).collect(),
        models: ob.models.iter().map(|m| m.name.clone()).collect(),
        error_message: ob.error_message.clone().or(ob.fetch_error.clone()),
    })
}

// ─── Helper Functions ───────────────────────────────────────────────────────

fn format_duration_short(duration_secs: u64) -> String {
    if duration_secs < 60 {
        format!("{}s", duration_secs)
    } else {
        format!("{}m", duration_secs / 60)
    }
}

fn extract_plan_steps(messages: &[MessageItem]) -> Vec<(usize, String, PlanStatus)> {
    messages.iter().filter_map(|msg| {
        if let MessageItem::PlanStep { step, text, status } = msg {
            Some((*step, text.clone(), status.clone()))
        } else {
            None
        }
    }).collect()
}

fn map_onboarding_step(step: &crate::components::onboarding::OnboardingStep) -> OnboardingStep {
    use crate::components::onboarding::OnboardingStep as Os;
    match step {
        Os::Welcome => OnboardingStep::Welcome,
        Os::ProviderSelect => OnboardingStep::ProviderSelect,
        Os::KeyInput => OnboardingStep::KeyInput,
        Os::ModelSelect => OnboardingStep::ModelSelect,
        Os::Complete => OnboardingStep::Complete,
    }
}
