use crate::tui::state::{AppState, TuiMode, Cmd};
use crate::components::command_palette::PaletteCommand;
use crate::components::CommandPalette;

pub fn open_palette(state: &mut AppState, palette: &mut CommandPalette) {
    state.command_palette.open = true;
    state.mode = TuiMode::CommandPalette;
    state.command_palette.filter.clear();
    state.command_palette.selected = 0;
    palette.selected = 0;
    palette.filter("");
    // P1-1 FIX: Reset argument mode when opening palette
    palette.is_argument_mode = false;
    palette.argument_input.clear();
    palette.pending_command = None;
}

pub fn handle_close_modal(state: &mut AppState) {
    state.mode = TuiMode::Chat;
    state.command_palette.open = false;
    state.command_palette.filter.clear();
    state.command_palette.selected = 0;
    state.permission_modal.tool = None;
    state.permission_modal.tool_call_id = None;
    state.diff_viewer = None;
    state.session_tree.hide();
}

/// P1-1 FIX: Handle Esc in command palette
/// If in argument mode, cancel argument input and return to command selection.
/// If not in argument mode, close the palette.
pub fn handle_palette_escape(state: &mut AppState, palette: &mut CommandPalette) {
    if palette.is_argument_mode {
        // Cancel argument mode and return to command selection
        palette.is_argument_mode = false;
        palette.argument_input.clear();
        palette.pending_command = None;
        // Reset filter to show all commands again
        palette.filter("");
        palette.selected = 0;
    } else {
        // Not in argument mode - close the palette
        handle_close_modal(state);
    }
}

pub fn handle_direct_command(state: &mut AppState, cmd: PaletteCommand) -> Vec<Cmd> {
    let mut cmds = vec![];
    match cmd {
        PaletteCommand::NewSession => { state.messages.clear(); state.mode = TuiMode::Chat; }
        PaletteCommand::LoadSession => { cmds.push(Cmd::LoadSession { name: String::new() }); }
        PaletteCommand::SaveSession => { cmds.push(Cmd::SaveSession { name: None }); }
        PaletteCommand::ClearChat => { state.messages.clear(); }
        PaletteCommand::SwitchModel | PaletteCommand::ReadFile { .. } | PaletteCommand::EditFile { .. } |
        PaletteCommand::WriteFile { .. } | PaletteCommand::DeleteFile { .. } | PaletteCommand::CompactContext => {}
        PaletteCommand::Quit => { state.running = false; }
        PaletteCommand::Cancel => {}
    }
    cmds
}
