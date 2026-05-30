//! Command palette helpers: filter, navigation, confirm.

use crate::components::CommandPalette;
use crate::tui::state::AppState;

/// Handle command palette messages - delegates to specific handlers.
pub fn handle_palette(state: &mut AppState, palette: &mut CommandPalette, msg: &crate::tui::state::Msg) -> Vec<crate::tui::update::ui::UiCmd> {
    use crate::tui::state::Msg;
    match msg {
        Msg::OpenCommandPalette => handle_open_command_palette(state, palette),
        Msg::CommandPaletteUp => handle_palette_up(state, palette),
        Msg::CommandPaletteDown => handle_palette_down(state, palette),
        Msg::CommandPaletteConfirm => handle_palette_confirm(state, palette),
        Msg::CommandPaletteBackspace => handle_palette_backspace(state, palette),
        Msg::CommandPaletteCancelArgument => handle_palette_cancel_argument(state, palette),
        Msg::CommandPaletteFilter(c) => handle_palette_filter(state, palette, *c),
        _ => vec![],
    }
}

fn handle_palette_filter(state: &mut AppState, palette: &mut CommandPalette, c: char) -> Vec<crate::tui::update::ui::UiCmd> {
    state.command_palette.filter.push(c);
    palette.filter(&state.command_palette.filter);
    palette.selected = palette.selected.min(palette.filtered_commands.len().saturating_sub(1));
    state.command_palette.selected = palette.selected;
    vec![]
}

fn handle_palette_backspace(state: &mut AppState, palette: &mut CommandPalette) -> Vec<crate::tui::update::ui::UiCmd> {
    state.command_palette.filter.pop();
    palette.filter(&state.command_palette.filter);
    palette.selected = palette.selected.min(palette.filtered_commands.len().saturating_sub(1));
    state.command_palette.selected = palette.selected;
    vec![]
}

fn handle_palette_up(state: &mut AppState, palette: &mut CommandPalette) -> Vec<crate::tui::update::ui::UiCmd> {
    palette.selected = palette.selected.saturating_sub(1);
    state.command_palette.selected = palette.selected;
    vec![]
}

fn handle_palette_down(state: &mut AppState, palette: &mut CommandPalette) -> Vec<crate::tui::update::ui::UiCmd> {
    palette.selected = (palette.selected + 1).min(palette.filtered_commands.len().saturating_sub(1));
    state.command_palette.selected = palette.selected;
    vec![]
}

fn handle_palette_confirm(state: &mut AppState, palette: &mut CommandPalette) -> Vec<crate::tui::update::ui::UiCmd> {
    if let Some(cmd) = palette.confirm(palette.selected) {
        let cmds = super::super::palette::handle_direct_command(state, cmd);
        super::super::palette::handle_close_modal(state);
        return cmds;
    }
    vec![]
}

fn handle_open_command_palette(state: &mut AppState, palette: &mut CommandPalette) -> Vec<crate::tui::update::ui::UiCmd> {
    super::super::palette::open_palette(state, palette);
    vec![]
}

fn handle_palette_cancel_argument(state: &mut AppState, palette: &mut CommandPalette) -> Vec<crate::tui::update::ui::UiCmd> {
    super::super::palette::handle_palette_escape(state, palette);
    vec![]
}
