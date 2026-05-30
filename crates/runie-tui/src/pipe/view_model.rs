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
use super::Pipe;

/// ViewModelPipe transforms AppState into ViewModels using builders.
pub struct ViewModelPipe;

impl ViewModelPipe {
    pub fn new() -> Self {
        Self
    }
}

impl Default for ViewModelPipe {
    fn default() -> Self {
        Self::new()
    }
}

impl Pipe<&AppState> for ViewModelPipe {
    type Output = ViewModels;

    fn pipe(&self, state: &AppState) -> ViewModels {
        ViewModels {
            global_tags: build_global_tags(state),
            message_list: build_message_list(state),
            input_bar: build_input_bar(state),
            status_bar: build_status_bar(state),
            agent_list: build_agent_list(state),
            command_palette: build_command_palette(state),
            permission_modal: build_permission_modal(state),
            overlay: build_overlay(state),
            session_tree: build_session_tree(state),
            diff_viewer: build_diff_viewer(state),
            onboarding: build_onboarding(state),
            top_bar: build_top_bar(state),
        }
    }
}

// ─── View model builders ────────────────────────────────────────────────────────

fn build_top_bar(state: &AppState) -> TopBarViewModel {
    TopBarViewModel {
        repo: state.top_bar.repo.clone(),
        branch: state.top_bar.branch.clone(),
        path: state.top_bar.path.clone(),
        model: state.top_bar.model.clone(),
        context_window: state.top_bar.context_window.unwrap_or(128_000),
        estimated_tokens: state.top_bar.estimated_tokens.unwrap_or(0),
    }
}

fn build_global_tags(state: &AppState) -> GlobalTagsViewModel {
    let model = state.current_model.as_deref().unwrap_or("—");
    let tokens = state.session_token_usage.total_tokens as u64;
    let cost = state.session_token_usage.estimated_cost;

    if state.agent_running {
        let status = state.status_header.as_deref().unwrap_or("running");
        let time = state.status_details.as_deref().unwrap_or("0s");
        GlobalTagsViewModel::running(status, time, tokens)
    } else {
        GlobalTagsViewModel::idle(model, tokens, cost)
    }
}

fn build_message_list(state: &AppState) -> MessageListViewModel {
    let wrap_cache = WrapCache::new();
    FeedBuilder::new()
        .messages(&state.messages)
        .scroll_offset(state.scroll.feed_offset)
        .agent_running(state.agent_running)
        .animation(state.animation.clone())
        .wrap_cache(wrap_cache)
        .build()
}

fn build_input_bar(state: &AppState) -> InputBarViewModel {
    let input_text = state.textarea.lines().join("\n");
    InputBuilder::new()
        .text(&input_text)
        .prompt("❯ ")
        .info(&state.input_right_info)
        .build()
}

fn build_status_bar(state: &AppState) -> StatusBarViewModel {
    StatusBarBuilder::new()
        .mode(state.mode.clone())
        .current_model(state.current_model.as_deref().unwrap_or("—"))
        .session_token_usage(state.session_token_usage.clone())
        .status_header(state.status_header.as_deref().unwrap_or(""))
        .status_details(state.status_details.as_deref().unwrap_or(""))
        .status_start_time(state.status_start_time.unwrap_or_else(std::time::Instant::now))
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
        let step = convert_onboarding_step(o.step.clone());
        OnboardingBuilder::new()
            .step(step)
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

fn convert_onboarding_step(step: crate::components::onboarding::OnboardingStep) -> crate::tui::view_models::OnboardingStep {
    use crate::components::onboarding::OnboardingStep as Src;
    use crate::tui::view_models::OnboardingStep as Dst;
    match step {
        Src::Welcome => Dst::Welcome,
        Src::ProviderSelect => Dst::ProviderSelect,
        Src::KeyInput => Dst::KeyInput,
        Src::ModelSelect => Dst::ModelSelect,
        Src::Complete => Dst::Complete,
    }
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
