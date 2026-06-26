//! Typed messages for `CompletionActor`.
//!
//! Each message variant carries the data needed to apply the mutation either:
//! - Asynchronously in `CompletionActor` (production)
//! - Synchronously via `apply_to()` (tests without a spawned actor)

use crate::actors::GenericActorHandle;
use crate::model::CompletionState;

/// All messages accepted by `CompletionActor`.
///
/// Covers path completion, @ mention suggestions, and ghost/tab completion state.
#[derive(Debug, Clone)]
pub enum CompletionMsg {
    // ── Path completion ───────────────────────────────────────────────────
    /// Toggle path completion popup.
    TogglePathCompletion { partial: String },
    /// Navigate to previous path suggestion.
    PathCompletionUp,
    /// Navigate to next path suggestion.
    PathCompletionDown,
    /// Select the current path suggestion and close popup.
    PathCompletionSelect { prefix: String },
    /// Close path completion popup without selecting.
    PathCompletionClose,

    // ── @ mention suggestions ─────────────────────────────────────────────
    /// Update @ mention suggestions list.
    AtSuggestionsChanged { suggestions: Vec<String> },
    /// Navigate to previous @ suggestion.
    AtSuggestionUp,
    /// Navigate to next @ suggestion.
    AtSuggestionDown,
    /// Insert the selected @ suggestion.
    AtSuggestionSelect,
    /// Clear @ suggestions.
    ClearAtRef,

    // ── Ghost/tab completion ───────────────────────────────────────────────
    /// Set ghost completion text.
    SetGhost { ghost: Option<String> },
    /// Set tab completion matches and prefix.
    SetTabComplete { prefix: Option<String>, matches: Vec<String> },
    /// Accept the current ghost/tab completion.
    AcceptGhost,
    /// Clear ghost completion state.
    ClearGhost,
    /// Advance to next tab completion match.
    TabCompleteNext,
    /// Abort file picker — restore input backup.
    FilePickerAbort,
    /// Clear all completion state.
    ClearAll,
}

/// Handle for sending messages to `CompletionActor`.
pub type CompletionActorHandle = GenericActorHandle<CompletionMsg>;

// ── Synchronous apply_to helpers ───────────────────────────────────────────────

/// Synchronous application of a `CompletionMsg` to `CompletionState`.
///
/// Mirrors `CompletionActor::handle_msg` for use in tests where the actor is not
/// spawned. This lets synchronous tests exercise the same mutation logic that
/// `CompletionActor` runs asynchronously in production.
impl CompletionMsg {
    pub fn apply_to(&self, state: &mut CompletionState) {
        match self {
            Self::TogglePathCompletion { partial } => apply_toggle_path(partial, state),
            Self::PathCompletionUp => apply_path_up(state),
            Self::PathCompletionDown => apply_path_down(state),
            Self::PathCompletionSelect { .. } => apply_path_close(state),
            Self::PathCompletionClose => apply_path_close(state),
            Self::AtSuggestionsChanged { suggestions } => apply_at_changed(suggestions, state),
            Self::AtSuggestionUp => apply_at_up(state),
            Self::AtSuggestionDown => apply_at_down(state),
            Self::AtSuggestionSelect => apply_at_select(state),
            Self::ClearAtRef => apply_clear_at(state),
            Self::SetGhost { .. } | Self::SetTabComplete { .. } | Self::AcceptGhost
            | Self::ClearGhost | Self::TabCompleteNext | Self::FilePickerAbort => {
                // These are handled by InputActor or are no-ops for CompletionState
            }
            Self::ClearAll => apply_clear_all(state),
        }
    }
}

fn apply_toggle_path(partial: &str, state: &mut CompletionState) {
    let cwd = std::env::current_dir().unwrap_or_default();
    let suggestions = crate::path_complete::complete_path(partial, &cwd);
    if suggestions.is_empty() {
        return;
    }
    state.path_suggestions = Some(suggestions);
    state.path_selected = Some(0);
}

fn apply_path_up(state: &mut CompletionState) {
    if let Some(ref items) = state.path_suggestions {
        let sel = state.path_selected.unwrap_or(0);
        state.path_selected = Some(if sel == 0 { items.len() - 1 } else { sel - 1 });
    }
}

fn apply_path_down(state: &mut CompletionState) {
    if let Some(ref items) = state.path_suggestions {
        let sel = state.path_selected.unwrap_or(0);
        state.path_selected = Some((sel + 1) % items.len());
    }
}

fn apply_path_close(state: &mut CompletionState) {
    state.path_suggestions = None;
    state.path_selected = None;
}

fn apply_at_changed(suggestions: &[String], state: &mut CompletionState) {
    state.at_suggestions = Some(suggestions.to_vec());
    state.at_selected = Some(0);
    state.last_at_query = None;
}

fn apply_at_up(state: &mut CompletionState) {
    let items = match state.at_suggestions.as_ref() {
        Some(i) if !i.is_empty() => i,
        _ => {
            state.at_suggestions = None;
            state.at_selected = None;
            return;
        }
    };
    let sel = state.at_selected.unwrap_or(0);
    state.at_selected = Some(if sel == 0 { items.len() - 1 } else { sel - 1 });
}

fn apply_at_down(state: &mut CompletionState) {
    let items = match state.at_suggestions.as_ref() {
        Some(i) if !i.is_empty() => i,
        _ => return,
    };
    let sel = state.at_selected.unwrap_or(0);
    state.at_selected = Some((sel + 1) % items.len());
}

fn apply_at_select(state: &mut CompletionState) {
    state.at_suggestions = None;
    state.at_selected = None;
}

fn apply_clear_at(state: &mut CompletionState) {
    state.at_suggestions = None;
    state.at_selected = None;
    state.last_at_query = None;
}

fn apply_clear_all(state: &mut CompletionState) {
    state.path_suggestions = None;
    state.path_selected = None;
    state.at_suggestions = None;
    state.at_selected = None;
    state.last_at_query = None;
}
