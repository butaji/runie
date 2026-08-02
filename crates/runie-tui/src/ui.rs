//! View — renders Snapshot to terminal via ratatui
//!
//! Architecture: the event loop builds immutable Snapshots;
//! the render actor draws them. No state mutations, no blocking
//! I/O, no caching — pure functions from Snapshot to Frame.
//!
//! DESIGN SYSTEM RULE: all colors, glyphs, and styles come from
//! crate::theme only. No literals, no hardcoded values.

use ratatui::{
    layout::{Constraint, Layout, Margin},
    style::Style,
    Frame,
};
use runie_core::Snapshot;

use crate::theme::color_bg;

pub(crate) mod feed_detail;
mod goal_pane;
mod hints;
mod input;
mod layout;
pub(crate) mod messages;
mod progress_bar;
mod queue_pane;
mod render_lines;
mod scroll;
pub(crate) mod subagent_detail;
pub(crate) mod tasks_pane;
mod utils;

pub use input::count_input_lines;
pub use render_lines::{element_line_count, to_lines_internal};
pub use scroll::render_scrollbar;

pub(crate) use hints::parse_hint_spans;
pub(crate) use layout::hstack;
pub(crate) use messages::estimate_element_tokens;
pub(crate) use progress_bar::progress_bar_spans;

/// Width of the Grok-style tasks side pane in columns.
const TASKS_PANE_WIDTH: u16 = 32;

/// Hard-degradation gate: below this height, optional rows (margin, hints)
/// are stripped so the prompt is never starved.
pub const SHORT_TERMINAL_ROWS: u16 = 16;

/// Auto-compact threshold: at or below this height, compact layout is
/// automatically engaged (unless explicitly overridden by the user).
pub const AUTO_COMPACT_MAX_ROWS: u16 = 20;

// Compile-time assertion: SHORT < AUTO.
const _: () = assert!(SHORT_TERMINAL_ROWS < AUTO_COMPACT_MAX_ROWS);

/// Derive the effective compact flag.
///
/// Compact is on when either the user set it manually, OR the terminal
/// is tall enough to measure (rows > 0) and is within the auto-compact band.
pub fn effective_compact(user_compact: bool, terminal_rows: u16) -> bool {
    user_compact || (terminal_rows > 0 && terminal_rows <= AUTO_COMPACT_MAX_ROWS)
}

/// Draw a Snapshot to the terminal.
#[allow(clippy::too_many_lines)]
pub fn draw_snapshot(f: &mut Frame, snap: &Snapshot) {
    let full_area = f.area();
    f.buffer_mut()
        .set_style(full_area, Style::default().bg(color_bg()));
    let margin = if snap.compact_layout {
        Margin::new(0, 0)
    } else {
        Margin::new(1, 1)
    };
    let area = full_area.inner(margin);
    let constraints = snapshot_constraints(snap);
    let c = layout::vstack(area, &constraints);

    // Runie has no persistent top context/header bar. The feed starts at the
    // top of the frame; the sole persistent status surface is rendered above
    // the input below.
    // Feed area is c[0]; split with tasks/goal pane when active.
    let message_area = if (snap.tasks_pane_visible || snap.goal_state.is_some()) && area.width > TASKS_PANE_WIDTH + 10 {
        let h = Layout::horizontal([Constraint::Min(10), Constraint::Length(TASKS_PANE_WIDTH)]);
        let split = h.split(c[0]);
        if snap.tasks_pane_visible {
            tasks_pane::render_tasks_pane(f, snap, split[1]);
        }
        if snap.goal_state.is_some() {
            goal_pane::render_goal_pane(f, snap, split[1]);
        }
        split[0]
    } else {
        c[0]
    };

    messages::render_messages(f, snap, message_area);

    if snap.has_models {
        // Dynamic layout: compact has fewer rows, so derive indices from constraint count.
        // Normal: [feed=0, (queue), margin, status, input, margin, hints]
        // Compact: [feed=0, (queue), status, input, spacer, (hints)]
        let queue_offset = if queue_pane::queue_pane_height(snap) > 0 { 1 } else { 0 };
        let status_idx = if snap.compact_layout { 1 } else { 2 } + queue_offset;
        let input_idx = status_idx + 1;
        if queue_offset > 0 {
            queue_pane::render(f, snap, c[1]);
        }
        crate::status_bar::render(f, snap, c[status_idx]);
        input::input(f, snap, c[input_idx]);
        // Hints: render if present in constraints (last slot)
        if c.len() > input_idx + 1 {
            // Keep one quiet row between the composer and the hotkeys bar.
            // The hints remain the final layout row in every mode.
            let hints_idx = input_idx + 2;
            if c.len() > hints_idx {
                hints::hints(f, snap, c[hints_idx]);
            }
        }
    } else if c.len() > 1 {
        hints::hints(f, snap, c[1]);
    }
    crate::popups::path_suggestions(f, snap);
    crate::popups::slash_dropdown(f, snap);
    crate::popups::panel::panel_dialog(f, snap);
    crate::popups::plan::render_plan_panel(f, snap);

    if snap.subagent_detail.is_some() {
        subagent_detail::render_subagent_detail(f, snap, message_area);
    }
    if snap.feed_element_detail.is_some() {
        feed_detail::render_feed_detail(f, snap, message_area);
    }
}

fn snapshot_constraints(snap: &Snapshot) -> Vec<Constraint> {
    if snap.has_models {
        let input_lines = count_input_lines(&snap.input_display);
        let input_height = (input_lines + 2).min(10) as u16;
        let queue_height = queue_pane::queue_pane_height(snap);

        let show_hints = snap.terminal_rows == 0
            || snap.terminal_rows > SHORT_TERMINAL_ROWS
            || snap.transient_message.is_some();

        if snap.compact_layout {
            let mut c = vec![
                Constraint::Min(3),    // messages/feed
            ];
            if queue_height > 0 {
                c.push(Constraint::Length(queue_height)); // queue pane
            }
            c.push(Constraint::Length(1)); // status
            c.push(Constraint::Length(input_height)); // input — one line when empty
            if show_hints {
                c.push(Constraint::Length(1)); // one-line gap above hints
                c.push(Constraint::Length(1)); // hints
            }
            c
        } else {
            let mut c = vec![
                Constraint::Min(3),    // messages/feed
            ];
            if queue_height > 0 {
                c.push(Constraint::Length(queue_height)); // queue pane
            }
            c.extend([
                Constraint::Length(1), // empty margin above status
                Constraint::Length(1), // status
                Constraint::Length(input_height), // input — one line when empty
                Constraint::Length(0), // no gap: input border directly precedes hints
                Constraint::Length(1), // hints
            ]);
            c
        }
    } else {
        vec![
            Constraint::Length(snap.last_visible_height),
            Constraint::Length(2), // hints bar
        ]
    }
}

/// Test helper: render the current AppState to a frame.
///
/// Production code should build a `Snapshot` and call `draw_snapshot` instead.
/// This helper only performs cache-building (ensure_fresh + snapshot); it no
/// longer writes viewport dimensions back into state.
pub fn view(f: &mut Frame, state: &mut runie_core::AppState) {
    state.ensure_fresh();
    let snap = state.snapshot();
    draw_snapshot(f, &snap);
}
