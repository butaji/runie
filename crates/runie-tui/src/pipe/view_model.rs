use crate::tui::state::{AppState, TuiMode};
use crate::tui::view_models::{
    AgentListViewModel, CommandPaletteViewModel, DiffViewerViewModel,
    OnboardingViewModel, OverlayViewModel, PermissionModalViewModel,
    SessionTreeViewModel, ViewModels,
};
use crate::components::message_list::render::WrapCache;
use crate::components::top_bar::TopBarBuilder;
use crate::components::message_list::FeedBuilder;
use crate::components::input_bar::InputBuilder;
use crate::components::status_bar::StatusBarBuilder;
use crate::components::permission_modal::PermissionBuilder;
use crate::components::onboarding::OnboardingBuilder;
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
        // TopBar - use TopBarBuilder
        let top_bar = TopBarBuilder::new()
            .repo(&state.top_bar.repo)
            .branch(&state.top_bar.branch)
            .path(&state.top_bar.path)
            .model(
                &state.top_bar.model,
                state.top_bar.context_window.unwrap_or(128_000),
            )
            .tokens(state.top_bar.estimated_tokens.unwrap_or(0))
            .build();

        // MessageList - use FeedBuilder with WrapCache
        let wrap_cache = WrapCache::new();
        let message_list = FeedBuilder::new()
            .messages(&state.messages)
            .scroll_offset(state.scroll.feed_offset)
            .agent_running(state.agent_running)
            .animation(state.animation.clone())
            .wrap_cache(wrap_cache)
            .build();

        // InputBar - use InputBuilder
        let input_text = state.textarea.lines().join("\n");
        let input_bar = InputBuilder::new()
            .text(&input_text)
            .prompt("❯ ")
            .info(&state.input_right_info)
            .build();

        // StatusBar - use StatusBarBuilder
        let status_bar = StatusBarBuilder::new()
            .mode(state.mode.clone())
            .current_model(state.current_model.as_deref().unwrap_or("—"))
            .session_token_usage(state.session_token_usage.clone())
            .status_header(state.status_header.as_deref().unwrap_or(""))
            .status_details(state.status_details.as_deref().unwrap_or(""))
            .status_start_time(state.status_start_time.unwrap_or_else(std::time::Instant::now))
            .build();

        // AgentList - inline (no builder exists)
        let agent_list = build_agent_list(state);

        // CommandPalette - inline (no builder exists)
        let command_palette = build_command_palette(state);

        // PermissionModal - use PermissionBuilder
        let permission_modal = build_permission_modal(state);

        // Overlay - inline (no builder exists)
        let overlay = build_overlay(state);

        // SessionTree - inline (no builder exists)
        let session_tree = build_session_tree(state);

        // DiffViewer - inline (no builder exists)
        let diff_viewer = build_diff_viewer(state);

        // Onboarding - use OnboardingBuilder
        let onboarding = build_onboarding(state);

        ViewModels {
            top_bar,
            message_list,
            input_bar,
            status_bar,
            agent_list,
            command_palette,
            permission_modal,
            overlay,
            session_tree,
            diff_viewer,
            onboarding,
        }
    }
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
    AgentListViewModel {
        plan_steps,
        running_jobs: running_jobs.clone(),
        active_count: running_jobs.len(),
        tokens: state.session_token_usage.total_tokens as u64,
        cost: state.session_token_usage.estimated_cost,
        agent_running: state.agent_running,
        braille_frame: state.animation.braille_frame,
    }
}

fn build_command_palette(state: &AppState) -> Option<CommandPaletteViewModel> {
    if state.mode != TuiMode::CommandPalette && !state.command_palette.open {
        return None;
    }
    Some(CommandPaletteViewModel {
        show: state.command_palette.open,
    })
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
    Some(OverlayViewModel {
        title: String::new(),
        content: vec![],
        tabs: vec![],
        active_tab: 0,
        show_close: true,
    })
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
        let step = match o.step {
            crate::components::onboarding::OnboardingStep::Welcome => {
                crate::tui::view_models::OnboardingStep::Welcome
            }
            crate::components::onboarding::OnboardingStep::ProviderSelect => {
                crate::tui::view_models::OnboardingStep::ProviderSelect
            }
            crate::components::onboarding::OnboardingStep::KeyInput => {
                crate::tui::view_models::OnboardingStep::KeyInput
            }
            crate::components::onboarding::OnboardingStep::ModelSelect => {
                crate::tui::view_models::OnboardingStep::ModelSelect
            }
            crate::components::onboarding::OnboardingStep::Complete => {
                crate::tui::view_models::OnboardingStep::Complete
            }
        };
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