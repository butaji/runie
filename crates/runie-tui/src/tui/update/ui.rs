//! UI domain update functions.
//! Handles: mode, overlays, command palette, model picker, sidebar.

use crate::components::{MessageItem, CommandPalette};
use crate::tui::state::{AppState, TuiMode, Cmd};
use crate::components::command_palette::PaletteCommand;
use crate::components::model_picker::ModelPicker;
use crate::components::onboarding::Onboarding;

/// UI-specific commands returned by update functions.
#[derive(Debug, Clone)]
pub enum UiCmd {
    Quit,
}

impl From<UiCmd> for Cmd {
    fn from(cmd: UiCmd) -> Self {
        match cmd {
            UiCmd::Quit => Cmd::Interrupt,
        }
    }
}

/// Update UI domain: mode, overlays, command palette, model picker.
pub fn update(state: &mut AppState, palette: &mut CommandPalette, msg: crate::tui::state::Msg) -> Vec<UiCmd> {
    match msg {
        crate::tui::state::Msg::ToggleSidebar => { state.show_sidebar = !state.show_sidebar; vec![] }
        crate::tui::state::Msg::OpenCommandPalette => { open_palette(state, palette); vec![] }
        crate::tui::state::Msg::CloseModal | crate::tui::state::Msg::ConfirmModal => { handle_close_modal(state); vec![] }
        crate::tui::state::Msg::CommandPaletteCancelArgument => { handle_palette_escape(state, palette); vec![] }
        crate::tui::state::Msg::CommandPaletteFilter(c) => handle_palette_filter(state, palette, c),
        crate::tui::state::Msg::CommandPaletteBackspace => handle_palette_backspace(state, palette),
        crate::tui::state::Msg::CommandPaletteUp => handle_palette_up(state, palette),
        crate::tui::state::Msg::CommandPaletteDown => handle_palette_down(state, palette),
        crate::tui::state::Msg::CommandPaletteConfirm => handle_palette_confirm(state, palette),
        crate::tui::state::Msg::DirectCommand(cmd) => handle_direct_command(state, cmd),
        crate::tui::state::Msg::SelectUp => select_nav(state, true),
        crate::tui::state::Msg::SelectDown => select_nav(state, false),
        crate::tui::state::Msg::SelectConfirm => select_confirm(state),
        crate::tui::state::Msg::SelectToggleDetails => select_toggle_details(state),
        crate::tui::state::Msg::SwitchModel => { handle_switch_model(state); vec![] }
        crate::tui::state::Msg::SlashCommand(cmd) => handle_slash(state, cmd),
        crate::tui::state::Msg::ToggleSessionTree => { super::slash::handle_tree(state); vec![] }
        crate::tui::state::Msg::SessionTreeUp => { state.session_tree.move_up(); vec![] }
        crate::tui::state::Msg::SessionTreeDown => { state.session_tree.move_down(); vec![] }
        crate::tui::state::Msg::SessionTreeConfirm => { super::tree::handle_tree_confirm(state); vec![] }
        crate::tui::state::Msg::SetGitInfo { repo, branch, path } => handle_set_git_info(state, repo, branch, path),
        crate::tui::state::Msg::SetTopBarMockChecks { checks_passed, checks_total, percentage, context_badges } => handle_set_top_bar_mock_checks(state, checks_passed, checks_total, percentage, context_badges),
        crate::tui::state::Msg::SetTopBarRealChecks { context_badges } => handle_set_top_bar_real_checks(state, context_badges),
        crate::tui::state::Msg::SetInputRightInfo(info) => handle_set_input_right_info(state, info),
        crate::tui::state::Msg::EnterOnboarding => handle_enter_onboarding(state),
        crate::tui::state::Msg::CopyLastResponse => { handle_copy_last_response(state); vec![] }
        _ => vec![],
    }
}

// ─── Command Palette Helpers ───────────────────────────────────────────────────

fn handle_palette_filter(state: &mut AppState, palette: &mut CommandPalette, c: char) -> Vec<UiCmd> {
    state.command_palette.filter.push(c);
    palette.filter(&state.command_palette.filter);
    palette.selected = palette.selected.min(palette.filtered_commands.len().saturating_sub(1));
    state.command_palette.selected = palette.selected;
    vec![]
}

fn handle_palette_backspace(state: &mut AppState, palette: &mut CommandPalette) -> Vec<UiCmd> {
    state.command_palette.filter.pop();
    palette.filter(&state.command_palette.filter);
    palette.selected = palette.selected.min(palette.filtered_commands.len().saturating_sub(1));
    state.command_palette.selected = palette.selected;
    vec![]
}

fn handle_palette_up(state: &mut AppState, palette: &mut CommandPalette) -> Vec<UiCmd> {
    palette.selected = palette.selected.saturating_sub(1);
    state.command_palette.selected = palette.selected;
    vec![]
}

fn handle_palette_down(state: &mut AppState, palette: &mut CommandPalette) -> Vec<UiCmd> {
    palette.selected = (palette.selected + 1).min(palette.filtered_commands.len().saturating_sub(1));
    state.command_palette.selected = palette.selected;
    vec![]
}

fn handle_palette_confirm(state: &mut AppState, palette: &mut CommandPalette) -> Vec<UiCmd> {
    if let Some(cmd) = palette.confirm(palette.selected) {
        let cmds = handle_direct_command(state, cmd);
        handle_close_modal(state);
        return cmds;
    }
    vec![]
}

// ─── Select/Model Picker Helpers ─────────────────────────────────────────────

fn select_nav(state: &mut AppState, up: bool) -> Vec<UiCmd> {
    if let Some(ref mut picker) = state.model_picker {
        if up { picker.prev(); } else { picker.next(); }
    }
    vec![]
}

fn select_confirm(state: &mut AppState) -> Vec<UiCmd> {
    if let Some(ref mut picker) = state.model_picker {
        if let Some((_provider_id, model_id)) = picker.selected_model() {
            state.current_model = Some(model_id.to_string());
            state.mode = TuiMode::Chat;
            state.model_picker = None;
        }
    }
    vec![]
}

fn select_toggle_details(state: &mut AppState) -> Vec<UiCmd> {
    if let Some(ref mut picker) = state.model_picker {
        picker.toggle_details();
    }
    vec![]
}

// ─── Command Palette Core ─────────────────────────────────────────────────────

/// Open the command palette.
pub fn open_palette(state: &mut AppState, palette: &mut CommandPalette) {
    state.command_palette.open = true;
    state.mode = TuiMode::CommandPalette;
    state.command_palette.filter.clear();
    state.command_palette.selected = 0;
    palette.selected = 0;
    palette.filter("");
    palette.is_argument_mode = false;
    palette.argument_input.clear();
    palette.pending_command = None;
}

/// Close modal and reset to chat mode.
pub fn handle_close_modal(state: &mut AppState) {
    state.mode = TuiMode::Chat;
    state.command_palette.open = false;
    state.command_palette.filter.clear();
    state.command_palette.selected = 0;
    state.permission_modal.tool = None;
    state.permission_modal.tool_call_id = None;
    state.diff_viewer = None;
    state.session_tree.hide();
    state.model_picker = None;
}

/// Handle Esc in command palette - cancel argument mode or close.
pub fn handle_palette_escape(state: &mut AppState, palette: &mut CommandPalette) {
    if palette.is_argument_mode {
        palette.is_argument_mode = false;
        palette.argument_input.clear();
        palette.pending_command = None;
        palette.filter("");
        palette.selected = 0;
    } else {
        handle_close_modal(state);
    }
}

/// Handle direct command from palette or elsewhere.
pub fn handle_direct_command(state: &mut AppState, cmd: PaletteCommand) -> Vec<UiCmd> {
    match cmd {
        PaletteCommand::NewSession => {
            state.messages.clear();
            state.mode = TuiMode::Chat;
            state.messages.push(MessageItem::System { text: "New session started".to_string() });
            vec![]
        }
        PaletteCommand::ClearChat => {
            state.messages.clear();
            state.messages.push(MessageItem::System { text: "Chat cleared".to_string() });
            vec![]
        }
        PaletteCommand::SwitchModel => { handle_switch_model(state); vec![] }
        PaletteCommand::Quit => {
            state.running = false;
            state.messages.push(MessageItem::System { text: "Goodbye!".to_string() });
            vec![UiCmd::Quit]
        }
        PaletteCommand::Cancel => vec![],
    }
}

/// Switch to model picker overlay.
fn handle_switch_model(state: &mut AppState) {
    let picker = ModelPicker::with_default_models();
    state.model_picker = Some(picker);
    state.mode = TuiMode::Overlay;
}

/// Handle slash commands.
pub fn handle_slash(state: &mut AppState, cmd: runie_core::slash_command::SlashCommand) -> Vec<UiCmd> {
    use runie_core::slash_command::SlashCommand;
    match cmd {
        SlashCommand::New => { state.messages.clear(); state.scroll.feed_offset = 0; state.messages.push(MessageItem::System { text: "New session started".to_string() }); vec![] }
        SlashCommand::Clear => { state.messages.clear(); state.scroll.feed_offset = 0; vec![] }
        SlashCommand::Model(model) => { state.current_model = Some(model.clone()); state.messages.push(MessageItem::System { text: format!("Model switched to {}", model) }); vec![] }
        SlashCommand::Tree => { super::slash::handle_tree(state); vec![] }
        SlashCommand::Fork => { state.messages.push(MessageItem::System { text: "Fork created at current position".to_string() }); vec![] }
        SlashCommand::Copy => { handle_copy_last_response(state); vec![] }
        SlashCommand::Quit => { state.running = false; vec![UiCmd::Quit] }
        SlashCommand::Help => { state.messages.push(MessageItem::System { text: runie_core::slash_command::format_help() }); vec![] }
        SlashCommand::Unknown(cmd) => { state.messages.push(MessageItem::System { text: format!("Unknown command: {}. Type /help for available commands.", cmd) }); vec![] }
        SlashCommand::Cost => { /* Cost command not implemented */ vec![] }
    }
}

// ─── Copy Last Response ────────────────────────────────────────────────────────

/// Copy the last assistant message to clipboard using OSC 52 escape sequence.
fn handle_copy_last_response(state: &mut AppState) -> Vec<UiCmd> {
    // Find last assistant message
    if let Some(last_assistant) = state.messages.iter().rev().find(|m| {
        matches!(m, MessageItem::Assistant { .. })
    }) {
        let text = match last_assistant {
            MessageItem::Assistant { text, .. } => text.clone(),
            _ => String::new(),
        };

        if !text.is_empty() {
            // Use OSC 52 escape sequence for clipboard (works in most modern terminals)
            let encoded_bytes = base64_encode(&text);
            let encoded = std::str::from_utf8(&encoded_bytes).unwrap_or_default();
            let osc52 = format!("\x1b]52;c;{}\x07", encoded);
            print!("{}", osc52);
            state.messages.push(MessageItem::System {
                text: "📋 Copied last response to clipboard".to_string(),
            });
        } else {
            state.messages.push(MessageItem::System {
                text: "No assistant response to copy".to_string(),
            });
        }
    } else {
        state.messages.push(MessageItem::System {
            text: "No assistant response to copy".to_string(),
        });
    }
    vec![]
}

/// Base64 encode a string (minimal implementation avoiding external dependency).
fn base64_encode(input: &str) -> Vec<u8> {
    const BASE64_CHARS: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut result = Vec::with_capacity((input.len() + 2) / 3 * 4);
    
    for chunk in input.as_bytes().chunks(3) {
        let b = match chunk.len() {
            1 => [chunk[0], 0, 0],
            2 => [chunk[0], chunk[1], 0],
            _ => [chunk[0], chunk[1], chunk[2]],
        };
        
        result.push(BASE64_CHARS[(b[0] >> 2) as usize]);
        result.push(BASE64_CHARS[((b[0] & 0x03) << 4 | b[1] >> 4) as usize]);
        
        if chunk.len() > 1 {
            result.push(BASE64_CHARS[((b[1] & 0x0f) << 2 | b[2] >> 6) as usize]);
        } else {
            result.push(b'=');
        }
        
        if chunk.len() > 2 {
            result.push(BASE64_CHARS[(b[2] & 0x3f) as usize]);
        } else {
            result.push(b'=');
        }
    }
    
    result
}

// ─── State Initialization Handlers ─────────────────────────────────────────────

fn handle_set_git_info(state: &mut AppState, repo: String, branch: String, path: String) -> Vec<UiCmd> {
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
) -> Vec<UiCmd> {
    state.top_bar.checks_passed = checks_passed;
    state.top_bar.checks_total = checks_total;
    state.top_bar.percentage = percentage;
    state.top_bar.context_badges = context_badges;
    state.top_bar.context_pct = None;
    state.top_bar.context_bar_pct = None;
    vec![]
}

fn handle_set_top_bar_real_checks(state: &mut AppState, context_badges: Vec<String>) -> Vec<UiCmd> {
    state.top_bar.checks_passed = None;
    state.top_bar.checks_total = None;
    state.top_bar.percentage = None;
    state.top_bar.context_badges = context_badges;
    state.top_bar.context_pct = None;
    state.top_bar.context_bar_pct = None;
    vec![]
}

fn handle_set_input_right_info(state: &mut AppState, info: String) -> Vec<UiCmd> {
    state.input_right_info = info;
    vec![]
}

fn handle_enter_onboarding(state: &mut AppState) -> Vec<UiCmd> {
    state.mode = TuiMode::Onboarding;
    state.onboarding = Some(Onboarding::new());
    vec![]
}
