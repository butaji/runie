//! UI domain update functions.
//! Handles: mode, overlays, command palette, model picker, sidebar.

pub mod navigation;
pub mod palette_helpers;
pub mod clipboard;

use crate::components::CommandPalette;
use crate::tui::state::{AppState, Cmd};

pub use navigation::{handle_select, handle_session_tree, handle_context, handle_model_mode, handle_set_git_info, handle_enter_onboarding};
pub use palette_helpers::handle_palette;
pub use clipboard::handle_copy_last_response;

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
    use crate::tui::state::Msg;

    // Palette/Modal
    if matches!(msg, Msg::CloseModal) {
        super::palette::handle_close_modal(state);
        return vec![];
    }
    if matches!(msg, Msg::ConfirmModal) { return vec![]; }
    if matches!(msg, Msg::OpenCommandPalette | Msg::CommandPaletteUp | Msg::CommandPaletteDown |
        Msg::CommandPaletteConfirm | Msg::CommandPaletteBackspace | Msg::CommandPaletteCancelArgument |
        Msg::CommandPaletteFilter(_))
    {
        return handle_palette(state, palette, &msg);
    }

    // Navigation
    if matches!(msg, Msg::SelectUp | Msg::SelectDown | Msg::SelectConfirm | Msg::SelectToggleDetails |
        Msg::SwitchModel | Msg::ToggleSessionTree | Msg::SessionTreeUp | Msg::SessionTreeDown |
        Msg::SessionTreeConfirm)
    {
        return handle_nav(&msg, state);
    }

    // State handlers
    return handle_state_msg(state, palette, msg);
}

fn is_context_msg(msg: &crate::tui::state::Msg) -> bool {
    use crate::tui::state::Msg;
    matches!(msg, Msg::SetTopBarMockChecks { .. } | Msg::SetTopBarRealChecks { .. } |
        Msg::SetInputRightInfo(_))
}

fn is_model_mode_msg(msg: &crate::tui::state::Msg) -> bool {
    use crate::tui::state::Msg;
    matches!(msg, Msg::SetCurrentModel(_) | Msg::SetMockMode(_) | Msg::ResetAgentState)
}

fn handle_state_match(state: &mut AppState, msg: crate::tui::state::Msg) -> Vec<UiCmd> {
    use crate::tui::state::Msg;
    match msg {
        Msg::DirectCommand(cmd) => super::palette::handle_direct_command(state, cmd),
        Msg::ToggleSidebar => handle_toggle_sidebar(state),
        Msg::SetGitInfo { repo, branch, path } =>
            handle_set_git_info(state, repo.clone(), branch.clone(), path.clone()),
        Msg::EnterOnboarding => handle_enter_onboarding(state),
        Msg::SlashCommand(cmd) => super::slash::handle_slash(state, cmd),
        Msg::CopyLastResponse => { handle_copy_last_response(state); vec![] }
        _ => vec![],
    }
}

fn handle_state_msg(state: &mut AppState, _palette: &mut CommandPalette, msg: crate::tui::state::Msg) -> Vec<UiCmd> {
    if is_context_msg(&msg) {
        return handle_context(&msg, state);
    }
    if is_model_mode_msg(&msg) {
        return handle_model_mode(&msg, state);
    }

    handle_state_match(state, msg)
}

/// Combined navigation handler for select and session tree.
fn handle_nav(msg: &crate::tui::state::Msg, state: &mut AppState) -> Vec<UiCmd> {
    use crate::tui::state::Msg;
    match msg {
        Msg::SelectUp | Msg::SelectDown | Msg::SelectConfirm | Msg::SelectToggleDetails | Msg::SwitchModel =>
            handle_select(msg, state),
        Msg::ToggleSessionTree | Msg::SessionTreeUp | Msg::SessionTreeDown | Msg::SessionTreeConfirm =>
            handle_session_tree(msg, state),
        _ => vec![],
    }
}

fn handle_toggle_sidebar(state: &mut AppState) -> Vec<UiCmd> {
    state.show_sidebar = !state.show_sidebar;
    vec![]
}
