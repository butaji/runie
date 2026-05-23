use crate::components::MessageItem;
use crate::tui::state::{AppState, Cmd, TuiMode};

pub fn handle_slash(state: &mut AppState, cmd: runie_core::slash_command::SlashCommand) -> Vec<Cmd> {
    let mut cmds = vec![];
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
        runie_core::slash_command::SlashCommand::Compact => {
            state.messages.push(MessageItem::System { text: "Session compaction not yet implemented".to_string() });
        }
        runie_core::slash_command::SlashCommand::Save(name) => cmds.push(Cmd::SaveSession { name }),
        runie_core::slash_command::SlashCommand::Load(name) => cmds.push(Cmd::LoadSession { name }),
        runie_core::slash_command::SlashCommand::Tree => handle_tree(state),
        runie_core::slash_command::SlashCommand::Fork => state.messages.push(MessageItem::System { text: "Fork created at current position".to_string() }),
        runie_core::slash_command::SlashCommand::Quit => state.running = false,
        runie_core::slash_command::SlashCommand::Help => {
            state.messages.push(MessageItem::System { text: runie_core::slash_command::format_help() });
        }
        runie_core::slash_command::SlashCommand::Unknown(cmd) => {
            state.messages.push(MessageItem::System { text: format!("Unknown command: {}. Type /help for available commands.", cmd) });
        }
    }
    cmds
}

pub fn handle_tree(state: &mut AppState) {
    state.session_tree.toggle();
    state.mode = if state.session_tree.visible { TuiMode::SessionTree } else { TuiMode::Chat };
}
