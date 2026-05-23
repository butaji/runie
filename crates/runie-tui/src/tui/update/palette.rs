use crate::components::MessageItem;
use crate::tui::state::{AppState, TuiMode, Cmd};

pub fn open_palette(state: &mut AppState) {
    state.command_palette.open = true;
    state.mode = TuiMode::CommandPalette;
    state.command_palette.filter.clear();
    state.command_palette.selected = 0;
}

pub fn handle_palette_msg(state: &mut AppState, msg: crate::tui::state::Msg) {
    use crate::tui::state::Msg;
    match msg {
        Msg::CommandPaletteUp => {
            if state.command_palette.selected > 0 {
                state.command_palette.selected -= 1;
            }
        }
        Msg::CommandPaletteDown => state.command_palette.selected += 1,
        Msg::CommandPaletteConfirm => handle_close_modal(state),
        _ => {}
    }
}

pub fn handle_close_modal(state: &mut AppState) {
    state.mode = TuiMode::Chat;
    state.command_palette.open = false;
    state.permission_modal.tool = None;
    state.permission_modal.tool_call_id = None;
    state.diff_viewer = None;
    state.session_tree.hide();
}
