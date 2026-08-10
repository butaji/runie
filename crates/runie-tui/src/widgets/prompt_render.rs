use super::appearance;
use ratatui::{buffer::Buffer, layout::Rect, style::Style, text::Span};
use runie_core::types::ThemeKind;
pub(super) fn caption_spans(caption: &str, theme: ThemeKind) -> Vec<Span<'static>> {
    let mut spans = vec![Span::raw(" ")];
    for (index, part) in caption.split(" · ").enumerate() {
        if index > 0 {
            spans.push(Span::styled(
                " · ",
                appearance::header_path_style_for(theme),
            ));
        }
        spans.push(Span::styled(
            part.to_owned(),
            if index == 0 {
                appearance::model_caption_style_for(theme)
            } else {
                appearance::muted_style_for(theme)
            },
        ));
    }
    spans.push(Span::raw(" "));
    spans
}

pub(super) fn draw_prompt_border(area: Rect, buf: &mut Buffer, border: Style) {
    let top = area.y;
    let bottom = area.y + area.height.saturating_sub(1);
    let right = area.x + area.width.saturating_sub(1);
    for x in area.x..area.x + area.width {
        set_border_cell(
            buf,
            x,
            top,
            if x == area.x {
                '╭'
            } else if x == right {
                '╮'
            } else {
                '─'
            },
            border,
        );
        if bottom != top {
            set_border_cell(
                buf,
                x,
                bottom,
                if x == area.x {
                    '╰'
                } else if x == right {
                    '╯'
                } else {
                    '─'
                },
                border,
            );
        }
    }
    for y in top.saturating_add(1)..bottom {
        set_border_cell(buf, area.x, y, '│', border);
        set_border_cell(buf, right, y, '│', border);
    }
}

pub(super) fn set_border_cell(buf: &mut Buffer, x: u16, y: u16, character: char, style: Style) {
    if let Some(cell) = buf.cell_mut((x, y)) {
        cell.set_char(character);
        cell.set_style(style);
    }
}
