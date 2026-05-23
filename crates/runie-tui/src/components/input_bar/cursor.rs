use ratatui::layout::Rect;

use super::InputBar;

pub fn cursor_screen_pos(input: &InputBar, area: Rect) -> ratatui::layout::Position {
    cursor_screen_pos_from_fields(input.cursor_line, input.cursor_col, area)
}

pub fn cursor_screen_pos_from_fields(
    cursor_line: usize,
    cursor_col: usize,
    area: Rect,
) -> ratatui::layout::Position {
    let x = area.x + 1;
    let y = area.y + 1 + cursor_line as u16;
    let cursor_x = x + 2 + cursor_col as u16;
    ratatui::layout::Position::new(cursor_x, y)
}
