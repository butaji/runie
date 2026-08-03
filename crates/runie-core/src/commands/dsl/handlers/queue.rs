//! `/queue` read-only snapshot of prompts waiting behind the active turn.

use crate::commands::CommandResult;
use crate::model::AppState;

/// Commit the current local queue as a feed system row, matching Grok's
/// queue-inspection behavior without introducing a second overlay surface.
pub fn handle_queue(state: &mut AppState, _args: &str) -> CommandResult {
    let queued = &state.agent_state().message_queue;
    if queued.is_empty() {
        state.add_system_msg("Queued prompts\n(none)".to_owned());
        return CommandResult::None;
    }

    let mut snapshot = format!("Queued prompts ({})", queued.len());
    for (index, message) in queued.iter().enumerate() {
        snapshot.push_str(&format!("\n#{}  {}", index + 1, message.content));
    }
    state.add_system_msg(snapshot);
    CommandResult::None
}
