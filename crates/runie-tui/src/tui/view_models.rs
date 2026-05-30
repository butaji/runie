use crate::components::{
    MessageItem, GitChange,
    SessionTreeEntry, LineStatus,
};
use crate::components::status_bar::BackgroundJob;
use crate::components::diff_viewer::DiffLine;
use crate::components::command_palette::CommandPalette;
use crate::components::global_tags::GlobalTagsViewModel;
use crate::components::top_bar::TopBarViewModel;
pub use crate::components::message_list::MessageListViewModel;
use crate::components::message_list::PlanStatus;
use crate::tui::state::TuiMode;
use runie_ai::TokenUsage;

// ─── InputBarViewModel ──────────────────────────────────────────────────────
pub struct InputBarViewModel {
    pub textarea: ratatui_textarea::TextArea<'static>,
    pub prompt: String,
    pub right_info: String,
}

// ─── StatusBarViewModel ─────────────────────────────────────────────────────
pub struct StatusBarViewModel {
    pub mode: TuiMode,
    pub current_model: Option<String>,
    pub session_token_usage: TokenUsage,
    // Live status indicator
    pub status_header: Option<String>,
    pub status_details: Option<String>,
    pub status_start_time: Option<std::time::Instant>,
}

// ─── AgentListViewModel ─────────────────────────────────────────────────────
pub struct AgentListViewModel {
    pub plan_steps: Vec<(usize, String, PlanStatus)>,
    pub running_jobs: Vec<BackgroundJob>,
    pub active_count: usize,
    pub tokens: u64,
    pub cost: f64,
    pub agent_running: bool,
    pub braille_frame: usize,
}

// ─── CommandPaletteViewModel ────────────────────────────────────────────────
pub struct CommandPaletteViewModel {
    pub show: bool,
}

// ─── PermissionModalViewModel ───────────────────────────────────────────────
pub struct PermissionModalViewModel {
    pub tool: String,
    pub args: String,
    pub desc: String,
    pub selected: usize,
    pub visible: bool,
    // P0-3 FIX: Add timeout countdown display
    pub timeout_secs: Option<u64>,
}

// ─── OverlayViewModel ───────────────────────────────────────────────────────
pub struct OverlayViewModel {
    pub title: String,
    pub content: Vec<String>,
    pub tabs: Vec<String>,
    pub active_tab: usize,
    pub show_close: bool,
}

// ─── SessionTreeViewModel ───────────────────────────────────────────────────
pub struct SessionTreeViewModel {
    pub entries: Vec<SessionTreeEntry>,
    pub selected: usize,
    pub visible: bool,
}

// ─── DiffViewerViewModel ────────────────────────────────────────────────────
pub struct DiffViewerViewModel {
    pub filename: String,
    pub diff_lines: Vec<DiffLine>,
    pub scroll_offset: usize,
    pub visible: bool,
}

// ─── OnboardingViewModel ───────────────────────────────────────────────────
pub struct OnboardingViewModel {
    pub step: OnboardingStep,
    pub selected_item: usize,
    pub selected_provider: Option<usize>,
    pub api_key_input: String,
    pub selected_model: Option<usize>,
    pub providers: Vec<String>,
    pub models: Vec<String>,
    pub error_message: Option<String>,
}

#[derive(Clone)]
pub enum OnboardingStep {
    Welcome,
    ProviderSelect,
    KeyInput,
    ModelSelect,
    Complete,
}

// ─── CodeBlockViewModel ─────────────────────────────────────────────────────
pub struct CodeBlockViewModel {
    pub lines: Vec<CodeLineViewModel>,
    pub start_line: usize,
    pub language: Option<String>,
}

pub struct CodeLineViewModel {
    pub number: usize,
    pub text: String,
    pub status: LineStatus,
}

// ─── CollapsibleViewModel ───────────────────────────────────────────────────
pub struct CollapsibleViewModel {
    pub title: String,
    pub expanded: bool,
    pub content_lines: Vec<String>,
}

// ─── ContextPanelViewModel ─────────────────────────────────────────────────
pub struct ContextPanelViewModel {
    pub recent_files: Vec<String>,
    pub git_changes: Vec<GitChange>,
    pub active_tool: Option<String>,
    pub model_name: String,
    pub session_info: String,
}

// ─── ViewModels ─────────────────────────────────────────────────────────────
pub struct ViewModels {
    pub global_tags: GlobalTagsViewModel,
    pub message_list: MessageListViewModel,
    pub input_bar: InputBarViewModel,
    pub status_bar: StatusBarViewModel,
    pub agent_list: AgentListViewModel,
    pub command_palette: Option<CommandPaletteViewModel>,
    pub permission_modal: Option<PermissionModalViewModel>,
    pub overlay: Option<OverlayViewModel>,
    pub session_tree: Option<SessionTreeViewModel>,
    pub diff_viewer: Option<DiffViewerViewModel>,
    pub onboarding: Option<OnboardingViewModel>,
    pub top_bar: TopBarViewModel,
}

impl ViewModels {
    pub fn from_render_state(state: &crate::tui::state::RenderState, palette: &CommandPalette, wrap_cache: crate::components::message_list::render::WrapCache) -> Self {
        Self {
            global_tags: build_global_tags_vm(state),
            message_list: build_message_list_vm(state, wrap_cache),
            input_bar: build_input_bar_vm(state),
            status_bar: build_status_bar_vm(state),
            agent_list: build_agent_list_vm(state),
            command_palette: build_command_palette_vm(state, palette),
            permission_modal: build_permission_modal_vm(state),
            overlay: build_overlay_vm(state),
            session_tree: build_session_tree_vm(state),
            diff_viewer: build_diff_viewer_vm(state),
            onboarding: build_onboarding_vm(state),
            top_bar: build_top_bar_vm(state),
        }
    }
}

// ─── Builder Helpers ────────────────────────────────────────────────────────

fn build_top_bar_vm(state: &crate::tui::state::RenderState) -> TopBarViewModel {
    TopBarViewModel {
        repo: state.top_bar.repo.clone(),
        branch: state.top_bar.branch.clone(),
        path: state.top_bar.path.clone(),
        model: state.top_bar.model.clone(),
        context_window: state.top_bar.context_window.unwrap_or(128_000),
        estimated_tokens: state.top_bar.estimated_tokens.unwrap_or(0),
    }
}

fn build_global_tags_vm(state: &crate::tui::state::RenderState) -> GlobalTagsViewModel {
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

fn build_message_list_vm(state: &crate::tui::state::RenderState, wrap_cache: crate::components::message_list::render::WrapCache) -> MessageListViewModel {
    use crate::components::message_list::Feed;
    MessageListViewModel::new(
        Feed::from(state.messages.clone()),
        state.scroll.feed_offset,
        state.agent_running,
        state.animation.clone(),
        wrap_cache,
    )
}

fn build_input_bar_vm(state: &crate::tui::state::RenderState) -> InputBarViewModel {
    InputBarViewModel {
        textarea: state.textarea.clone(),
        prompt: "❯ ".to_string(),
        right_info: state.input_right_info.clone(),
    }
}

fn build_status_bar_vm(state: &crate::tui::state::RenderState) -> StatusBarViewModel {
    StatusBarViewModel {
        mode: state.mode.clone(),
        current_model: state.current_model.clone(),
        session_token_usage: state.session_token_usage.clone(),
        status_header: state.status_header.clone(),
        status_details: state.status_details.clone(),
        status_start_time: state.status_start_time,
    }
}

fn build_agent_list_vm(state: &crate::tui::state::RenderState) -> AgentListViewModel {
    let plan_steps = extract_plan_steps(&state.messages);
    let running_jobs: Vec<_> = state.background_jobs.iter()
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

fn build_command_palette_vm(
    state: &crate::tui::state::RenderState,
    _palette: &CommandPalette,
) -> Option<CommandPaletteViewModel> {
    if state.mode != TuiMode::CommandPalette && !state.command_palette.open {
        return None;
    }
    Some(CommandPaletteViewModel {
        show: state.command_palette.open,
    })
}

fn build_permission_modal_vm(state: &crate::tui::state::RenderState) -> Option<PermissionModalViewModel> {
    if state.mode != TuiMode::Permission {
        return None;
    }
    
    // P0-3 FIX: Calculate remaining timeout seconds
    const TIMEOUT_SECS: u64 = 300; // 5 minutes
    let timeout_secs = state.permission_modal.timeout_start.map(|start| {
        let elapsed = start.elapsed().as_secs();
        TIMEOUT_SECS.saturating_sub(elapsed)
    });
    
    Some(PermissionModalViewModel {
        tool: state.permission_modal.tool.clone().unwrap_or_default(),
        args: state.permission_modal.args.clone().unwrap_or_default(),
        desc: state.permission_modal.desc.clone().unwrap_or_default(),
        selected: 0,
        visible: true,
        timeout_secs,
    })
}

fn build_overlay_vm(state: &crate::tui::state::RenderState) -> Option<OverlayViewModel> {
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

fn build_session_tree_vm(state: &crate::tui::state::RenderState) -> Option<SessionTreeViewModel> {
    if state.mode != TuiMode::SessionTree {
        return None;
    }
    Some(SessionTreeViewModel {
        entries: state.session_tree.entries.clone(),
        selected: state.session_tree.selected,
        visible: state.session_tree.visible,
    })
}

fn build_diff_viewer_vm(state: &crate::tui::state::RenderState) -> Option<DiffViewerViewModel> {
    state.diff_viewer.as_ref().map(|dv| DiffViewerViewModel {
        filename: dv.filename.clone(),
        diff_lines: dv.compute_diff(),
        scroll_offset: dv.scroll_offset,
        visible: dv.visible,
    })
}

fn build_onboarding_vm(state: &crate::tui::state::RenderState) -> Option<OnboardingViewModel> {
    state.onboarding.as_ref().map(|o| OnboardingViewModel {
        step: map_onboarding_step(&o.step),
        selected_item: o.selected_item,
        selected_provider: o.selected_provider,
        api_key_input: o.api_key_input.clone(),
        selected_model: o.selected_model,
        providers: o.providers.iter().map(|p| p.name.clone()).collect(),
        models: o.models.iter().map(|m| m.name.clone()).collect(),
        error_message: o.error_message.clone(),
    })
}

// ─── Helper Functions ───────────────────────────────────────────────────────

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
