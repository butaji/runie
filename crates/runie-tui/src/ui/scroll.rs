//! Vertical scrollbar rendering.

use ratatui::{
    layout::Rect,
    widgets::{Scrollbar, ScrollbarOrientation, ScrollbarState},
    Frame,
};

use crate::theme::{style_scrollbar, SCROLLBAR_THUMB, SCROLLBAR_TRACK};

pub fn render_scrollbar(f: &mut Frame, area: Rect, total: usize, offset: u16, height: usize) {
    let scrollbar = Scrollbar::default()
        .orientation(ScrollbarOrientation::VerticalRight)
        .begin_symbol(None)
        .end_symbol(None)
        .track_symbol(Some(SCROLLBAR_TRACK))
        .thumb_symbol(SCROLLBAR_THUMB)
        .style(style_scrollbar());

    // Inverted feed: newest at bottom. offset=0 means top (oldest),
    // offset=max_scroll means bottom (newest). Ratatui's scrollbar
    // thumb reaches the track end only when position == max_position.
    // We achieve this by setting content_length = max_scroll + 1 so
    // max_position = max_scroll, matching our offset range exactly.
    let max_scroll = total.saturating_sub(height);
    let content_length = max_scroll.saturating_add(1);
    let mut state = ScrollbarState::new(content_length)
        .position(offset as usize)
        .viewport_content_length(height);
    f.render_stateful_widget(scrollbar, area, &mut state);
}
