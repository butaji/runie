use crate::tui::state::{AppState, TuiMode, Cmd};
use crate::components::{MessageItem, CommandPalette};
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

fn push_file_cmd(cmds: &mut Vec<Cmd>, state: &mut AppState, path: &str, file_cmd: Cmd, action: &str) {
    cmds.push(file_cmd);
    state.messages.push(MessageItem::System { text: format!("{}: {}", action, path) });
}

pub fn handle_switch_model(state: &mut AppState) {
    let picker = ModelPicker::with_default_models();
    state.model_picker = Some(picker);
    state.mode = TuiMode::Overlay;
}

fn handle_provider_command(state: &mut AppState, cmd: PaletteCommand) {
    match cmd {
        PaletteCommand::ManageProviders => {
            state.messages.push(MessageItem::System { text: "Provider management: use config file".to_string() });
        }
        PaletteCommand::AddProvider => {
            state.messages.push(MessageItem::System { text: "Add provider: not yet implemented".to_string() });
        }
        PaletteCommand::RemoveProvider => {
            state.messages.push(MessageItem::System { text: "Remove provider: not yet implemented".to_string() });
        }
        PaletteCommand::EditApiKey => {
            state.messages.push(MessageItem::System { text: "Edit API key: not yet implemented".to_string() });
        }
        PaletteCommand::SetProviderPriority => {
            state.messages.push(MessageItem::System { text: "Provider priority: not yet implemented".to_string() });
        }
        PaletteCommand::BrowseModels => {
            state.messages.push(MessageItem::System { text: "Browse models: not yet implemented".to_string() });
        }
        _ => {}
    }
}

pub fn handle_direct_command(state: &mut AppState, cmd: PaletteCommand) -> Vec<Cmd> {
    let mut cmds = vec![];
    match cmd {
        PaletteCommand::NewSession => {
            state.messages.clear();
            state.mode = TuiMode::Chat;
            state.messages.push(MessageItem::System { text: "New session started".to_string() });
        }
        PaletteCommand::LoadSession { name } => {
            cmds.push(Cmd::LoadSession { name: name.clone() });
            state.messages.push(MessageItem::System { text: format!("Loading session: {}", name) });
        }
        PaletteCommand::SaveSession { name } => {
            cmds.push(Cmd::SaveSession { name: Some(name.clone()) });
            state.messages.push(MessageItem::System { text: format!("Saving session: {}", name) });
        }
        PaletteCommand::ClearChat => {
            state.messages.clear();
            state.messages.push(MessageItem::System { text: "Chat cleared".to_string() });
        }
        PaletteCommand::SwitchModel => handle_switch_model(state),
        PaletteCommand::ReadFile { path } => push_file_cmd(&mut cmds, state, &path, Cmd::ReadFile { path: path.clone() }, "Reading file"),
        PaletteCommand::EditFile { path } => push_file_cmd(&mut cmds, state, &path, Cmd::EditFile { path: path.clone() }, "Editing file"),
        PaletteCommand::WriteFile { path } => push_file_cmd(&mut cmds, state, &path, Cmd::WriteFile { path: path.clone() }, "Writing file"),
        PaletteCommand::DeleteFile { path } => push_file_cmd(&mut cmds, state, &path, Cmd::DeleteFile { path: path.clone() }, "Deleting file"),
        PaletteCommand::CompactContext => {
            cmds.push(Cmd::CompactContext);
            state.messages.push(MessageItem::System { text: "Compacting context...".to_string() });
        }
        PaletteCommand::Quit => {
            state.running = false;
            state.messages.push(MessageItem::System { text: "Goodbye!".to_string() });
        }
        PaletteCommand::ManageProviders | PaletteCommand::AddProvider | PaletteCommand::RemoveProvider | PaletteCommand::EditApiKey | PaletteCommand::SetProviderPriority | PaletteCommand::BrowseModels => {
            handle_provider_command(state, cmd);
        }
        PaletteCommand::Cancel => {}
    }
    cmds
}
