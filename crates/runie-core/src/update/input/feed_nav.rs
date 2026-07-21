//! Turn and response anchor navigation.
//!
//! Implements Grok-style feed navigation patterns:
//!   - `h`/`l` — turn navigation (jump between user prompt boundaries)
//!   - `K`/`J` — response anchor nav (snap to first/last agent message in current turn)

use crate::model::AppState;
use crate::update::input::scroll::{scroll_to_post, current_top_post};
use crate::view::elements::PostKind;
use crate::view::turns::{rebuild_turns, turn_containing};

/// Jump to the previous turn's prompt boundary.
pub fn prev_turn(state: &mut AppState) {
    let snap = state.snapshot();
    let turns = rebuild_turns(&snap.elements);

    if turns.is_empty() {
        state.input_mut().input_flash = 3;
        return;
    }

    let current_idx = state.view().current_turn;

    // Find previous turn
    let target_idx = match current_idx {
        Some(idx) if idx > 0 => Some(idx - 1),
        Some(0) | None => {
            // At or before first turn — flash
            state.input_mut().input_flash = 3;
            return;
        }
        _ => None,
    };

    let idx = match target_idx {
        Some(i) => i,
        None => return,
    };

    if let Some(turn) = turns.get(idx) {
        let target_post = find_post_at_element(&snap, turn.prompt_index);
        state.view_mut().current_turn = Some(idx);
        state.view_mut().selected_post = target_post;
        state.view_mut().follow_mode = false;
        if let Some(post_idx) = target_post {
            scroll_to_post(state, &snap, post_idx);
        }
        state.view_mut().dirty = true;
    }
}

/// Jump to the next turn's prompt boundary.
pub fn next_turn(state: &mut AppState) {
    let snap = state.snapshot();
    let turns = rebuild_turns(&snap.elements);

    if turns.is_empty() {
        return;
    }

    let current_idx = state.view().current_turn;
    let last_idx = turns.len() - 1;

    let target_idx = match current_idx {
        Some(idx) if idx < last_idx => Some(idx + 1),
        Some(_) | None => {
            // Already at last turn or no turn — scroll to bottom
            state.view_mut().follow_mode = true;
            let visible = state.view().last_visible_height.max(3) as usize;
            let max_scroll = snap.total_lines.saturating_sub(visible);
            state.view_mut().scroll = max_scroll;
            if state.view().vim_nav_mode {
                state.view_mut().selected_post = snap.posts.len().checked_sub(1);
            }
            state.view_mut().dirty = true;
            return;
        }
    };

    let idx = match target_idx {
        Some(i) => i,
        None => return,
    };

    if let Some(turn) = turns.get(idx) {
        let target_post = find_post_at_element(&snap, turn.prompt_index);
        state.view_mut().current_turn = Some(idx);
        state.view_mut().selected_post = target_post;
        state.view_mut().follow_mode = false;
        if let Some(post_idx) = target_post {
            scroll_to_post(state, &snap, post_idx);
        }
        state.view_mut().dirty = true;
    }
}

/// Jump to the previous agent message anchor in the current turn.
pub fn prev_response(state: &mut AppState) {
    let snap = state.snapshot();
    let turns = rebuild_turns(&snap.elements);
    let current_turn_idx = state.view().current_turn;

    // Find the current turn's range
    let current_sel = state.view().selected_post;
    let range_start = if let Some(turn_idx) = current_turn_idx {
        turns.get(turn_idx).map(|t| t.prompt_index)
    } else if let Some(sel) = current_sel {
        snap.posts.get(sel).map(|p| p.start)
    } else {
        None
    };

    // Find previous agent response before the current position
    let current_elem_idx = current_sel
        .and_then(|sel| snap.posts.get(sel).map(|p| p.start));

    let mut found = None;
    if let Some(start_idx) = range_start {
        for (elem_idx, elem) in snap.elements.iter().enumerate() {
            if elem_idx >= start_idx {
                break;
            }
            if matches!(elem, crate::view::elements::Element::AgentMessage { .. }) {
                // Only consider if strictly before current position
                if let Some(cur) = current_elem_idx {
                    if elem_idx < cur {
                        found = Some(elem_idx);
                    }
                } else {
                    found = Some(elem_idx);
                }
            }
        }
    }

    if let Some(elem_idx) = found {
        if let Some(post_idx) = find_post_at_element(&snap, elem_idx) {
            state.view_mut().selected_post = Some(post_idx);
            scroll_to_post(state, &snap, post_idx);
            state.view_mut().follow_mode = false;
            state.view_mut().dirty = true;
        }
    } else {
        // Nothing found — flash
        state.input_mut().input_flash = 3;
    }
}

/// Jump to the next agent message anchor in the current turn (or any agent message if no turn context).
pub fn next_response(state: &mut AppState) {
    let snap = state.snapshot();
    let turns = rebuild_turns(&snap.elements);
    let current_turn_idx = state.view().current_turn;

    // Find the current turn's range
    let range_end = if let Some(turn_idx) = current_turn_idx {
        turns.get(turn_idx).map(|t| t.end_index)
    } else {
        None
    };

    let current_elem_idx = state
        .view()
        .selected_post
        .and_then(|sel| snap.posts.get(sel).map(|p| p.end));

    let mut found = None;
    let search_end = range_end.unwrap_or(snap.elements.len());

    for (elem_idx, elem) in snap.elements.iter().enumerate() {
        if elem_idx < search_end {
            if matches!(elem, crate::view::elements::Element::AgentMessage { .. }) {
                if let Some(cur) = current_elem_idx {
                    if elem_idx >= cur {
                        found = Some(elem_idx);
                        break;
                    }
                } else {
                    found = Some(elem_idx);
                    break;
                }
            }
        }
    }

    if let Some(elem_idx) = found {
        if let Some(post_idx) = find_post_at_element(&snap, elem_idx) {
            state.view_mut().selected_post = Some(post_idx);
            scroll_to_post(state, &snap, post_idx);
            state.view_mut().follow_mode = false;
            state.view_mut().dirty = true;
        }
    } else {
        // Nothing found — flash
        state.input_mut().input_flash = 3;
    }
}

/// Update current_turn when the selected post changes.
pub fn sync_current_turn(state: &mut AppState) {
    let snap = state.snapshot();
    let turns = rebuild_turns(&snap.elements);

    let sel = match state.view().selected_post {
        Some(s) => s,
        None => return,
    };

    let elem_idx = match snap.posts.get(sel) {
        Some(p) => p.start,
        None => return,
    };

    if let Some(turn_idx) = turn_containing(&turns, elem_idx) {
        if state.view().current_turn != Some(turn_idx) {
            state.view_mut().current_turn = Some(turn_idx);
        }
    }
}

/// Find the post index that contains the given element index.
fn find_post_at_element(snap: &crate::Snapshot, element_index: usize) -> Option<usize> {
    snap.posts
        .iter()
        .position(|p| p.start <= element_index && element_index < p.end)
        .or_else(|| {
            // Fallback: find first post at or after element
            snap.posts.iter().position(|p| p.start >= element_index)
        })
}
