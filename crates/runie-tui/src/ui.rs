//! View — renders Snapshot to terminal via ratatui
//!
//! Architecture: the event loop builds immutable Snapshots;
//! the render actor draws them. No state mutations, no blocking
//! I/O, no caching — pure functions from Snapshot to Frame.
//!
//! DESIGN SYSTEM RULE: all colors, glyphs, and styles come from
//! crate::theme only. No literals, no hardcoded values.

use ratatui::{
    layout::{Constraint, Margin},
    style::Style,
    Frame,
};
use runie_core::Snapshot;

use crate::theme::{color_bg, set_current_theme};

mod hints;
mod input;
mod layout;
pub(crate) mod messages;
mod mouse;
mod render_lines;
mod scroll;

pub use input::count_input_lines;
pub use render_lines::element_line_count;
pub use scroll::render_scrollbar;

pub(crate) use hints::parse_hint_spans;
pub(crate) use layout::hstack;
pub(crate) use messages::estimate_element_tokens;

/// Draw a Snapshot to the terminal. Pure function — no mutable state.
pub fn draw_snapshot(f: &mut Frame, snap: &Snapshot) {
    set_current_theme(&snap.theme_name);
    let full_area = f.area();
    f.buffer_mut()
        .set_style(full_area, Style::default().bg(color_bg()));
    let margin = if full_area.width > 20 && full_area.height > 10 {
        Margin::new(1, 1)
    } else {
        Margin::new(0, 0)
    };
    let area = full_area.inner(margin);
    let input_lines = count_input_lines(&snap.input);
    let input_height = (input_lines + 2).min(10) as u16;
    let c = layout::vstack(
        area,
        &[
            Constraint::Min(3),
            Constraint::Length(1), // empty margin above status
            Constraint::Length(1), // status
            Constraint::Length(input_height),
            Constraint::Length(1),
            Constraint::Length(1),
        ],
    );
    messages::render_messages(f, snap, c[0]);
    // c[1] is the empty margin line — no rendering needed
    crate::status_bar::render(f, snap, c[2]);
    input::input(f, snap, c[3]);
    hints::hints(f, snap, c[5]);
    crate::popups::path_suggestions(f, snap);
    crate::popups::panel::panel_dialog(f, snap);
}

/// Legacy entry point for code that still builds AppState directly.
pub fn view(f: &mut Frame, state: &mut runie_core::AppState) {
    state.ensure_fresh();
    // Record the message viewport height so vim nav mode `j`/`k` and
    // arrow keys can compute element-level jumps. The messages area is
    // the top chunk of the vertical stack; approximate it as the full
    // area minus the input box, status, and margins.
    let full = f.area();
    let messages_height = full.height.saturating_sub(8).max(3);
    state.set_last_visible_height(messages_height);
    state.set_last_content_width(full.width);
    let snap = state.snapshot();
    draw_snapshot(f, &snap);
}
