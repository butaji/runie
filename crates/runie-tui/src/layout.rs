//! 3-region chat layout: scrollback (top, grows) + prompt (middle, fixed) + status (bottom, 1 row).

use crate::view::{chat_view, LayoutDirection, LayoutEntry, LayoutSize, Slot, StackLayout};
use ratatui::layout::Rect;

pub const STATUS_HEIGHT: u16 = 1;
pub const PROMPT_HEIGHT: u16 = 3;
pub const HEADER_HEIGHT: u16 = 1;
pub const BOTTOM_MARGIN: u16 = 1;
// Grok's full-mode LayoutConfig defaults (xai-grok-pager/appearance):
// two terminal columns outside the agent view on each side.
pub const OUTER_HPAD_LEFT: u16 = 2;
pub const OUTER_HPAD_RIGHT: u16 = 2;
// Scrollback HorizontalLayout defaults: one accent rail, two columns before
// content, and one trailing column after content.
pub const SCROLLBACK_ACCENT_WIDTH: u16 = 1;
pub const SCROLLBACK_BLOCK_PAD_LEFT: u16 = 2;
pub const SCROLLBACK_BLOCK_PAD_RIGHT: u16 = 1;
/// Compact Grok scrollback keeps a small lead above the absolute tail.
pub const COMPACT_SCROLL_LEAD_ROWS: usize = 2;
pub const COMPACT_SCROLL_OVERFLOW_LEAD_ROWS: usize = 8;
pub const COMPACT_SCROLL_OVERFLOW_THRESHOLD: usize = 8;
pub const GROK_SHORT_TERMINAL_ROWS: u16 = 16;
pub const GROK_AUTO_COMPACT_MAX_ROWS: u16 = 20;
pub const GROK_SMALL_SCREEN_TIP_MAX_ROWS: u16 = 30;

/// Grok derives compact mode from full terminal height; an unmeasured height
/// must not force compact mode.
pub const fn grok_effective_compact(user_compact: bool, terminal_rows: u16) -> bool {
    user_compact || (terminal_rows > 0 && terminal_rows <= GROK_AUTO_COMPACT_MAX_ROWS)
}

/// Grok keeps the compact-mode tip in the small-screen band immediately
/// above auto-compact. The predicate is pure so event/replay renderers can
/// make the same decision as the live terminal renderer.
pub const fn grok_small_screen_tip_visible(terminal_rows: u16) -> bool {
    terminal_rows > GROK_AUTO_COMPACT_MAX_ROWS && terminal_rows <= GROK_SMALL_SCREEN_TIP_MAX_ROWS
}

#[derive(Debug, Clone, Copy)]
pub struct ChatLayout {
    pub header: Rect,
    pub scrollback: Rect,
    pub prompt: Rect,
    pub status: Rect,
    pub footer_badge: Rect,
}

/// The declarative surface consumed by the terminal layout adapter.
pub fn chat_elements() -> crate::view::Element {
    chat_view()
}

pub fn chat_layout(area: Rect) -> ChatLayout {
    chat_layout_with_prompt_height(area, PROMPT_HEIGHT)
}

#[allow(
    clippy::too_many_lines,
    reason = "the layout reducer keeps all dependent regions visible together"
)]
pub fn chat_layout_with_prompt_height(area: Rect, prompt_height: u16) -> ChatLayout {
    debug_assert_eq!(
        chat_elements().slots().collect::<Vec<_>>(),
        vec![
            Slot::Header,
            Slot::Scrollback,
            Slot::Prompt,
            Slot::Status,
            Slot::FooterBadge,
        ]
    );
    // Grok reserves one terminal row above the chat and two columns on each
    // side for its full-mode chrome. The transcript rail itself is inside the
    // scrollback content projection.
    let inner = Rect {
        x: area.x.saturating_add(OUTER_HPAD_LEFT),
        y: area.y.saturating_add(1),
        width: area
            .width
            .saturating_sub(OUTER_HPAD_LEFT + OUTER_HPAD_RIGHT),
        height: area.height.saturating_sub(1),
    };
    let [header_height, scrollback_height, prompt_height] =
        chat_region_heights(inner.height, prompt_height);
    let header = Rect {
        x: inner.x,
        y: inner.y,
        width: inner.width,
        height: header_height,
    };
    let scrollback = Rect {
        x: inner.x,
        y: header.y + header.height,
        width: inner.width,
        height: scrollback_height,
    };
    let prompt = Rect {
        x: inner.x,
        y: scrollback.y + scrollback.height,
        width: inner.width,
        height: prompt_height,
    };
    let status = Rect {
        x: inner.x,
        y: prompt.y + prompt.height + BOTTOM_MARGIN,
        width: inner.width,
        height: STATUS_HEIGHT,
    };
    let footer_badge = Rect {
        x: inner.x,
        y: status.y + status.height,
        width: inner.width,
        height: BOTTOM_MARGIN.min(area.height),
    };
    ChatLayout {
        header,
        scrollback,
        prompt,
        status,
        footer_badge,
    }
}

fn chat_region_heights(inner_height: u16, prompt_height: u16) -> [u16; 3] {
    const ENTRIES: [LayoutEntry; 3] = [
        LayoutEntry {
            slot: Slot::Header,
            basis: LayoutSize::Fixed(HEADER_HEIGHT),
            grow: 0,
            shrink: 0,
            min_size: HEADER_HEIGHT,
            max_size: Some(HEADER_HEIGHT),
        },
        LayoutEntry::grow(Slot::Scrollback, 0),
        LayoutEntry {
            slot: Slot::Prompt,
            basis: LayoutSize::Fixed(PROMPT_HEIGHT),
            grow: 0,
            shrink: 0,
            min_size: PROMPT_HEIGHT,
            max_size: None,
        },
    ];
    let mut entries = ENTRIES;
    entries[2].basis = LayoutSize::Fixed(prompt_height.max(PROMPT_HEIGHT));
    let layout = StackLayout {
        direction: LayoutDirection::Vertical,
        gap: 0,
        entries: &entries,
    };
    let reserved = STATUS_HEIGHT + BOTTOM_MARGIN + BOTTOM_MARGIN;
    let sizes = layout.allocate(
        &[HEADER_HEIGHT, 0, prompt_height],
        Some(inner_height.saturating_sub(reserved)),
    );
    [sizes[0], sizes[1], sizes[2]]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn grok_compact_mode_uses_full_terminal_height() {
        assert!(!grok_effective_compact(false, 0));
        assert!(grok_effective_compact(false, GROK_SHORT_TERMINAL_ROWS));
        assert!(grok_effective_compact(false, GROK_AUTO_COMPACT_MAX_ROWS));
        assert!(!grok_effective_compact(
            false,
            GROK_AUTO_COMPACT_MAX_ROWS + 1
        ));
        assert!(grok_effective_compact(true, 80));
    }

    #[test]
    fn grok_small_screen_tip_uses_the_pre_compact_band() {
        assert!(!grok_small_screen_tip_visible(GROK_AUTO_COMPACT_MAX_ROWS));
        assert!(grok_small_screen_tip_visible(
            GROK_AUTO_COMPACT_MAX_ROWS + 1
        ));
        assert!(grok_small_screen_tip_visible(
            GROK_SMALL_SCREEN_TIP_MAX_ROWS
        ));
        assert!(!grok_small_screen_tip_visible(
            GROK_SMALL_SCREEN_TIP_MAX_ROWS + 1
        ));
    }

    #[test]
    fn splits_24x80_into_three_regions() {
        let area = Rect {
            x: 0,
            y: 0,
            width: 80,
            height: 24,
        };
        let layout = chat_layout(area);
        assert_eq!(layout.status.height, 1);
        assert_eq!(layout.header.height, HEADER_HEIGHT);
        assert_eq!(layout.prompt.height, PROMPT_HEIGHT);
        assert_eq!(
            layout.header.height
                + layout.scrollback.height
                + layout.prompt.height
                + layout.status.height,
            21
        );
    }

    #[test]
    fn narrow_layout_preserves_prompt_and_status_rows() {
        let area = Rect {
            x: 0,
            y: 0,
            width: 40,
            height: 12,
        };
        let layout = chat_layout(area);
        assert_eq!(layout.scrollback.width, 36);
        assert_eq!(layout.prompt.height, PROMPT_HEIGHT);
        assert_eq!(layout.status.height, STATUS_HEIGHT);
        assert_eq!(
            layout.prompt.y + layout.prompt.height + BOTTOM_MARGIN,
            layout.status.y
        );
    }

    #[test]
    fn grok_full_mode_uses_source_layout_chrome() {
        assert_eq!(OUTER_HPAD_LEFT, 2);
        assert_eq!(OUTER_HPAD_RIGHT, 2);
        assert_eq!(SCROLLBACK_ACCENT_WIDTH, 1);
        assert_eq!(SCROLLBACK_BLOCK_PAD_LEFT, 2);
        assert_eq!(SCROLLBACK_BLOCK_PAD_RIGHT, 1);
    }
}
