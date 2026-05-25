pub mod agent;
pub mod misc;
pub mod onboarding;
pub mod palette;
pub mod slash;
pub mod tree;

#[cfg(test)]
mod palette_tests;

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

// P1-REMAINING-1 FIX: Extracted from update() for clear input double-tap confirmation
fn handle_clear_input_confirm(state: &mut AppState) {
    if state.clear_input_confirm.wants_clear() {
        // Second tap within 2 seconds - clear the input
        state.textarea.select_all();
        state.textarea.delete_line_by_end();
        state.input_right_info = String::new();
    } else {
        // First tap - show hint
        state.input_right_info = "Ctrl+C again to clear text".to_string();
    }
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

// P1-3 FIX: Extracted from update() for TextareaKey handling
fn handle_textarea_key(state: &mut AppState, key: crossterm::event::KeyEvent) {
    let input = key_to_textarea_input(key);
    state.textarea.input(input);
}

// P1-3 FIX: Extracted from update() for CommandPaletteConfirm handling
fn handle_palette_confirm(state: &mut AppState, palette: &mut CommandPalette) -> Vec<Cmd> {
    let mut cmds = vec![];
    if let Some(cmd) = palette.confirm(palette.selected) {
        cmds.extend(palette::handle_direct_command(state, cmd));
        palette::handle_close_modal(state);
    }
    cmds
}

// P1-2 FIX: Extracted from update() for Select navigation handling
fn handle_select_nav(state: &mut AppState, msg: &Msg) {
    let picker = match &mut state.model_picker {
        Some(p) => p,
        None => return,
    };
    match msg {
        Msg::SelectUp => {
            picker.prev();
        }
        Msg::SelectDown => {
            picker.next();
        }
        Msg::SelectConfirm => {
            if let Some((_provider_id, model_id)) = picker.selected_model() {
                state.current_model = Some(model_id.to_string());
                state.mode = TuiMode::Chat;
                state.model_picker = None;
            }
        }
        Msg::SelectToggleDetails => {
            picker.toggle_details();
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
        // P1-3 FIX: TextareaKey extracted to handle_textarea_key
        Msg::TextareaKey(key) => { handle_textarea_key(state, key); }
        Msg::ToggleSidebar => { state.show_sidebar = !state.show_sidebar; }
        Msg::OpenCommandPalette => { palette::open_palette(state, palette); }
        Msg::CloseModal | Msg::ConfirmModal => { palette::handle_close_modal(state); }
        Msg::AgentEvent(event) => { agent::handle_agent_event(state, event); }
        // P0-1 FIX: Handle permission timeout
        Msg::PermissionConfirm | Msg::PermissionCancel | Msg::PermissionAlways | Msg::PermissionSkip => { cmds.extend(agent::handle_permission_msg(state, msg)); }
        Msg::PermissionTimeout => { cmds.extend(agent::handle_permission_timeout(state)); }
        // P1-1 FIX: Handle Esc in command palette (cancel argument mode or close)
        Msg::CommandPaletteCancelArgument => { palette::handle_palette_escape(state, palette); }
        Msg::CommandPaletteFilter(_) | Msg::CommandPaletteBackspace | Msg::CommandPaletteUp | Msg::CommandPaletteDown => { handle_palette_msg(state, palette, &msg); }
        // P1-3 FIX: CommandPaletteConfirm extracted to handle_palette_confirm
        Msg::CommandPaletteConfirm => { cmds.extend(handle_palette_confirm(state, palette)); }
        Msg::ScrollUp | Msg::ScrollPageUp => { state.scroll.feed_offset = state.scroll.feed_offset.saturating_sub(if matches!(msg, Msg::ScrollPageUp) { 10 } else { 1 }); }
        Msg::ScrollDown | Msg::ScrollPageDown => { state.scroll.feed_offset = (state.scroll.feed_offset + if matches!(msg, Msg::ScrollPageDown) { 10 } else { 1 }).min(state.messages.len().saturating_sub(1)); }
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
        // P1-REMAINING-1 FIX: Double-tap Ctrl+C to clear text (prevents accidental loss)
        Msg::ClearInputConfirm => { handle_clear_input_confirm(state); }
        Msg::ClearInput => { state.textarea.select_all(); state.textarea.delete_line_by_end(); }
        Msg::ClearChat => { state.messages.clear(); }
        Msg::DirectCommand(cmd) => { cmds.extend(palette::handle_direct_command(state, cmd)); }
        Msg::Resize(w, h) => { state.terminal_size = (w, h); }
        // P1-2 FIX: Select/Overlay navigation (model picker)
        Msg::SelectUp | Msg::SelectDown | Msg::SelectConfirm | Msg::SelectToggleDetails => { handle_select_nav(state, &msg); }
        Msg::SwitchModel => { palette::handle_switch_model(state); }
    }

    cmds
}
