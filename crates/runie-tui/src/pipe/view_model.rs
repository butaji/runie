use crate::tui::state::{AppState, TuiMode};
use crate::tui::view_models::{
    AgentListViewModel, CommandPaletteViewModel, DiffViewerViewModel,
    OnboardingViewModel, OverlayViewModel, PermissionModalViewModel,
    SessionTreeViewModel, ViewModels, InputBarViewModel, StatusBarViewModel,
};
use crate::components::global_tags::GlobalTagsViewModel;
use crate::components::top_bar::TopBarViewModel;
use crate::components::message_list::MessageListViewModel;
use crate::components::message_list::render::WrapCache;
use crate::components::message_list::FeedBuilder;
use crate::components::input_bar::InputBuilder;
use crate::components::status_bar::StatusBarBuilder;
use crate::components::permission_modal::PermissionBuilder;
use crate::components::onboarding::OnboardingBuilder;
use crate::components::agent_list::AgentListBuilder;
use crate::components::command_palette::CommandPaletteBuilder;
use crate::components::overlay::OverlayBuilder;
use crate::components::message_list::PlanStatus;

/// ViewModelPipe transforms AppState into ViewModels using builders.
pub struct ViewModelPipe;

impl ViewModelPipe {

    #[must_use]
    
    pub fn new() -> Self {
        Self
    }

    pub fn build(&self, state: &AppState) -> ViewModels {
        ViewModels {
            global_tags: Some(build_global_tags(state)),
            message_list: Some(build_message_list(state)),
            input_bar: Some(build_input_bar(state)),
            status_bar: Some(build_status_bar(state)),
            agent_list: Some(build_agent_list(state)),
            command_palette: build_command_palette(state),
            permission_modal: build_permission_modal(state),
            overlay: build_overlay(state),
            session_tree: build_session_tree(state),
            diff_viewer: build_diff_viewer(state),
            onboarding: build_onboarding(state),
            top_bar: Some(build_top_bar(state)),
            code_blocks: Vec::new(),
            collapsibles: Vec::new(),
            context_panel: None,
        }
    }
}

impl Default for ViewModelPipe {
    fn default() -> Self {
        Self::new()
    }
}

// ─── View model builders ────────────────────────────────────────────────────────

fn build_top_bar(state: &AppState) -> TopBarViewModel {
    // Top bar shows repo/branch/path from context state (set by SetGitInfo)
    // Model info belongs ONLY in global_tags, not top bar
    TopBarViewModel {
        repo: state.context.repo.clone(),
        branch: state.context.branch.clone(),
        path: state.context.path.clone(),
        context_window: state.top_bar.context_window.unwrap_or(512_000),
        estimated_tokens: state.top_bar.estimated_tokens.unwrap_or(0),
        agent_running: state.agent_running,
        braille_frame: state.animation.braille_frame,
        mode: state.mode.clone(),
    }
}

fn build_global_tags(state: &AppState) -> GlobalTagsViewModel {
    use crate::messages::MessageRegistry;
    let _model = state.current_model.as_deref().unwrap_or("—");
    let tokens = state.session_token_usage.total_tokens as u64;
    let _cost = state.session_token_usage.estimated_cost;

    if state.agent_running {
        // Bug 3 fix: Use MessageRegistry for consistent casing ("Running" not "running")
        let status = state.status_header.as_deref().unwrap_or(MessageRegistry::status_running());
        // Error state: no spinner, no turn info
        if status == MessageRegistry::status_error() {
            return GlobalTagsViewModel::error(status);
        }
        let time = state.status_details.as_deref().unwrap_or("0s");
        let spinner = crate::glyphs::SPINNER_FRAMES[state.animation.braille_frame % 10];
        GlobalTagsViewModel::running(spinner, status, time, tokens, state.last_turn_duration_secs, state.last_turn_tokens, state.last_turn_tool_calls)
    } else {
        GlobalTagsViewModel::idle()
    }
}

fn build_message_list(state: &AppState) -> MessageListViewModel {
    let wrap_cache = WrapCache::new();
    // Pass streaming thinking content if agent is running and thinking exists
    let streaming_think_content = if state.agent_running {
        state.thinking.as_ref().map(|t| t.text.clone())
    } else {
        None
    };
    FeedBuilder::new()
        .messages(&state.messages)
        .scroll_offset(state.scroll.feed_offset)
        .agent_running(state.agent_running)
        .animation(state.animation.clone())
        .wrap_cache(wrap_cache)
        .session_starting(state.session_starting)
        .streaming_think_content(streaming_think_content)
        .build()
}

fn build_input_bar(state: &AppState) -> InputBarViewModel {
    use crate::tui::state::PermissionMode;

    // Build mode indicator
    let mode_indicator = match state.permission_mode {
        PermissionMode::Normal => "Grok Build".to_string(),
        PermissionMode::Plan => "Grok Build · plan".to_string(),
        PermissionMode::AutoApprove => "Grok Build · always-approve".to_string(),
    };

    let input_text = state.textarea.lines().join("\n");
    InputBuilder::new()
        .text(&input_text)
        .prompt(format!("{ch} ", ch = crate::glyphs::CHEVRON).as_str())
        .info(&state.input_right_info)
        .mode_indicator(&mode_indicator)
        .build()
}

fn build_status_bar(state: &AppState) -> StatusBarViewModel {
    let input_text = state.textarea.lines().join("\n");
    let agent_running = state.agent_running;
    // DEBUG: Trace agent_running state
    tracing::debug!(
        "build_status_bar: agent_running={}, mode={:?}, status_header={:?}",
        agent_running,
        state.mode,
        state.status_header
    );
    StatusBarBuilder::new()
        .mode(state.mode.clone())
        .current_model(state.current_model.as_deref().unwrap_or("—"))
        .session_token_usage(state.session_token_usage.clone())
        .status_header(state.status_header.as_deref().unwrap_or(""))
        .status_details(state.status_details.as_deref().unwrap_or(""))
        .status_start_time(state.status_start_time.unwrap_or_else(std::time::Instant::now))
        .agent_running(agent_running)
        .input_has_text(!input_text.trim().is_empty())
        .build()
}

// ─── Inline builders for ViewModels without dedicated builders ─────────────────

fn build_agent_list(state: &AppState) -> AgentListViewModel {
    let plan_steps = extract_plan_steps(&state.messages);
    let running_jobs: Vec<_> = state
        .background_jobs
        .iter()
        .filter(|j| j.status == crate::components::status_bar::JobStatus::Running)
        .cloned()
        .collect();

    let mut builder = AgentListBuilder::new()
        .tokens(state.session_token_usage.total_tokens as u64)
        .cost(state.session_token_usage.estimated_cost)
        .agent_running(state.agent_running)
        .braille_frame(state.animation.braille_frame);

    for step in plan_steps {
        builder = builder.plan_step(step.0, &step.1, step.2);
    }

    for job in &running_jobs {
        builder = builder.running_job(&job.name);
    }

    builder.build()
}

fn build_command_palette(state: &AppState) -> Option<CommandPaletteViewModel> {
    if state.mode != TuiMode::CommandPalette && !state.command_palette.open {
        return None;
    }
    Some(CommandPaletteBuilder::new().visible(state.command_palette.open).build())
}

fn build_permission_modal(state: &AppState) -> Option<PermissionModalViewModel> {
    if state.mode != TuiMode::Permission {
        return None;
    }
    const TIMEOUT_SECS: u64 = 300;
    let timeout_secs = state.permission_modal.timeout_start.map(|start| {
        let elapsed = start.elapsed().as_secs();
        TIMEOUT_SECS.saturating_sub(elapsed)
    });

    let tool = state.permission_modal.tool.as_deref().unwrap_or("");
    let args = state.permission_modal.args.as_deref().unwrap_or("");
    let desc = state.permission_modal.desc.as_deref().unwrap_or("");

    Some(
        PermissionBuilder::new()
            .tool(tool, args)
            .description(desc)
            .timeout_secs(timeout_secs.unwrap_or(TIMEOUT_SECS))
            .build(),
    )
}

fn build_overlay(state: &AppState) -> Option<OverlayViewModel> {
    if state.mode != TuiMode::Overlay {
        return None;
    }
    Some(OverlayBuilder::new().build())
}

fn build_session_tree(state: &AppState) -> Option<SessionTreeViewModel> {
    if state.mode != TuiMode::SessionTree {
        return None;
    }
    Some(SessionTreeViewModel {
        entries: state.session_tree.entries.clone(),
        selected: state.session_tree.selected,
        visible: state.session_tree.visible,
    })
}

fn build_diff_viewer(state: &AppState) -> Option<DiffViewerViewModel> {
    state.diff_viewer.as_ref().map(|dv| DiffViewerViewModel {
        filename: dv.filename.clone(),
        diff_lines: dv.compute_diff(),
        scroll_offset: dv.scroll_offset,
        visible: dv.visible,
    })
}

fn build_onboarding(state: &AppState) -> Option<OnboardingViewModel> {
    state.onboarding.as_ref().map(|o| {
        OnboardingBuilder::new()
            .step(o.step)
            .selected_item(o.selected_item)
            .selected_provider(o.selected_provider)
            .selected_model(o.selected_model)
            .key(&o.api_key_input)
            .providers(o.providers.iter().map(|p| p.name.clone()).collect())
            .models(o.models.iter().map(|m| m.name.clone()).collect())
            .error_message(o.error_message.as_deref().unwrap_or(""))
            .build()
    })
}

fn extract_plan_steps(messages: &[crate::components::MessageItem]) -> Vec<(usize, String, PlanStatus)> {
    messages
        .iter()
        .filter_map(|msg| {
            if let crate::components::MessageItem::PlanStep { step, text, status } = msg {
                Some((*step, text.clone(), status.clone()))
            } else {
                None
            }
        })
        .collect()
}
