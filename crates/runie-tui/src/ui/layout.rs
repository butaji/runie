//! Layout helpers for the main TUI view.

use ratatui::layout::{Constraint, Direction, Layout, Rect};

pub(crate) fn vstack(area: Rect, heights: &[Constraint]) -> Vec<Rect> {
    Layout::default()
        .direction(Direction::Vertical)
        .constraints(heights)
        .split(area)
        .to_vec()
}

pub(crate) fn hstack(area: Rect, widths: &[Constraint]) -> Vec<Rect> {
    Layout::default()
        .direction(Direction::Horizontal)
        .constraints(widths)
        .split(area)
        .to_vec()
}
