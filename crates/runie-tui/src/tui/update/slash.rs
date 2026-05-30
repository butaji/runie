use crate::components::MessageItem;
use crate::tui::state::{AppState, TuiMode};
use crate::tui::update::ui::UiCmd;

pub fn handle_slash(state: &mut AppState, cmd: runie_core::slash_command::SlashCommand) -> Vec<UiCmd> {
    use runie_core::slash_command::SlashCommand;
    match cmd {
        // Session-modifying commands - grouped
        SlashCommand::New | SlashCommand::Clear | SlashCommand::Fork | SlashCommand::Copy =>
            { handle_session_cmd(state, &cmd); vec![] }
        SlashCommand::Model(model) => { handle_model(state, model); vec![] }
        SlashCommand::Tree => { handle_tree(state); vec![] }
        SlashCommand::Quit => { handle_quit(state); vec![] }
        // Informational - grouped
        SlashCommand::Help | SlashCommand::Cost => { handle_info_cmd(state, &cmd); vec![] }
        SlashCommand::Unknown(cmd) => { handle_unknown(state, cmd); vec![] }
    }
}

fn handle_session_cmd(state: &mut AppState, cmd: &runie_core::slash_command::SlashCommand) {
    use runie_core::slash_command::SlashCommand;
    match cmd {
        SlashCommand::New => handle_new(state),
        SlashCommand::Clear => handle_clear(state),
        SlashCommand::Fork => handle_fork(state),
        SlashCommand::Copy => handle_copy(state),
        _ => {}
    }
}

fn handle_info_cmd(state: &mut AppState, cmd: &runie_core::slash_command::SlashCommand) {
    use runie_core::slash_command::SlashCommand;
    match cmd {
        SlashCommand::Help => handle_help(state),
        SlashCommand::Cost => handle_cost(state),
        _ => {}
    }
}

fn handle_new(state: &mut AppState) {
    state.messages.clear();
    state.scroll.feed_offset = 0;
    state.messages.push(MessageItem::System { text: "New session started".to_string() });
}

fn handle_clear(state: &mut AppState) {
    state.messages.clear();
    state.scroll.feed_offset = 0;
}

fn handle_model(state: &mut AppState, model: String) {
    state.current_model = Some(model.clone());
    state.messages.push(MessageItem::System { text: format!("Model switched to {}", model) });
}

fn handle_fork(state: &mut AppState) {
    state.messages.push(MessageItem::System { text: "Fork created at current position".to_string() });
}

fn handle_quit(state: &mut AppState) {
    state.running = false;
}

fn handle_help(state: &mut AppState) {
    state.messages.push(MessageItem::System { text: runie_core::slash_command::format_help() });
}

fn handle_unknown(state: &mut AppState, cmd: String) {
    state.messages.push(MessageItem::System { text: format!("Unknown command: {}. Type /help for available commands.", cmd) });
}

fn handle_cost(_state: &mut AppState) {
    // Cost command shows cost info - not implemented yet
}

fn handle_copy(state: &mut AppState) {
    state.messages.push(MessageItem::System { text: "Copy command - not implemented yet".to_string() });
}

pub fn handle_tree(state: &mut AppState) {
    state.session_tree.toggle();
    state.mode = if state.session_tree.visible { TuiMode::SessionTree } else { TuiMode::Chat };
}
