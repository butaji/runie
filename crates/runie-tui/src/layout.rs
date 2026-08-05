//! 3-region chat layout: scrollback (top, grows) + prompt (middle, fixed) + status (bottom, 1 row).

use ratatui::layout::Rect;

pub const STATUS_HEIGHT: u16 = 1;
pub const PROMPT_HEIGHT: u16 = 3;
pub const HEADER_HEIGHT: u16 = 1;
pub const BOTTOM_MARGIN: u16 = 1;

#[derive(Debug, Clone, Copy)]
pub struct ChatLayout {
    pub header: Rect,
    pub scrollback: Rect,
    pub prompt: Rect,
    pub status: Rect,
}

pub fn chat_layout(area: Rect) -> ChatLayout {
    // Grok reserves one terminal row above the chat and two columns on each
    // side for its full-mode chrome.
    let inner = Rect {
        x: area.x.saturating_add(2),
        y: area.y.saturating_add(1),
        width: area.width.saturating_sub(4),
        height: area.height.saturating_sub(1),
    };
    let status = Rect {
        x: inner.x,
        y: inner.y + inner.height.saturating_sub(STATUS_HEIGHT + BOTTOM_MARGIN),
        width: inner.width,
        height: STATUS_HEIGHT,
    };
    let prompt = Rect {
        x: inner.x,
        y: status.y.saturating_sub(PROMPT_HEIGHT + BOTTOM_MARGIN),
        width: inner.width,
        height: PROMPT_HEIGHT,
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
}
