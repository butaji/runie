use crate::components::{
    MessageItem, GitChange,
    PaletteItem, PaletteStep, SessionTreeEntry, LineStatus,
};
use crate::components::status_bar::BackgroundJob;
use crate::components::diff_viewer::DiffLine;
use crate::components::command_palette::CommandPalette;
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
    pub background_jobs: Vec<BackgroundJob>,
    pub braille_frame: usize,
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
    pub step: PaletteStep,
    pub query: String,
    pub selected: usize,
    pub objects: Vec<PaletteItem>,
    pub actions: Vec<PaletteItem>,
    pub arguments: Vec<String>,
    pub show: bool,
}

// ─── PermissionModalViewModel ───────────────────────────────────────────────
pub struct PermissionModalViewModel {
    pub tool: String,
    pub args: String,
    pub desc: String,
    pub selected: usize,
    pub visible: bool,
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
    pub top_bar: TopBarViewModel,
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
}

impl ViewModels {
    pub fn from_render_state(state: &crate::tui::state::RenderState, palette: &CommandPalette) -> Self {
        Self {
            top_bar: TopBarViewModel::from_state(&state.top_bar),
            message_list: build_message_list_vm(state),
            input_bar: build_input_bar_vm(state),
            status_bar: build_status_bar_vm(state),
            agent_list: build_agent_list_vm(state),
            command_palette: build_command_palette_vm(state, palette),
            permission_modal: build_permission_modal_vm(state),
            overlay: build_overlay_vm(state),
            session_tree: build_session_tree_vm(state),
            diff_viewer: build_diff_viewer_vm(state),
            onboarding: build_onboarding_vm(state),
        }
    }
}

// ─── Builder Helpers ────────────────────────────────────────────────────────

fn build_message_list_vm(state: &crate::tui::state::RenderState) -> MessageListViewModel {
    MessageListViewModel {
        messages: state.messages.clone(),
        scroll_offset: state.scroll.feed_offset,
        agent_running: state.agent_running,
        animation: state.animation.clone(),
    }
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
        background_jobs: state.background_jobs.clone(),
        braille_frame: state.animation.braille_frame,
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
    palette: &CommandPalette,
) -> Option<CommandPaletteViewModel> {
    if state.mode != TuiMode::CommandPalette && !state.command_palette.open {
        return None;
    }
    Some(CommandPaletteViewModel {
        step: palette.step.clone(),
        query: palette.query.clone(),
        selected: palette.selected,
        objects: palette.objects.clone(),
        actions: palette.actions.clone(),
        arguments: vec![],
        show: state.command_palette.open,
    })
}

fn build_permission_modal_vm(state: &crate::tui::state::RenderState) -> Option<PermissionModalViewModel> {
    if state.mode != TuiMode::Permission {
        return None;
    }
    Some(PermissionModalViewModel {
        tool: state.permission_modal.tool.clone().unwrap_or_default(),
        args: state.permission_modal.args.clone().unwrap_or_default(),
        desc: state.permission_modal.desc.clone().unwrap_or_default(),
        selected: 0,
        visible: true,
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
