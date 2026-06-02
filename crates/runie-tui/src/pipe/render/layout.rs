//! RenderPipe layout calculations.

use ratatui::layout::{Constraint, Layout, Rect};

const SIDEBAR_WIDTH: u16 = 28;

pub fn input_bar_height(state: &crate::tui::AppState) -> u16 {
    // No attachments yet in pipe render path
    crate::components::input_bar::input_bar_height(&state.textarea, false)
}

pub fn layout_main(padded: Rect, show_status: bool, input_h: u16) -> [Rect; 5] {
    let constraints = [
        Constraint::Length(2),        // topbar + padding
        Constraint::Min(1),           // feed
        Constraint::Length(1),       // global_tags
        Constraint::Length(input_h),  // input
        if show_status { Constraint::Length(1) } else { Constraint::Length(0) }, // hotkeys
    ];
    Layout::vertical(constraints).areas(padded)
}
