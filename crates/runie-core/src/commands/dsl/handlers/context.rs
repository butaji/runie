//! `/context` feed snapshot.

use crate::commands::CommandResult;
use crate::model::{AppState, Role};

/// Emit a typed context snapshot for the scrollback feed.
pub fn handle_context(state: &mut AppState, _args: &str) -> CommandResult {
    let model = state.current_model().to_owned();
    let used = state.agent_state().tokens_in;
    let total = state.current_model_context_window().unwrap_or_default();
    let turns = state.session().messages.iter().filter(|m| m.role == Role::User).count();
    let tool_calls = state.session().messages.iter().map(|m| m.tool_calls().len()).sum::<usize>();
    state.add_system_msg(format!("Context snapshot: {model}|{used}|{total}|{turns}|{tool_calls}"));
    CommandResult::None
}
