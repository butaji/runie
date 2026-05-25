pub mod agent;
pub mod misc;
pub mod onboarding;
pub mod palette;
pub mod slash;
pub mod tree;

use crate::tui::state::{AppState, Msg, Cmd, TuiMode};
use crate::tui::key_to_textarea_input;
use crate::components::CommandPalette;

// P0-1 FIX: Extracted from update() to reduce function size below 40-line build limit
fn handle_quit_or_stop(state: &mut AppState, msg: &Msg) -> Vec<Cmd> {
    state.agent_running = false;
    if matches!(msg, Msg::Quit) {
        state.running = false;
    }
    // BG-2 FIX: Always reset mode to Chat on interrupt/stop
    if state.mode != TuiMode::Onboarding {
        state.mode = TuiMode::Chat;
    }
    if matches!(msg, Msg::Stop) {
        vec![Cmd::Interrupt]
    } else {
        vec![]
    }
}

// P0-1 FIX: Extracted from update() for permission timeout check
fn handle_tick_permission_check(state: &mut AppState, palette: &mut CommandPalette) -> Vec<Cmd> {
    let mut cmds = vec![];
    if let Some(timeout_msg) = misc::check_permission_timeout(state) {
        cmds.extend(update(state, palette, timeout_msg));
    }
    cmds
}


fn route_onboarding(state: &mut AppState, msg: Msg) -> Vec<Cmd> {
    match msg {
        Msg::Paste(text) => {
            let mut cmds = vec![];
            for c in text.chars() {
                cmds.extend(onboarding::handle_onboarding_msg(state, Msg::OnboardingKeyInput(c)));
            }
            cmds
        }
        _ => onboarding::handle_onboarding_msg(state, msg),
    }
}

fn handle_palette_msg(state: &mut AppState, palette: &mut CommandPalette, msg: &Msg) {
    match msg {
        Msg::CommandPaletteFilter(c) => {
            state.command_palette.filter.push(*c);
            palette.filter(&state.command_palette.filter);
            palette.selected = palette.selected.min(palette.filtered_commands.len().saturating_sub(1));
            state.command_palette.selected = palette.selected;
        }
        Msg::CommandPaletteBackspace => {
            state.command_palette.filter.pop();
            palette.filter(&state.command_palette.filter);
            palette.selected = palette.selected.min(palette.filtered_commands.len().saturating_sub(1));
            state.command_palette.selected = palette.selected;
        }
        Msg::CommandPaletteUp => {
            palette.selected = palette.selected.saturating_sub(1);
            state.command_palette.selected = palette.selected;
        }
        Msg::CommandPaletteDown => {
            palette.selected = (palette.selected + 1).min(palette.filtered_commands.len().saturating_sub(1));
            state.command_palette.selected = palette.selected;
        }
        _ => {}
    }
}

pub fn update(state: &mut AppState, palette: &mut CommandPalette, msg: Msg) -> Vec<Cmd> {
    let mut cmds = vec![];

    match msg {
        // P0-1 FIX: Handle Ctrl+C / Stop — interrupt agent and return to Chat mode
        Msg::Quit | Msg::Stop => { cmds.extend(handle_quit_or_stop(state, &msg)); }
        Msg::Submit => { cmds.extend(misc::handle_submit(state)); }
        // TextareaKey is handled here AND in handle_key (for handle_event path)
        // This ensures tui_run.rs which calls update() directly still gets textarea input
        Msg::TextareaKey(key) => {
            let input = key_to_textarea_input(key);
            state.textarea.input(input);
        }
        Msg::ToggleSidebar => { state.show_sidebar = !state.show_sidebar; }
        Msg::OpenCommandPalette => { palette::open_palette(state, palette); }
        Msg::CloseModal | Msg::ConfirmModal => { palette::handle_close_modal(state); }
        Msg::AgentEvent(event) => { agent::handle_agent_event(state, event); }
        // P0-1 FIX: Handle permission timeout
        Msg::PermissionConfirm | Msg::PermissionCancel | Msg::PermissionAlways | Msg::PermissionSkip => { cmds.extend(agent::handle_permission_msg(state, msg)); }
        Msg::PermissionTimeout => { cmds.extend(agent::handle_permission_timeout(state)); }
        Msg::CommandPaletteFilter(_) | Msg::CommandPaletteBackspace | Msg::CommandPaletteUp | Msg::CommandPaletteDown | Msg::CommandPaletteConfirm => { handle_palette_msg(state, palette, &msg); }
        Msg::ScrollUp | Msg::ScrollPageUp => { state.scroll.feed_offset = state.scroll.feed_offset.saturating_sub(if matches!(msg, Msg::ScrollPageUp) { 10 } else { 1 }); },
        Msg::ScrollDown | Msg::ScrollPageDown => { state.scroll.feed_offset = (state.scroll.feed_offset + if matches!(msg, Msg::ScrollPageDown) { 10 } else { 1 }).min(state.messages.len().saturating_sub(1)); },
        Msg::Tick | Msg::CursorBlink => { 
            misc::handle_anim(state, &msg);
            // P0-1 FIX: Check for permission timeout
            cmds.extend(handle_tick_permission_check(state, palette));
        }
        Msg::SlashCommand(cmd) => { cmds.extend(slash::handle_slash(state, cmd)); }
        Msg::ToggleSessionTree => { slash::handle_tree(state); }
        Msg::SessionTreeUp | Msg::SessionTreeDown => { tree::handle_tree_nav(state, &msg); }
        Msg::SessionTreeConfirm => { tree::handle_tree_confirm(state); }
        Msg::OnboardingNext | Msg::OnboardingBack | Msg::OnboardingNavigateUp | Msg::OnboardingNavigateDown | Msg::OnboardingSelectProvider(_) | Msg::OnboardingSelectModel(_) | Msg::OnboardingKeyInput(_) | Msg::OnboardingKeyBackspace | Msg::OnboardingSearchInput(_) | Msg::OnboardingSearchBackspace | Msg::OnboardingSubmit | Msg::OnboardingSkip | Msg::ModelsFetched(_) | Msg::ModelsFetchFailed(_) | Msg::Paste(_) => cmds.extend(route_onboarding(state, msg)),
        Msg::InsertNewline => { state.textarea.insert_newline(); }
        Msg::ClearInput => { state.textarea.select_all(); state.textarea.delete_line_by_end(); }
        Msg::ClearChat => { state.messages.clear(); }
        Msg::DirectCommand(cmd) => { cmds.extend(palette::handle_direct_command(state, cmd)); }
        Msg::Resize(w, h) => {
            state.terminal_size = (w, h);
        }
    }

    cmds
}
