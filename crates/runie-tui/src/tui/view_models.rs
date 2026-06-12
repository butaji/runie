//! ViewModel types and helpers for the TUI.

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

// Re-export OnboardingStep from components to avoid duplication
pub use crate::components::onboarding::OnboardingStep;

// Build functions are in view_models_build module (declared in tui.rs)
pub use super::view_models_build::*;

// ─── InputBarViewModel ──────────────────────────────────────────────────────
#[derive(Debug)]
pub struct InputBarViewModel {
    pub textarea: ratatui_textarea::TextArea<'static>,
    pub prompt: String,
    pub right_info: String,
    /// Placeholder text shown when empty and unfocused
    pub placeholder: String,
    /// Mode indicator text (e.g., "runie", "runie · plan", "runie · yolo")
    pub mode_indicator: String,
    /// List of attached file names to display as pills
    pub attached_files: Vec<String>,
    /// Character count for long inputs
    pub char_count: Option<usize>,
    /// Context window size for calculating threshold
    pub context_window: Option<usize>,
}

// ─── McpStatus ─────────────────────────────────────────────────────────────
#[derive(Debug, Clone)]
pub enum McpStatus {
    Connected(u32),
    Unavailable(u32),
    None,
}

// ─── StatusBarViewModel ─────────────────────────────────────────────────────
#[derive(Debug)]
pub struct StatusBarViewModel {
    pub mode: TuiMode,
    pub current_model: Option<String>,
    pub session_token_usage: runie_ai::TokenUsage,
    // Live status indicator
    pub status_header: Option<String>,
    pub status_details: Option<String>,
    pub status_start_time: Option<std::time::Instant>,
    pub mcp_status: McpStatus,
    pub agent_running: bool,
    pub input_has_text: bool,
}

impl Default for StatusBarViewModel {
    fn default() -> Self {
        Self {
            mode: TuiMode::Chat,
            current_model: None,
            session_token_usage: runie_ai::TokenUsage::default(),
            status_header: None,
            status_details: None,
            status_start_time: None,
            mcp_status: McpStatus::None,
            agent_running: false,
            input_has_text: false,
        }
    }
}

// ─── AgentListViewModel ─────────────────────────────────────────────────────
#[derive(Debug, Clone)]
pub struct AgentListViewModel {
    pub plan_steps: Vec<(usize, String, PlanStatus)>,
    pub running_jobs: Vec<BackgroundJob>,
    pub active_count: usize,
    pub tokens: u64,
    pub cost: f64,
    pub agent_running: bool,
    pub braille_frame: u8,
}

// ─── CommandPaletteViewModel ────────────────────────────────────────────────
#[derive(Debug, Clone)]
pub struct CommandPaletteViewModel {
    pub show: bool,
}

// ─── PermissionModalViewModel ───────────────────────────────────────────────
#[derive(Debug, Clone)]
pub struct PermissionModalViewModel {
    pub tool: String,
    pub args: String,
    pub desc: String,
    pub selected: usize,
    pub visible: bool,
    pub timeout_secs: Option<u64>,
}

// ─── OverlayViewModel ───────────────────────────────────────────────────────
#[derive(Debug, Clone)]
pub struct OverlayViewModel {
    pub visible: bool,
}

// ─── SessionTreeViewModel ───────────────────────────────────────────────────
#[derive(Debug, Clone)]
pub struct SessionTreeViewModel {
    pub entries: Vec<SessionTreeEntry>,
    pub selected: usize,
    pub visible: bool,
}

// ─── DiffViewerViewModel ───────────────────────────────────────────────────
#[derive(Debug, Clone)]
pub struct DiffViewerViewModel {
    pub filename: String,
    pub diff_lines: Vec<DiffLine>,
    pub scroll_offset: usize,
    pub visible: bool,
}

// ─── OnboardingViewModel ───────────────────────────────────────────────────
#[derive(Debug, Clone)]
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

// ─── CodeBlockViewModel ─────────────────────────────────────────────────────
#[derive(Debug, Clone)]
pub struct CodeBlockViewModel {
    pub lang: Option<String>,
    pub code: String,
    pub filename: Option<String>,
    pub line_status: Vec<LineStatus>,
}

// ─── CollapsibleViewModel ───────────────────────────────────────────────────
#[derive(Debug, Clone)]
pub struct CollapsibleViewModel {
    pub title: String,
    pub expanded: bool,
    pub children: Vec<CollapsibleViewModel>,
}

// ─── ContextPanelViewModel ─────────────────────────────────────────────────
#[derive(Debug, Clone)]
pub struct ContextPanelViewModel {
    pub panels: Vec<ContextPanelItem>,
}

#[derive(Debug, Clone)]
pub struct ContextPanelItem {
    pub title: String,
    pub content: String,
    pub pinned: bool,
}

// ─── ViewModels ─────────────────────────────────────────────────────────────

/// Holds all view-models for a single render pass.
/// Each field is `None` when that panel is hidden, so the renderer
/// can simply `match` or `if let Some(vm) = vm.foo` without any
/// extra booleans.
#[derive(Debug, Default)]
pub struct ViewModels {
    pub top_bar: Option<TopBarViewModel>,
    pub global_tags: Option<GlobalTagsViewModel>,
    pub message_list: Option<MessageListViewModel>,
    pub input_bar: Option<InputBarViewModel>,
    pub status_bar: Option<StatusBarViewModel>,
    pub agent_list: Option<AgentListViewModel>,
    pub command_palette: Option<CommandPaletteViewModel>,
    pub permission_modal: Option<PermissionModalViewModel>,
    pub overlay: Option<OverlayViewModel>,
    pub session_tree: Option<SessionTreeViewModel>,
    pub diff_viewer: Option<DiffViewerViewModel>,
    pub onboarding: Option<OnboardingViewModel>,
    pub code_blocks: Vec<CodeBlockViewModel>,
    pub collapsibles: Vec<CollapsibleViewModel>,
    pub context_panel: Option<ContextPanelViewModel>,
}

/// One-stop shop to rebuild all view-models from current AppState.
/// Call this at the start of each render pass; the functions inside
/// are cheap (mostly struct construction and hashmap lookups).
impl ViewModels {
    pub fn new(state: &crate::tui::state::AppState) -> Self {
        Self {
            top_bar: Some(build_top_bar_vm(state)),
            global_tags: Some(build_global_tags_vm(state)),
            message_list: None, // Built separately with wrap_cache
            input_bar: Some(build_input_bar_vm(state)),
            status_bar: Some(build_status_bar_vm(state)),
            agent_list: Some(build_agent_list_vm(state)),
            command_palette: build_command_palette_vm(state, &state.command_palette),
            permission_modal: build_permission_modal_vm(state),
            overlay: build_overlay_vm(state),
            session_tree: build_session_tree_vm(state),
            diff_viewer: build_diff_viewer_vm(state),
            onboarding: build_onboarding_vm(state),
            code_blocks: Vec::new(),
            collapsibles: Vec::new(),
            context_panel: None,
        }
    }
}

// ─── Thinking Stripping ─────────────────────────────────────────────────────

/// Strips <think>...</think> blocks and thinking bullet markers from
/// assistant text for display when `show_thoughts` is false.
///
/// Returns `Cow::Borrowed(text)` when no thinking markers were found
/// so the common case (no thinking) avoids a per-message allocation.
pub fn strip_thinking_from_assistant(text: &str) -> std::borrow::Cow<'_, str> {
    let mut result: Vec<&str> = Vec::new();
    let mut in_thinking_block = false;
    let mut any_change = false;
    for line in text.lines() {
        let trimmed = line.trim();
        if process_strip_line(line, trimmed, &mut in_thinking_block, &mut any_change) {
            continue;
        }
        result.push(line);
    }
    if !any_change {
        return std::borrow::Cow::Borrowed(text);
    }
    std::borrow::Cow::Owned(result.join("\n"))
}

/// Returns true if the line should be skipped (i.e. is part of the
/// thinking that we're stripping).
fn process_strip_line(
    line: &str,
    trimmed: &str,
    in_thinking_block: &mut bool,
    any_change: &mut bool,
) -> bool {
    // Handle same-line think block: "<think>...</think>" on one line.
    if !*in_thinking_block
        && trimmed.starts_with("<think>")
        && trimmed.ends_with("</think>")
    {
        *any_change = true;
        return true;
    }
    // Opening <think> marker
    if trimmed.starts_with("<think>") {
        *in_thinking_block = true;
        *any_change = true;
        return true;
    }
    // Closing </think> marker (only relevant while in block)
    if *in_thinking_block && trimmed.starts_with("</think>") {
        *in_thinking_block = false;
        *any_change = true;
        return true;
    }
    // Skip lines while inside a multi-line think block
    if *in_thinking_block {
        *any_change = true;
        return true;
    }
    // Skip "thinking bullet" marker lines (· • ◦ ▸ ▹)
    if trimmed
        .chars()
        .next()
        .map_or(false, |c| matches!(c, '·' | '•' | '◦' | '▸' | '▹'))
    {
        *any_change = true;
        return true;
    }
    let _ = line;
    false
}
