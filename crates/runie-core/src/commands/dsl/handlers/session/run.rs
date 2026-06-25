//! Session run helpers (name/fork/compact).

use crate::model::AppState;

/// Set the session display name.
pub fn run_name(state: &mut AppState, name: &str) {
    let name = name.trim();
    if name.is_empty() {
        let current = state
            .session
            .session_display_name
            .as_deref()
            .unwrap_or("(unset)");
        state.add_system_msg(format!("Current display name: {}", current));
        return;
    }
    let truncated = if name.chars().count() > 64 {
        format!("{}…", name.chars().take(64).collect::<String>())
    } else {
        name.to_string()
    };
    state.session.session_display_name = Some(truncated.clone());
    state.add_system_msg(format!("Session name set to '{}'", truncated));
}

/// Default fork index: the most recent user message. 0 if no user messages.
pub fn fallback_fork_index(state: &AppState) -> usize {
    state
        .session
        .messages
        .iter()
        .enumerate()
        .rfind(|(_, m)| m.role == crate::model::Role::User)
        .map(|(i, _)| i)
        .unwrap_or(0)
}

/// Fork the session at a message index.
pub fn run_fork(state: &mut AppState, index_raw: &str) {
    let message_index = if index_raw.trim().is_empty() {
        fallback_fork_index(state)
    } else {
        match index_raw.trim().parse::<usize>() {
            Ok(n) => n,
            Err(_) => {
                state.add_system_msg(format!(
                    "Invalid message index '{}': expected a non-negative integer.",
                    index_raw
                ));
                return;
            }
        }
    };
    if message_index >= state.session.messages.len() {
        state.add_system_msg(format!(
            "Index {} out of range (0–{})",
            message_index,
            state.session.messages.len().saturating_sub(1)
        ));
        return;
    }
    let mut tree = state.session.session_tree.take().unwrap_or_else(|| {
        crate::session::tree::SessionTree::from_messages(&state.session.messages)
    });
    match tree.fork_at(message_index) {
        Some(path) => {
            tree.navigate_to(&path);
            state.session.session_tree = Some(tree);
            state.add_system_msg(format!("Forked at message {}.", message_index));
        }
        None => state.add_system_msg("Could not fork.".into()),
    }
}

/// Compact the context, keeping the last `keep_raw` tokens.
pub fn run_compact(state: &mut AppState, keep_raw: &str, focus: &str) {
    let keep = if keep_raw.trim().is_empty() {
        2000
    } else {
        match keep_raw.trim().parse::<usize>() {
            Ok(n) if n > 0 => n,
            Ok(_) => 2000,
            Err(_) => {
                state.add_system_msg(format!(
                    "Invalid keep value '{}': expected a positive integer.",
                    keep_raw
                ));
                return;
            }
        }
    };
    let msg = state.compact(keep);
    let result = if focus.is_empty() {
        msg
    } else {
        format!("{} (focus: {})", msg, focus)
    };
    state.add_system_msg(result);
}
