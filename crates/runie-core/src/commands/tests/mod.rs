use super::*;
use crate::Event;
use crate::model::AppState;

/// Type a slash command directly into the input and submit it.
/// Bypasses the `/` → command-palette shortcut so tests can exercise
/// the slash-command dispatcher itself.
pub(super) fn run_slash(state: &mut AppState, text: &str) {
    state.input.input = text.to_string();
    state.input.cursor_pos = text.len();
    state.update(crate::Event::Submit);
}

/// Execute the handler function inside a command's flow, ignoring any
/// `.sub()` wrapper that would otherwise push the current dialog onto
/// the back stack.
pub(super) fn exec_handler(state: &mut AppState, name: &str, args: &str) -> CommandResult {
    let cmd = state.registry.get(name).unwrap();
    match &cmd.flow {
        CommandFlow::Handler(f) => f(state, args),
        CommandFlow::Sub(inner) => match inner.as_ref() {
            CommandFlow::Handler(f) => f(state, args),
            _ => panic!("command {name} is not a handler"),
        },
        _ => panic!("command {name} is not a handler"),
    }
}

pub(super) fn palette_stack(state: &AppState) -> Option<&crate::dialog::PanelStack> {
    match &state.open_dialog {
        Some(DialogState::CommandPalette(stack)) => Some(stack),
        _ => None,
    }
}

mod forms;
mod handlers;
mod hotkeys;
mod model;
mod palette;
mod prompts;
mod registry;
mod session;
mod skills;
mod slash_dispatch;
mod usage;
