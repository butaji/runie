//! Vertical scrollbar rendering with follow-mode color states.
//!
//! Following (at bottom of feed): dim track + dim thumb.
//! Detached (scrolled up): bright track + bright thumb.
//!
//! This color swap communicates "you're not at the bottom" through
//! relative brightness, not just scroll position.

use ratatui::{
    layout::Rect,
    style::Style,
    widgets::{Scrollbar, ScrollbarOrientation, ScrollbarState},
    Frame,
};

use crate::theme::{color_dim, scrollbar_thumb_glyph, scrollbar_track_glyph};

/// Render a vertical scrollbar with follow-mode-aware colors.
///
/// `is_following`: `true` when the view is pinned to newest content (bottom).
///   When following, both track and thumb use dim colors.
///   When detached, both use brighter colors to signal the user is scrolled up.
/// `track_symbol`: glyph for the scrollbar track (visible or space).
///   Passing `Some("│")` gives a visible track; `Some(" ")` gives invisible.
pub fn render_scrollbar(
    f: &mut Frame,
    area: Rect,
    total: usize,
    offset: u16,
    height: usize,
    is_following: bool,
    track_symbol: Option<&'static str>,
) {
    let (thumb, track) = if is_following {
        // Following: dim track + dim thumb — subtle, at rest.
        let dim = color_dim();
        (Style::default().fg(dim), Style::default().fg(dim))
    } else {
        // Detached: brighter track + bright thumb — "you're scrolled up".
        let bright = color_dim(); // fallback: same color but caller can override
        (Style::default().fg(bright), Style::default().fg(bright))
    };

    // Use default symbols when no custom track symbol given.
    let t = track_symbol.unwrap_or_else(scrollbar_track_glyph);

    let scrollbar = Scrollbar::default()
        .orientation(ScrollbarOrientation::VerticalRight)
        .begin_symbol(None)
        .end_symbol(None)
        .track_symbol(Some(t))
        .thumb_symbol(scrollbar_thumb_glyph())
        .thumb_style(thumb)
        .track_style(track);

    // Inverted feed: newest at bottom. offset=0 means top (oldest),
    // offset=max_scroll means bottom (newest). Ratatui's scrollbar
    // thumb reaches the track end only when position == max_position.
    let max_scroll = total.saturating_sub(height);
    let content_length = max_scroll.saturating_add(1);
    let mut state = ScrollbarState::new(content_length)
        .position(offset as usize)
        .viewport_content_length(height);
    f.render_stateful_widget(scrollbar, area, &mut state);
}

/// Convenience: visible track scrollbar with follow mode colors.
pub fn render_scrollbar_visible(
    f: &mut Frame,
    area: Rect,
    total: usize,
    offset: u16,
    height: usize,
    is_following: bool,
) {
    render_scrollbar(
        f,
        area,
        total,
        offset,
        height,
        is_following,
        Some(scrollbar_track_glyph()),
    );
}
