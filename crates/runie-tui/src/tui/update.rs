pub mod agent;
pub mod misc;
pub mod onboarding;
pub mod palette;
pub mod slash;
pub mod tree;

use crate::tui::state::{AppState, Msg, Cmd};
use crate::tui::key_to_textarea_input;

pub fn update(state: &mut AppState, msg: Msg) -> Vec<Cmd> {
    let mut cmds = vec![];

    match msg {
        Msg::Quit => { state.running = false; }
        Msg::Submit => { cmds.extend(misc::handle_submit(state)); }
        // TextareaKey is handled here AND in handle_key (for handle_event path)
        // This ensures tui_run.rs which calls update() directly still gets textarea input
        Msg::TextareaKey(key) => {
            let input = key_to_textarea_input(key);
            state.textarea.input(input);
        }
        Msg::ToggleSidebar => { state.show_sidebar = !state.show_sidebar; }
        Msg::OpenCommandPalette => { palette::open_palette(state); }
        Msg::CloseModal | Msg::ConfirmModal => { palette::handle_close_modal(state); }
        Msg::AgentEvent(event) => { agent::handle_agent_event(state, event); }
        Msg::PermissionConfirm | Msg::PermissionCancel | Msg::PermissionAlways | Msg::PermissionSkip => { cmds.push(agent::handle_permission_msg(state, msg)); }
        Msg::CommandPaletteFilter(c) => { state.command_palette.filter.push(c); }
        Msg::CommandPaletteBackspace => { state.command_palette.filter.pop(); }
        Msg::CommandPaletteUp | Msg::CommandPaletteDown | Msg::CommandPaletteConfirm => { palette::handle_palette_msg(state, msg); }
        Msg::ScrollUp | Msg::ScrollPageUp => { state.scroll.feed_offset = state.scroll.feed_offset.saturating_sub(if matches!(msg, Msg::ScrollPageUp) { 10 } else { 1 }); }
        Msg::ScrollDown | Msg::ScrollPageDown => { state.scroll.feed_offset = (state.scroll.feed_offset + if matches!(msg, Msg::ScrollPageDown) { 10 } else { 1 }).min(state.messages.len().saturating_sub(1)); }
        Msg::Tick | Msg::CursorBlink => { misc::handle_anim(state, &msg); }
        Msg::SlashCommand(cmd) => { cmds.extend(slash::handle_slash(state, cmd)); }
        Msg::ToggleSessionTree => { slash::handle_tree(state); }
        Msg::SessionTreeUp | Msg::SessionTreeDown => { tree::handle_tree_nav(state, &msg); }
        Msg::SessionTreeConfirm => { tree::handle_tree_confirm(state); }
        Msg::OnboardingNext | Msg::OnboardingBack | Msg::OnboardingNavigateUp |
        Msg::OnboardingNavigateDown | Msg::OnboardingSelectProvider(_) |
        Msg::OnboardingSelectModel(_) | Msg::OnboardingKeyInput(_) | Msg::OnboardingKeyBackspace |
        Msg::OnboardingSubmit | Msg::OnboardingSkip => { cmds.extend(onboarding::handle_onboarding_msg(state, msg)); }
        Msg::InsertNewline => { state.textarea.insert_newline(); }
        Msg::ClearInput => { state.textarea.select_all(); state.textarea.delete_line_by_end(); }
    }

    cmds
}
