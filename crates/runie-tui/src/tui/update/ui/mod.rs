//! UI domain update functions.
//! Handles: mode, overlays, command palette, model picker, sidebar.

pub mod navigation;
pub mod palette_helpers;
pub mod clipboard;

use crate::components::CommandPalette;
use crate::tui::state::{AppState, Cmd};

pub use navigation::{handle_select, handle_session_tree, handle_context, handle_model_mode, handle_set_git_info, handle_enter_onboarding, handle_update_top_bar_context};
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

    // Extensions Modal
    if matches!(msg, Msg::OpenExtensionsModal | Msg::CloseExtensionsModal |
        Msg::ExtensionsModalUp | Msg::ExtensionsModalDown | Msg::ExtensionsModalSelect |
        Msg::ExtensionsModalLeft | Msg::ExtensionsModalRight |
        Msg::ExtensionsModalSearchInput(_) | Msg::ExtensionsModalSearchBackspace)
    {
        return handle_extensions_modal(state, &msg);
    }

    // Plan Modal
    if matches!(msg, Msg::PlanModeApprove | Msg::PlanModeDeny |
        Msg::PlanModeViewNext | Msg::PlanModeViewPrev)
    {
        return handle_plan_modal_msg(state, &msg);
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
        Msg::ToggleThoughts => handle_toggle_thoughts(state),
        Msg::SetGitInfo { repo, branch, path } =>
            handle_set_git_info(state, repo.clone(), branch.clone(), path.clone()),
        Msg::EnterOnboarding => handle_enter_onboarding(state),
        Msg::SlashCommand(cmd) => super::slash::handle_slash(state, cmd),
        Msg::CopyLastResponse => { handle_copy_last_response(state); vec![] }
        Msg::ShowHelp => { super::slash::handle_help(state); vec![] }
        Msg::UpdateTopBarContext { model, context_window, estimated_tokens } =>
            handle_update_top_bar_context(state, Some(model), context_window, estimated_tokens),
        Msg::ToggleSubagentPanel => { state.subagent_panel.toggle(); vec![] }
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

fn handle_toggle_thoughts(state: &mut AppState) -> Vec<UiCmd> {
    state.show_thoughts = !state.show_thoughts;
    vec![]
}

fn handle_extensions_modal(state: &mut AppState, msg: &crate::tui::state::Msg) -> Vec<UiCmd> {
    use crate::tui::state::Msg;
    use crate::components::extensions_modal::ExtensionTab;

    let modal = match &mut state.extensions_modal {
        Some(m) => m,
        None => return vec![],
    };

    match msg {
        Msg::OpenExtensionsModal => {
            // Already open
        }
        Msg::CloseExtensionsModal => {
            state.extensions_modal = None;
            state.mode = crate::tui::TuiMode::Chat;
        }
        Msg::ExtensionsModalUp => {
            modal.move_up();
        }
        Msg::ExtensionsModalDown => {
            modal.move_down();
        }
        Msg::ExtensionsModalSelect => {
            // Handle selection (install/update action)
        }
        Msg::ExtensionsModalLeft => {
            // Switch to previous tab
            let tabs = ExtensionTab::all();
            if let Some(current_idx) = tabs.iter().position(|t| *t == modal.active_tab) {
                if current_idx > 0 {
                    modal.set_tab(tabs[current_idx - 1]);
                }
            }
        }
        Msg::ExtensionsModalRight => {
            // Switch to next tab
            let tabs = ExtensionTab::all();
            if let Some(current_idx) = tabs.iter().position(|t| *t == modal.active_tab) {
                if current_idx < tabs.len() - 1 {
                    modal.set_tab(tabs[current_idx + 1]);
                }
            }
        }
        Msg::ExtensionsModalSearchInput(c) => {
            modal.search_query.push(*c);
        }
        Msg::ExtensionsModalSearchBackspace => {
            modal.search_query.pop();
        }
        _ => {}
    }
    vec![]
}

fn handle_plan_modal_msg(state: &mut AppState, msg: &crate::tui::state::Msg) -> Vec<UiCmd> {
    use crate::tui::state::Msg;
    use crate::tui::state::PermissionMode;

    match msg {
        Msg::PlanModeApprove => {
            // User approved the plan - close modal and switch to normal mode
            state.plan_modal.close();
            state.mode = crate::tui::TuiMode::Chat;
            state.permission_mode = PermissionMode::Normal;
            state.messages.push(crate::components::MessageItem::System {
                text: "Plan approved".to_string(),
            });
            vec![]
        }
        Msg::PlanModeDeny => {
            // User denied the plan - close modal without applying
            state.plan_modal.close();
            state.mode = crate::tui::TuiMode::Chat;
            state.messages.push(crate::components::MessageItem::System {
                text: "Plan denied".to_string(),
            });
            vec![]
        }
        Msg::PlanModeViewPrev => {
            // Scroll up in the plan
            state.plan_modal.scroll_up();
            vec![]
        }
        Msg::PlanModeViewNext => {
            // Scroll down in the plan
            state.plan_modal.scroll_down();
            vec![]
        }
        _ => vec![],
    }
}
