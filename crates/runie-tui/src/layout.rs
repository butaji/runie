//! 3-region chat layout: scrollback (top, grows) + prompt (middle, fixed) + status (bottom, 1 row).

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

#[derive(Debug, Clone, Copy)]
pub struct ChatLayout {
    pub header: Rect,
    pub scrollback: Rect,
    pub prompt: Rect,
    pub status: Rect,
    pub footer_badge: Rect,
}

pub fn chat_layout(area: Rect) -> ChatLayout {
    chat_layout_with_prompt_height(area, PROMPT_HEIGHT)
}

#[allow(
    clippy::too_many_lines,
    reason = "the layout reducer keeps all dependent regions visible together"
)]
pub fn chat_layout_with_prompt_height(area: Rect, prompt_height: u16) -> ChatLayout {
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
    let status = Rect {
        x: inner.x,
        y: inner.y + inner.height.saturating_sub(STATUS_HEIGHT + BOTTOM_MARGIN),
        width: inner.width,
        height: STATUS_HEIGHT,
    };
    let footer_badge = Rect {
        x: inner.x,
        y: status.y + STATUS_HEIGHT,
        width: inner.width,
        height: BOTTOM_MARGIN.min(area.height),
    };
    let available_prompt = status.y.saturating_sub(inner.y + HEADER_HEIGHT);
    let prompt_height = prompt_height
        .max(PROMPT_HEIGHT)
        .min(available_prompt.max(PROMPT_HEIGHT));
    let prompt = Rect {
        x: inner.x,
        y: status.y.saturating_sub(prompt_height + BOTTOM_MARGIN),
        width: inner.width,
        height: prompt_height,
    };
    let header = Rect {
        x: inner.x,
        y: inner.y,
        width: inner.width,
        height: HEADER_HEIGHT,
    };
    let scrollback = Rect {
        x: inner.x,
        y: inner.y + HEADER_HEIGHT,
        width: inner.width,
        height: prompt.y.saturating_sub(inner.y + HEADER_HEIGHT),
    };
    ChatLayout {
        header,
        scrollback,
        prompt,
        status,
        footer_badge,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

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
