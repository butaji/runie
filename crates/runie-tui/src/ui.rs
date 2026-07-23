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
mod render_lines;
mod scroll;
pub(crate) mod subagent_detail;
pub(crate) mod tasks_pane;
mod utils;

pub use input::count_input_lines;
pub use render_lines::element_line_count;
pub use scroll::render_scrollbar;

pub(crate) use hints::parse_hint_spans;
pub(crate) use layout::hstack;
pub(crate) use messages::estimate_element_tokens;

/// Width of the Grok-style tasks side pane in columns.
const TASKS_PANE_WIDTH: u16 = 32;

/// Draw a Snapshot to the terminal.
#[allow(clippy::too_many_lines)]
pub fn draw_snapshot(f: &mut Frame, snap: &Snapshot) {
    let full_area = f.area();
    f.buffer_mut()
        .set_style(full_area, Style::default().bg(color_bg()));
    let margin = if full_area.width > 20 && full_area.height > 10 {
        Margin::new(1, 1)
    } else {
        Margin::new(0, 0)
    };
    let area = full_area.inner(margin);
    let constraints = snapshot_constraints(snap);
    let c = layout::vstack(area, &constraints);

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
        // c[1] is the empty margin line — no rendering needed
        crate::status_bar::render(f, snap, c[2]);
        input::input(f, snap, c[3]);
    }
    if snap.has_models {
        hints::hints(f, snap, c[5]);
    } else if c.len() > 1 {
        hints::hints(f, snap, c[1]);
    }
    crate::popups::path_suggestions(f, snap);
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
        vec![
            Constraint::Min(3),
            Constraint::Length(1), // empty margin above status
            Constraint::Length(1), // status
            Constraint::Length(input_height),
            Constraint::Length(1),
            Constraint::Length(1), // hints
        ]
    } else {
        vec![
            Constraint::Length(snap.last_visible_height),
            Constraint::Length(2), // hints bar (fixed height, not expandable)
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
