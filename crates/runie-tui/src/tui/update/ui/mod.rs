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

    // Overlay modals (Extensions, Plan)
    if let Some(result) = handle_overlay_modals(state, &msg) {
        return result;
    }

    // Questionnaire
    if let Some(result) = handle_questionnaire_update(state, &msg) {
        return result;
    }

    // State handlers
    return handle_state_msg(state, palette, msg);
}

fn handle_overlay_modals(state: &mut AppState, msg: &crate::tui::state::Msg) -> Option<Vec<UiCmd>> {
    use crate::tui::state::Msg;

    // Extensions Modal
    if matches!(msg, Msg::OpenExtensionsModal | Msg::CloseExtensionsModal |
        Msg::ExtensionsModalUp | Msg::ExtensionsModalDown | Msg::ExtensionsModalSelect |
        Msg::ExtensionsModalLeft | Msg::ExtensionsModalRight |
        Msg::ExtensionsModalSearchInput(_) | Msg::ExtensionsModalSearchBackspace)
    {
        return Some(handle_extensions_modal(state, msg));
    }

    // Plan Modal
    if matches!(msg, Msg::PlanModeApprove | Msg::PlanModeDeny |
        Msg::PlanModeViewNext | Msg::PlanModeViewPrev)
    {
        return Some(handle_plan_modal_msg(state, msg));
    }

    None
}

fn handle_questionnaire_update(state: &mut AppState, msg: &crate::tui::state::Msg) -> Option<Vec<UiCmd>> {
    use crate::tui::state::Msg;
    use crate::tui::TuiMode;

    if matches!(msg, Msg::ToggleQuestionnaire) {
        return Some(handle_toggle_questionnaire(state));
    }
    if matches!(state.mode, TuiMode::Questionnaire) {
        if matches!(msg, Msg::QuestionnaireUp | Msg::QuestionnaireDown |
            Msg::QuestionnairePrevQuestion | Msg::QuestionnaireNextQuestion |
            Msg::QuestionnaireSelect | Msg::QuestionnaireToggleCustom |
            Msg::CloseQuestionnaire)
        {
            return Some(handle_questionnaire_msg(state, msg));
        }
    }
    None
}

fn is_context_msg(msg: &crate::tui::state::Msg) -> bool {
    use crate::tui::state::Msg;
    matches!(msg, Msg::SetTopBarMockChecks { .. } | Msg::SetTopBarRealChecks { .. } |
        Msg::SetInputRightInfo(_))
}

fn is_model_mode_msg(msg: &crate::tui::state::Msg) -> bool {
    use crate::tui::state::Msg;
    matches!(msg, Msg::SetCurrentModel(_) | Msg::SetMockMode(_) | Msg::SetPermissionMode(_) | Msg::ResetAgentState)
}

fn handle_state_match(state: &mut AppState, msg: crate::tui::state::Msg) -> Vec<UiCmd> {
    use crate::tui::state::Msg;
    match msg {
        Msg::DirectCommand(cmd) => super::palette::handle_direct_command(state, cmd),
        Msg::SetGitInfo { repo, branch, path } => handle_git_info(state, repo, branch, path),
        Msg::EnterOnboarding => handle_enter_onboarding(state),
        Msg::SlashCommand(cmd) => super::slash::handle_slash(state, cmd),
        Msg::UpdateTopBarContext { model, context_window, estimated_tokens } => handle_top_bar_context(state, model, context_window, estimated_tokens),
        Msg::ToggleSidebar | Msg::ToggleThoughts | Msg::ToggleSubagentPanel | Msg::TogglePromptQueue | Msg::ToggleWorktreeMode => toggle_state(state, msg),
        Msg::NewSessionWorktree | Msg::ImportClaudeSettings | Msg::CopyLastResponse | Msg::ShowHelp => handle_misc(state, msg),
        _ => vec![],
    }
}

fn handle_misc(state: &mut AppState, msg: crate::tui::state::Msg) -> Vec<UiCmd> {
    use crate::tui::state::Msg;
    match msg {
        Msg::NewSessionWorktree => new_session_worktree(state),
        Msg::ImportClaudeSettings => import_claude_settings(state),
        Msg::CopyLastResponse => copy_last(state),
        Msg::ShowHelp => show_help(state),
        _ => vec![],
    }
}

fn toggle_state(state: &mut AppState, msg: crate::tui::state::Msg) -> Vec<UiCmd> {
    use crate::tui::state::Msg;
    match msg {
        Msg::ToggleSidebar => { state.show_sidebar = !state.show_sidebar; }
        Msg::ToggleThoughts => { state.show_thoughts = !state.show_thoughts; }
        Msg::ToggleSubagentPanel => { state.subagent_panel.toggle(); }
        Msg::TogglePromptQueue => { state.input_right_info = "Prompt queue toggled".to_string(); }
        Msg::ToggleWorktreeMode => { state.input_right_info = "Worktree mode toggled".to_string(); }
        _ => {}
    }
    vec![]
}

fn handle_git_info(state: &mut AppState, repo: String, branch: String, path: String) -> Vec<UiCmd> { handle_set_git_info(state, repo, branch, path) }
fn handle_top_bar_context(state: &mut AppState, model: String, context_window: Option<usize>, estimated_tokens: Option<usize>) -> Vec<UiCmd> { handle_update_top_bar_context(state, Some(model), context_window, estimated_tokens) }


fn copy_last(state: &mut AppState) -> Vec<UiCmd> { handle_copy_last_response(state); vec![] }
fn show_help(state: &mut AppState) -> Vec<UiCmd> { super::slash::handle_help(state); vec![] }
fn toggle_subagent(state: &mut AppState) -> Vec<UiCmd> { state.subagent_panel.toggle(); vec![] }
fn toggle_prompt_queue(state: &mut AppState) -> Vec<UiCmd> { handle_toggle_prompt_queue(state); vec![] }
fn toggle_worktree_mode(state: &mut AppState) -> Vec<UiCmd> { handle_toggle_worktree_mode(state); vec![] }
fn new_session_worktree(state: &mut AppState) -> Vec<UiCmd> { handle_new_session_worktree(state); vec![] }
fn import_claude_settings(state: &mut AppState) -> Vec<UiCmd> { handle_import_claude_settings(state); vec![] }

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
    if matches!(msg, Msg::OpenExtensionsModal) {
        return ext_open();
    }
    let Some(modal) = state.extensions_modal.as_mut() else {
        return vec![];
    };
    if matches!(msg, Msg::CloseExtensionsModal) {
        return ext_close(state);
    }
    if matches!(msg, Msg::ExtensionsModalUp | Msg::ExtensionsModalDown) { return ext_nav(modal, msg); }
    if matches!(msg, Msg::ExtensionsModalSelect) { return ext_select(); }
    if matches!(msg, Msg::ExtensionsModalLeft | Msg::ExtensionsModalRight) { return ext_tab(modal, msg); }
    if let Msg::ExtensionsModalSearchInput(c) = msg { return ext_search_push(modal, *c); }
    if matches!(msg, Msg::ExtensionsModalSearchBackspace) { return ext_search_pop(modal); }
    vec![]
}

fn ext_open() -> Vec<UiCmd> { vec![] }
fn ext_select() -> Vec<UiCmd> { vec![] }

fn ext_close(state: &mut AppState) -> Vec<UiCmd> {
    state.extensions_modal = None;
    state.mode = crate::tui::TuiMode::Chat;
    vec![]
}
fn ext_nav(m: &mut crate::components::extensions_modal::ExtensionsModal, msg: &crate::tui::state::Msg) -> Vec<UiCmd> {
    use crate::tui::state::Msg;
    match msg {
        Msg::ExtensionsModalUp => m.move_up(),
        Msg::ExtensionsModalDown => m.move_down(),
        _ => {}
    }
    vec![]
}
fn ext_tab(m: &mut crate::components::extensions_modal::ExtensionsModal, msg: &crate::tui::state::Msg) -> Vec<UiCmd> {
    use crate::tui::state::Msg;
    use crate::components::extensions_modal::ExtensionTab;
    let tabs = ExtensionTab::all();
    let idx = tabs.iter().position(|t| *t == m.active_tab);
    match msg {
        Msg::ExtensionsModalLeft if idx.is_some_and(|i| i > 0) => {
            if let Some(i) = idx {
                m.set_tab(tabs[i - 1]);
            }
        }
        Msg::ExtensionsModalRight if idx.is_some_and(|i| i < tabs.len() - 1) => {
            if let Some(i) = idx {
                m.set_tab(tabs[i + 1]);
            }
        }
        _ => {}
    }
    vec![]
}
fn ext_search_push(m: &mut crate::components::extensions_modal::ExtensionsModal, c: char) -> Vec<UiCmd> {
    m.search_query.push(c);
    vec![]
}
fn ext_search_pop(m: &mut crate::components::extensions_modal::ExtensionsModal) -> Vec<UiCmd> {
    m.search_query.pop();
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

fn handle_toggle_prompt_queue(state: &mut AppState) {
    // Toggle prompt queue pane visibility
    state.input_right_info = "Prompt queue toggled".to_string();
}

fn handle_new_session_worktree(state: &mut AppState) {
    // Create new session in worktree mode
    state.messages.clear();
    state.messages.push(crate::components::MessageItem::System {
        text: "New worktree session started".to_string(),
    });
    state.input_right_info = "New worktree session".to_string();
}

fn handle_toggle_worktree_mode(state: &mut AppState) {
    state.input_right_info = "Worktree mode toggled".to_string();
}

fn handle_import_claude_settings(state: &mut AppState) {
    state.input_right_info = "Importing Claude settings...".to_string();
    state.messages.push(crate::components::MessageItem::System {
        text: "Claude settings import initiated".to_string(),
    });
}

fn handle_questionnaire_msg(state: &mut AppState, msg: &crate::tui::state::Msg) -> Vec<UiCmd> {
    let Some(q) = state.questionnaire.as_mut() else { return vec![] };
    use crate::tui::state::Msg;
    if matches!(msg, Msg::QuestionnaireUp | Msg::QuestionnaireDown) {
        return qa_option_nav(q, msg);
    }
    if matches!(msg, Msg::QuestionnairePrevQuestion | Msg::QuestionnaireNextQuestion) {
        return qa_question_nav(q, msg);
    }
    match msg {
        Msg::QuestionnaireSelect => { qa_select(q); vec![] }
        Msg::QuestionnaireToggleCustom => { qa_toggle_custom(q); vec![] }
        Msg::CloseQuestionnaire => {
            q.visible = false;
            state.mode = crate::tui::TuiMode::Chat;
            vec![]
        }
        _ => vec![],
    }
}

fn qa_option_nav(q: &mut crate::components::questionnaire_panel::QuestionnaireState, msg: &crate::tui::state::Msg) -> Vec<UiCmd> {
    use crate::tui::state::Msg;
    match msg {
        Msg::QuestionnaireUp => q.prev_option(),
        Msg::QuestionnaireDown => q.next_option(),
        _ => {}
    }
    vec![]
}
fn qa_question_nav(q: &mut crate::components::questionnaire_panel::QuestionnaireState, msg: &crate::tui::state::Msg) -> Vec<UiCmd> {
    use crate::tui::state::Msg;
    match msg {
        Msg::QuestionnairePrevQuestion => q.prev_question(),
        Msg::QuestionnaireNextQuestion => q.next_question(),
        _ => {}
    }
    vec![]
}
fn qa_select(q: &mut crate::components::questionnaire_panel::QuestionnaireState) { q.select_current(); }
fn qa_toggle_custom(q: &mut crate::components::questionnaire_panel::QuestionnaireState) { q.toggle_custom(); }

fn handle_toggle_questionnaire(state: &mut AppState) -> Vec<UiCmd> {
    if let Some(ref mut q) = state.questionnaire {
        q.toggle();
        if q.visible {
            state.mode = crate::tui::TuiMode::Questionnaire;
        } else {
            state.mode = crate::tui::TuiMode::Chat;
        }
    } else {
        // Initialize with default questionnaire if none exists
        state.questionnaire = Some(crate::components::questionnaire_panel::QuestionnaireState::default());
        if let Some(ref mut q) = state.questionnaire {
            q.visible = true;
        }
        state.mode = crate::tui::TuiMode::Questionnaire;
    }
    vec![]
}
