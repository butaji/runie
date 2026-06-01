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
use crate::messages::MessageRegistry;
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
    pub fn from_app_state(state: &crate::tui::state::AppState, palette: &CommandPalette, wrap_cache: crate::components::message_list::render::WrapCache) -> Self {
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

/// Strip thinking text from assistant messages.
/// Models embed thinking as lines starting with markers like · • ◦ ▸.
/// Also strips [thinking:...] wrappers and <think> blocks.
pub fn strip_thinking_from_assistant(text: &str) -> String {
    let lines: Vec<&str> = text.lines().collect();
    let mut result = Vec::new();
    let mut in_thinking_block = false;
    let mut thinking_depth = 0usize;

    for line in &lines {
        let trimmed = line.trim();

        // Handle <think> XML blocks (DeepSeek style)
        if trimmed.starts_with("<think>") {
            in_thinking_block = true;
            thinking_depth = 1;
            if trimmed.contains("</think>") {
                in_thinking_block = false;
                thinking_depth = 0;
            }
            continue;
        }
        if in_thinking_block {
            if trimmed.starts_with("</think>") {
                in_thinking_block = false;
                thinking_depth = 0;
            }
            continue;
        }

        // Handle [thinking:...] wrapper (our agent format)
        if trimmed.starts_with("[thinking:") && trimmed.ends_with("]") {
            continue;
        }

        // Check if line starts with a thinking marker
        let first_char = trimmed.chars().next();
        let is_thinking_marker = first_char.map_or(false, |c| {
            matches!(c, '·' | '•' | '◦' | '▸' | '▹')
        });

        if is_thinking_marker {
            continue;
        }

        result.push(*line);
    }

    result.join("\n")
}

// ─── Build Helper Functions ─────────────────────────────────────────────────

fn build_top_bar_vm(state: &crate::tui::state::AppState) -> TopBarViewModel {
    TopBarViewModel::from_state(&state.top_bar)
}

fn build_global_tags_vm(state: &crate::tui::state::AppState) -> GlobalTagsViewModel {
    if state.agent_running {
        // Show spinner with status while agent is running
        let status = state.status_header.as_deref().unwrap_or("thinking");
        let elapsed = state.status_start_time
            .map(|t| {
                let dur = t.elapsed().as_secs();
                format_duration_short(dur)
            })
            .unwrap_or_default();
        let mut vm = GlobalTagsViewModel::running(
            status, &elapsed, state.token_usage.total_tokens as u64,
            state.last_turn_duration_secs,
            state.last_turn_tokens,
            state.last_turn_tool_calls,
        );
        vm.left = Some(format!("{} {} · Ctrl+Enter:interject", crate::glyphs::spinner_frame(state.animation.braille_frame), status));
        vm
    } else if let Some(ref header) = state.status_header {
        // Error state
        GlobalTagsViewModel::error(header)
    } else {
        // Idle state - empty
        GlobalTagsViewModel::idle()
    }
}

fn build_message_list_vm(
    state: &crate::tui::state::AppState,
    wrap_cache: crate::components::message_list::render::WrapCache,
) -> MessageListViewModel {
    use crate::components::message_list::Feed;

    // Strip thinking from assistant messages when show_thoughts is false
    let messages_stripped: Vec<MessageItem> = state.messages.iter().map(|msg| {
        match msg {
            MessageItem::Assistant { text, model, timestamp } if !state.show_thoughts => {
                MessageItem::Assistant {
                    text: strip_thinking_from_assistant(text),
                    model: model.clone(),
                    timestamp: timestamp.clone(),
                }
            }
            other => other.clone(),
        }
    }).collect();

    let feed = Feed::from(messages_stripped);

    MessageListViewModel::new(
        feed,
        state.scroll.feed_offset,
        state.agent_running,
        state.animation.clone(),
        wrap_cache,
    )
}

fn build_input_bar_vm(state: &crate::tui::state::AppState) -> InputBarViewModel {
    InputBarViewModel {
        textarea: state.textarea.clone(),
        prompt: state.input_draft.clone(),
        right_info: state.input_right_info.clone(),
    }
}

fn build_status_bar_vm(state: &crate::tui::state::AppState) -> StatusBarViewModel {
    StatusBarViewModel {
        mode: state.mode.clone(),
        current_model: state.current_model.clone(),
        session_token_usage: state.session_token_usage.clone(),
        status_header: state.status_header.clone(),
        status_details: state.status_details.clone(),
        status_start_time: state.status_start_time,
    }
}

fn build_agent_list_vm(state: &crate::tui::state::AppState) -> AgentListViewModel {
    AgentListViewModel {
        plan_steps: extract_plan_steps(&state.messages),
        running_jobs: state.background_jobs.clone(),
        active_count: state.background_jobs.iter().filter(|j| matches!(j.status, crate::components::status_bar::JobStatus::Running)).count(),
        tokens: state.session_token_usage.total_tokens as u64,
        cost: 0.0, // Cost calculation requires pricing data
        agent_running: state.agent_running,
        braille_frame: state.animation.braille_frame,
    }
}

fn build_command_palette_vm(
    state: &crate::tui::state::AppState,
    _palette: &CommandPalette,
) -> Option<CommandPaletteViewModel> {
    if state.command_palette.open {
        Some(CommandPaletteViewModel {
            show: true,
        })
    } else {
        None
    }
}

fn build_permission_modal_vm(state: &crate::tui::state::AppState) -> Option<PermissionModalViewModel> {
    let pm = &state.permission_modal;
    if pm.tool.is_some() {
        Some(PermissionModalViewModel {
            tool: pm.tool.clone().unwrap_or_default(),
            args: pm.args.clone().unwrap_or_default(),
            desc: pm.desc.clone().unwrap_or_default(),
            selected: 0,
            visible: true,
            timeout_secs: pm.timeout_start.map(|t| {
                // Calculate remaining seconds (60 second default timeout)
                let elapsed = t.elapsed().as_secs();
                if elapsed < 60 { 60 - elapsed } else { 0 }
            }),
        })
    } else {
        None
    }
}

fn build_overlay_vm(state: &crate::tui::state::AppState) -> Option<OverlayViewModel> {
    // Overlay is shown when there's a context panel or similar
    // For now, return None unless explicitly needed
    None
}

fn build_session_tree_vm(state: &crate::tui::state::AppState) -> Option<SessionTreeViewModel> {
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

fn build_diff_viewer_vm(state: &crate::tui::state::AppState) -> Option<DiffViewerViewModel> {
    state.diff_viewer.as_ref().map(|dv| DiffViewerViewModel {
        filename: dv.filename.clone(),
        diff_lines: dv.compute_diff(),
        scroll_offset: dv.scroll_offset,
        visible: dv.visible,
    })
}

fn build_onboarding_vm(state: &crate::tui::state::AppState) -> Option<OnboardingViewModel> {
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
