#[cfg(test)]
mod tests {
    use super::super::*;
    use ratatui::buffer::Buffer;
    use ratatui::layout::Rect;
    use ratatui::style::Style;

    #[test]
    fn test_menu_item_no_selected_indicator() {
        let mut buf = Buffer::empty(Rect::new(0, 0, 80, 10));
        draw_menu_item(
            "New worktree",
            "ctrl-w",
            10,
            5,
            60,
            &mut buf,
            Style::default(),
            Style::default(),
            Style::default(),
        );

        let line = buf
            .content
            .iter()
            .map(|c| c.symbol())
            .collect::<String>();
        assert!(!line.contains('▸'));
        assert!(!line.contains('❯'));
    }

    #[test]
    fn test_menu_item_hint_no_parens() {
        let mut buf = Buffer::empty(Rect::new(0, 0, 80, 10));
        draw_menu_item(
            "New worktree",
            "ctrl-w",
            10,
            5,
            60,
            &mut buf,
            Style::default(),
            Style::default(),
            Style::default(),
        );

        let line = buf
            .content
            .iter()
            .map(|c| c.symbol())
            .collect::<String>();
        assert!(line.contains("ctrl-w"));
        assert!(!line.contains("(ctrl-w)"));
    }

    #[test]
    fn test_menu_item_no_description() {
        let mut buf = Buffer::empty(Rect::new(0, 0, 80, 10));
        draw_menu_item(
            "New worktree",
            "ctrl-w",
            10,
            5,
            60,
            &mut buf,
            Style::default(),
            Style::default(),
            Style::default(),
        );

        let line = buf
            .content
            .iter()
            .map(|c| c.symbol())
            .collect::<String>();
        assert!(!line.contains("Start a parallel"));
    }

    #[test]
    fn test_divider_uses_correct_char() {
        let mut buf = Buffer::empty(Rect::new(0, 0, 80, 3));
        draw_divider(10, 1, 60, &mut buf, Style::default());

        let line = buf
            .content
            .iter()
            .map(|c| c.symbol())
            .collect::<String>();
        assert!(line.contains('─'));
    }
}
