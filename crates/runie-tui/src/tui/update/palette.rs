use crate::tui::state::{AppState, TuiMode, Cmd};
use crate::components::command_palette::PaletteCommand;

pub fn open_palette(state: &mut AppState) {
    state.command_palette.open = true;
    state.mode = TuiMode::CommandPalette;
    state.command_palette.filter.clear();
    state.command_palette.selected = 0;
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
