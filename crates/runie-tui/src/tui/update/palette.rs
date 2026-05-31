use crate::tui::state::{AppState, TuiMode};
use crate::tui::update::ui::UiCmd;
use crate::components::CommandPalette;
use crate::components::command_palette::PaletteCommand;
use crate::components::model_picker::ModelPicker;

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
    state.model_picker = None;
}

/// P1-1 FIX: Handle Esc in command palette
/// If in argument mode, cancel argument input and return to command selection.
/// If not in argument mode, close the palette.
pub fn handle_palette_escape(state: &mut AppState, palette: &mut CommandPalette) {
    if palette.is_argument_mode {
        // BUG-08 FIX: Use cancel_argument_mode() method
        palette.cancel_argument_mode();
        // Reset filter to show all commands again
        palette.filter("");
    } else {
        // Not in argument mode - close the palette
        handle_close_modal(state);
    }
}

pub fn handle_switch_model(state: &mut AppState) -> Vec<UiCmd> {
    let picker = ModelPicker::with_default_models();
    state.model_picker = Some(picker);
    state.mode = TuiMode::Overlay;
    vec![]
}

fn run_slash(state: &mut AppState, cmd: runie_core::slash_command::SlashCommand) -> Vec<UiCmd> {
    let result = crate::tui::update::slash::handle_slash(state, cmd);
    state.mode = TuiMode::Chat;
    state.command_palette.open = false;
    result
}

fn open_onboarding(state: &mut AppState) -> Vec<UiCmd> {
    state.mode = TuiMode::Onboarding;
    state.onboarding = Some(crate::components::Onboarding::default());
    vec![]
}

pub fn handle_direct_command(state: &mut AppState, cmd: PaletteCommand) -> Vec<UiCmd> {
    use runie_core::slash_command::SlashCommand;
    match cmd {
        PaletteCommand::NewSession => run_slash(state, SlashCommand::New),
        PaletteCommand::ClearChat => run_slash(state, SlashCommand::Clear),
        PaletteCommand::SwitchModel => {
            handle_switch_model(state);
            // SwitchModel opens model picker - close command palette but keep picker open
            state.command_palette.open = false;
            state.command_palette.filter.clear();
            state.command_palette.selected = 0;
            vec![]
        }
        PaletteCommand::ForkSession => run_slash(state, SlashCommand::Fork),
        PaletteCommand::SessionTree => {
            crate::tui::update::slash::handle_tree(state);
            vec![]
        }
        PaletteCommand::Onboard => open_onboarding(state),
        PaletteCommand::CopyLast => {
            let cmds = crate::tui::update::ui::handle_copy_last_response(state);
            state.mode = TuiMode::Chat;
            state.command_palette.open = false;
            cmds
        }
        PaletteCommand::ShowCost => run_slash(state, SlashCommand::Cost),
        PaletteCommand::Help => run_slash(state, SlashCommand::Help),
        PaletteCommand::Quit => {
            state.running = false;
            vec![UiCmd::Quit]
        }
        PaletteCommand::Cancel => vec![],
    }
}
