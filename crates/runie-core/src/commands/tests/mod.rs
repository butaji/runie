use super::*;
use crate::commands::DialogKind;
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
/// the back stack. Handles both simple `Handler` and `FormWithHandler`
/// (PanelStack + form_handler) variants.
pub(super) fn exec_handler(state: &mut AppState, name: &str, args: &str) -> CommandResult {
    let cmd = state.registry.get(name).unwrap_or_else(|| panic!("command {name} not found"));
    match &cmd.flow {
        CommandFlow::Handler(f) => f(state, args),
        CommandFlow::Sub(inner) => match inner.as_ref() {
            CommandFlow::Handler(f) => f(state, args),
            _ => panic!("command {name} is not a handler"),
        },
        CommandFlow::PanelStack(_) => {
            // FormWithHandler: flow is PanelStack, actual handler is in form_handler field.
            if let Some(f) = cmd.form_handler {
                f(state, args)
            } else {
                panic!("command {name} has PanelStack flow but no form_handler")
            }
        }
        _ => panic!("command {name} is not a handler"),
    }
}

pub(super) fn palette_stack(state: &AppState) -> Option<&crate::dialog::PanelStack> {
    match &state.open_dialog {
        Some(DialogState::Active {
            kind: DialogKind::CommandPalette,
            panels: stack,
        }) => Some(stack),
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
