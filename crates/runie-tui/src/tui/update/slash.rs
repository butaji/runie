use crate::components::MessageItem;
use crate::tui::state::{AppState, TuiMode};

pub fn handle_slash(state: &mut AppState, cmd: runie_core::slash_command::SlashCommand) -> Vec<crate::tui::state::Cmd> {
    match cmd {
        runie_core::slash_command::SlashCommand::New => {
            state.messages.clear();
            state.scroll.feed_offset = 0;
            state.messages.push(MessageItem::System { text: "New session started".to_string() });
        }
        runie_core::slash_command::SlashCommand::Clear => {
            state.messages.clear();
            state.scroll.feed_offset = 0;
        }
        runie_core::slash_command::SlashCommand::Model(model) => {
            state.current_model = Some(model.clone());
            state.messages.push(MessageItem::System { text: format!("Model switched to {}", model) });
        }
        runie_core::slash_command::SlashCommand::Tree => handle_tree(state),
        runie_core::slash_command::SlashCommand::Fork => state.messages.push(MessageItem::System { text: "Fork created at current position".to_string() }),
        runie_core::slash_command::SlashCommand::Quit => state.running = false,
        runie_core::slash_command::SlashCommand::Help => {
            state.messages.push(MessageItem::System { text: runie_core::slash_command::format_help() });
        }
        runie_core::slash_command::SlashCommand::Unknown(cmd) => {
            state.messages.push(MessageItem::System { text: format!("Unknown command: {}. Type /help for available commands.", cmd) });
        }
        runie_core::slash_command::SlashCommand::Cost => {
            // Cost command shows cost info - not implemented yet
        }
    }
    vec![]
}

pub fn handle_tree(state: &mut AppState) {
    state.session_tree.toggle();
    state.mode = if state.session_tree.visible { TuiMode::SessionTree } else { TuiMode::Chat };
}
