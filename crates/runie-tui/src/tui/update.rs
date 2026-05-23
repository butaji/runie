pub mod agent;
pub mod cursor;
pub mod input;
pub mod misc;
pub mod onboarding;
pub mod palette;
pub mod slash;
pub mod tree;

use crate::components::MessageItem;
use crate::tui::state::{AppState, TuiMode, Msg, Cmd, OnboardingStep};

pub fn update(state: &mut AppState, msg: Msg) -> Vec<Cmd> {
    let mut cmds = vec![];

    match msg {
        Msg::Quit => { state.running = false; }
        Msg::Submit => { cmds.extend(misc::handle_submit(state)); }
        Msg::InsertChar(c) => { input::handle_insert_char(state, c); }
        Msg::Backspace => { input::handle_backspace(state); }
        Msg::InsertNewline => { input::handle_insert_newline(state); }
        Msg::MoveCursorLeft | Msg::MoveCursorRight | Msg::MoveCursorUp | Msg::MoveCursorDown => { cursor::handle_cursor_move(state, &msg); }
        Msg::MoveCursorToStart | Msg::MoveCursorToEnd => { cursor::handle_cursor_edge(state, &msg); }
        Msg::DeleteForward => { input::handle_delete_forward(state); }
        Msg::DeleteWordBackward => { input::handle_delete_word_backward(state); }
        Msg::DeleteToStart => { input::handle_delete_to_start(state); }
        Msg::ToggleSidebar => { state.show_sidebar = !state.show_sidebar; }
        Msg::OpenCommandPalette => { palette::open_palette(state); }
        Msg::CloseModal | Msg::ConfirmModal => { palette::handle_close_modal(state); }
        Msg::AgentEvent(event) => { agent::handle_agent_event(state, event); }
        Msg::PermissionConfirm | Msg::PermissionCancel | Msg::PermissionAlways | Msg::PermissionSkip => { cmds.push(agent::handle_permission_msg(state, msg)); }
        Msg::CommandPaletteFilter(c) => { state.command_palette.filter.push(c); }
        Msg::CommandPaletteBackspace => { state.command_palette.filter.pop(); }
        Msg::CommandPaletteUp | Msg::CommandPaletteDown | Msg::CommandPaletteConfirm => { palette::handle_palette_msg(state, msg); }
        Msg::ScrollUp | Msg::ScrollPageUp => { state.scroll.feed_offset = state.scroll.feed_offset.saturating_sub(if matches!(msg, Msg::ScrollPageUp) { 10 } else { 1 }); }
        Msg::ScrollDown | Msg::ScrollPageDown => { misc::handle_scroll(state, if matches!(msg, Msg::ScrollPageDown) { 10 } else { 1 }); }
        Msg::Tick | Msg::CursorBlink => { misc::handle_anim(state, &msg); }
        Msg::SlashCommand(cmd) => { cmds.extend(slash::handle_slash(state, cmd)); }
        Msg::ToggleSessionTree => { slash::handle_tree(state); }
        Msg::SessionTreeUp | Msg::SessionTreeDown => { tree::handle_tree_nav(state, &msg); }
        Msg::SessionTreeConfirm => { tree::handle_tree_confirm(state); }
        Msg::OnboardingNext | Msg::OnboardingBack | Msg::OnboardingNavigateUp |
        Msg::OnboardingNavigateDown | Msg::OnboardingSelectProvider(_) |
        Msg::OnboardingSelectModel(_) | Msg::OnboardingKeyInput(_) | Msg::OnboardingKeyBackspace |
        Msg::OnboardingSubmit | Msg::OnboardingSkip => { cmds.extend(onboarding::handle_onboarding_msg(state, msg)); }
    }

    cmds
}
