//! 3-region chat layout: scrollback (top, grows) + prompt (middle, fixed) + status (bottom, 1 row).

use ratatui::layout::Rect;

pub const STATUS_HEIGHT: u16 = 1;
pub const PROMPT_HEIGHT: u16 = 1;
pub const HEADER_HEIGHT: u16 = 0; // minimal mode: no header line

#[derive(Debug, Clone, Copy)]
pub struct ChatLayout {
    pub header: Rect,
    pub scrollback: Rect,
    pub prompt: Rect,
    pub status: Rect,
}

pub fn chat_layout(area: Rect) -> ChatLayout {
    let status = Rect {
        x: area.x,
        y: area.y + area.height.saturating_sub(STATUS_HEIGHT),
        width: area.width,
        height: STATUS_HEIGHT,
    };
    let prompt = Rect {
        x: area.x,
        y: status.y.saturating_sub(PROMPT_HEIGHT),
        width: area.width,
        height: PROMPT_HEIGHT,
    };
    let header = Rect {
        x: area.x,
        y: area.y,
        width: area.width,
        height: HEADER_HEIGHT,
    };
    let scrollback = Rect {
        x: area.x,
        y: area.y + HEADER_HEIGHT,
        width: area.width,
        height: prompt.y.saturating_sub(area.y + HEADER_HEIGHT),
    };
    ChatLayout { header, scrollback, prompt, status }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn splits_24x80_into_three_regions() {
        let area = Rect { x: 0, y: 0, width: 80, height: 24 };
        let layout = chat_layout(area);
        assert_eq!(layout.status.height, 1);
        assert_eq!(layout.header.height, HEADER_HEIGHT);
        assert_eq!(layout.prompt.height, PROMPT_HEIGHT);
        assert_eq!(
            layout.header.height
                + layout.scrollback.height
                + layout.prompt.height
                + layout.status.height,
            24
        );
    }
}