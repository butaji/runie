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

    #[test]
    fn test_tip_text_constant() {
        // Verify the tip text constant matches the expected Grok spec
        assert_eq!(TIP_TEXT, "Tip: Press Ctrl-W to start a parallel task in its own worktree.");
    }

    #[test]
    fn test_tip_text_renders() {
        let mut buf = Buffer::empty(Rect::new(0, 0, 120, 30));
        // Render tip at position 0, 25 with full area width
        render_tip(Rect::new(0, 0, 120, 30), 25, &mut buf, Style::default());

        // Check that tip text is rendered
        let content: String = buf
            .content
            .iter()
            .map(|c| c.symbol())
            .collect::<String>();
        assert!(content.contains("Tip:"));
        assert!(content.contains("Ctrl-W"));
    }
}
