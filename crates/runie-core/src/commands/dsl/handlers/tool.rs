//! Safety and permission commands.

use crate::commands::CommandResult;
use crate::model::AppState;

crate::handlers! {
    registry,
    "readonly" => |_: &mut AppState, _: &str| CommandResult::Event(crate::Event::ToggleReadOnly),
    "trust" => |_: &mut AppState, _: &str| CommandResult::Event(crate::Event::TrustProject),
    "untrust" => |_: &mut AppState, _: &str| CommandResult::Event(crate::Event::UntrustProject),
    // Keyboard-only trigger for the context-detail swap (grok parity;
    // mouse support is rejected). Toggles the pin; the TUI renders the
    // progress bar + percentage while pinned.
    "context-detail" => |state: &mut AppState, _: &str| {
        let pinned = {
            let view = state.view_mut();
            *view.context_detail_pinned_mut() = !view.context_detail_pinned;
            view.context_detail_pinned
        };
        CommandResult::Message(if pinned {
            "Context detail expanded — showing usage progress bar.".into()
        } else {
            "Context detail collapsed — showing token usage.".into()
        })
    },
}
