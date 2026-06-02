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
    state.extensions_modal = None;
}

pub fn handle_palette_escape(state: &mut AppState, palette: &mut CommandPalette) {
    if palette.is_argument_mode {
        palette.cancel_argument_mode();
        palette.filter("");
    } else {
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

fn handle_theme(state: &mut AppState) -> Vec<UiCmd> {
    state.current_theme = crate::theme::ThemeWrapper::cycle_theme(&state.current_theme).name().to_string();
    vec![]
}

fn handle_simple_slash(state: &mut AppState, cmd: runie_core::slash_command::SlashCommand) -> Vec<UiCmd> {
    run_slash(state, cmd)
}

fn handle_copy_last(state: &mut AppState) -> Vec<UiCmd> {
    let cmds = crate::tui::update::ui::handle_copy_last_response(state);
    state.mode = TuiMode::Chat;
    state.command_palette.open = false;
    state.command_palette.filter.clear();
    state.command_palette.selected = 0;
    cmds
}

fn handle_cancel(state: &mut AppState) -> Vec<UiCmd> {
    state.command_palette.open = false;
    state.command_palette.filter.clear();
    state.command_palette.selected = 0;
    vec![]
}

fn handle_quit(state: &mut AppState) -> Vec<UiCmd> {
    state.running = false;
    vec![UiCmd::Quit]
}

pub fn handle_direct_command(state: &mut AppState, cmd: PaletteCommand) -> Vec<UiCmd> {
    use PaletteCommand::*;
    match cmd {
        NewSession | ClearChat | ForkSession | ShowCost | Help | Extensions =>
            run_simple_slash(state, cmd),
        SwitchModel => handle_switch_model(state),
        SessionTree => handle_session_tree(state),
        Onboard => open_onboarding(state),
        CopyLast => handle_copy_last(state),
        Theme => handle_theme(state),
        Quit => handle_quit(state),
        Cancel => handle_cancel(state),
    }
}

fn run_simple_slash(state: &mut AppState, cmd: PaletteCommand) -> Vec<UiCmd> {
    use PaletteCommand::*;
    use runie_core::slash_command::SlashCommand;
    let slash_cmd = match cmd {
        NewSession => SlashCommand::New,
        ClearChat => SlashCommand::Clear,
        ForkSession => SlashCommand::Fork,
        ShowCost => SlashCommand::Cost,
        Help => SlashCommand::Help,
        Extensions => SlashCommand::Extensions,
        _ => unreachable!(),
    };
    handle_simple_slash(state, slash_cmd)
}

fn handle_new_session(state: &mut AppState) -> Vec<UiCmd> {
    handle_simple_slash(state, runie_core::slash_command::SlashCommand::New)
}

fn handle_clear_chat(state: &mut AppState) -> Vec<UiCmd> {
    handle_simple_slash(state, runie_core::slash_command::SlashCommand::Clear)
}

fn handle_fork_session(state: &mut AppState) -> Vec<UiCmd> {
    handle_simple_slash(state, runie_core::slash_command::SlashCommand::Fork)
}

fn handle_session_tree(state: &mut AppState) -> Vec<UiCmd> {
    crate::tui::update::slash::handle_tree(state);
    vec![]
}

fn handle_show_cost(state: &mut AppState) -> Vec<UiCmd> {
    handle_simple_slash(state, runie_core::slash_command::SlashCommand::Cost)
}

fn handle_help(state: &mut AppState) -> Vec<UiCmd> {
    handle_simple_slash(state, runie_core::slash_command::SlashCommand::Help)
}

fn handle_extensions(state: &mut AppState) -> Vec<UiCmd> {
    handle_simple_slash(state, runie_core::slash_command::SlashCommand::Extensions)
}
