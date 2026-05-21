
    use super::*;
    use ratatui::buffer::Buffer;

    fn setup_theme() -> ThemeWrapper {
        ThemeWrapper::default()
    }

    // ──── Cursor Rendering Tests ────

    #[test]
    fn test_cursor_empty_first_line() {
        let theme = setup_theme();
        let input = InputBar {
            lines: vec![String::new()],
            cursor_line: 0,
            cursor_col: 0,
            ..Default::default()
        };
        let area = Rect::new(0, 0, 40, 5);
        let mut buf = Buffer::empty(area);
        input.render_ref(area, &mut buf, &theme);

        assert_eq!(buf.cell((1, 1)).unwrap().symbol(), "❯");
        assert_eq!(buf.cell((2, 1)).unwrap().symbol(), " ");
    }

    #[test]
    fn test_cursor_after_text() {
        let theme = setup_theme();
        let mut input = InputBar::default();
        input.insert_char('h');
        input.insert_char('i');
        let area = Rect::new(0, 0, 40, 5);
        let mut buf = Buffer::empty(area);
        input.render_ref(area, &mut buf, &theme);

        assert_eq!(buf.cell((3, 1)).unwrap().symbol(), "h");
        assert_eq!(buf.cell((4, 1)).unwrap().symbol(), "i");
    }

    #[test]
    fn test_cursor_in_middle_of_text() {
        let theme = setup_theme();
        let mut input = InputBar::default();
        input.insert_char('h');
        input.insert_char('i');
        input.move_cursor_left();
        let area = Rect::new(0, 0, 40, 5);
        let mut buf = Buffer::empty(area);
        input.render_ref(area, &mut buf, &theme);

        assert_eq!(buf.cell((3, 1)).unwrap().symbol(), "h");
        assert_eq!(buf.cell((4, 1)).unwrap().symbol(), "i");
    }

    #[test]
    fn test_cursor_color_matches_chevron() {
        let theme = setup_theme();
        let input = InputBar::default();
        let area = Rect::new(0, 0, 40, 5);
        let mut buf = Buffer::empty(area);
        input.render_ref(area, &mut buf, &theme);

        let chevron_cell = buf.cell((1, 1)).unwrap();
        assert_eq!(chevron_cell.symbol(), "❯");
    }

    #[test]
    fn test_cursor_on_second_line() {
        let theme = setup_theme();
        let mut input = InputBar::default();
        input.insert_newline();
        input.insert_char('x');
        let area = Rect::new(0, 0, 40, 5);
        let mut buf = Buffer::empty(area);
        input.render_ref(area, &mut buf, &theme);

        assert_eq!(buf.cell((1, 2)).unwrap().symbol(), " ");
        assert_eq!(buf.cell((2, 2)).unwrap().symbol(), " ");
        assert_eq!(buf.cell((3, 2)).unwrap().symbol(), "x");
    }

    #[test]
    fn test_cursor_at_end_of_long_text() {
        let theme = setup_theme();
        let mut input = InputBar::default();
        for _ in 0..10 {
            input.insert_char('a');
        }
        let area = Rect::new(0, 0, 40, 5);
        let mut buf = Buffer::empty(area);
        input.render_ref(area, &mut buf, &theme);

        assert_eq!(buf.cell((4, 1)).unwrap().symbol(), "a");
    }

    // ──── Input Editing Tests ────

    #[test]
    fn test_insert_char() {
        let mut input = InputBar::default();
        input.insert_char('h');
        input.insert_char('i');
        assert_eq!(input.lines[0], "hi");
        assert_eq!(input.cursor_col, 2);
    }

    #[test]
    fn test_insert_newline() {
        let mut input = InputBar::default();
        input.insert_char('h');
        input.insert_char('i');
        input.insert_newline();
        assert_eq!(input.lines.len(), 2);
        assert_eq!(input.lines[0], "hi");
        assert_eq!(input.lines[1], "");
        assert_eq!(input.cursor_line, 1);
        assert_eq!(input.cursor_col, 0);
    }

    #[test]
    fn test_backspace_in_middle() {
        let mut input = InputBar::default();
        input.insert_char('h');
        input.insert_char('i');
        input.move_cursor_left();
        input.backspace();
        assert_eq!(input.lines[0], "i");
        assert_eq!(input.cursor_col, 0);
    }

    #[test]
    fn test_backspace_at_line_start() {
        let mut input = InputBar::default();
        input.insert_char('h');
        input.insert_char('i');
        input.insert_newline();
        input.backspace();
        assert_eq!(input.lines.len(), 1);
        assert_eq!(input.lines[0], "hi");
        assert_eq!(input.cursor_line, 0);
        assert_eq!(input.cursor_col, 2);
    }

    #[test]
    fn test_delete_word_backward() {
        let mut input = InputBar::default();
        for ch in "hello world".chars() {
            input.insert_char(ch);
        }
        input.delete_word_backward();
        assert_eq!(input.lines[0], "hello ");
        assert_eq!(input.cursor_col, 6);
    }

    #[test]
    fn test_delete_to_start() {
        let mut input = InputBar::default();
        input.insert_char('h');
        input.insert_char('i');
        input.move_cursor_to_start();
        input.delete_to_start();
        assert_eq!(input.lines[0], "hi");
        assert_eq!(input.cursor_col, 0);
    }

    #[test]
    fn test_delete_to_end() {
        let mut input = InputBar::default();
        input.insert_char('h');
        input.insert_char('i');
        input.move_cursor_left();
        input.delete_to_end();
        assert_eq!(input.lines[0], "h");
        assert_eq!(input.cursor_col, 1);
    }

    #[test]
    fn test_delete_forward() {
        let mut input = InputBar::default();
        input.insert_char('h');
        input.insert_char('i');
        input.move_cursor_left();
        input.delete_forward();
        assert_eq!(input.lines[0], "h");
    }

    // ──── Cursor Navigation Tests ────

    #[test]
    fn test_move_cursor_left() {
        let mut input = InputBar::default();
        input.insert_char('h');
        input.insert_char('i');
        input.move_cursor_left();
        assert_eq!(input.cursor_col, 1);
    }


    #[test]
    fn test_move_cursor_to_start_multi_line() {
        let mut input = InputBar::default();
        input.insert_char('h');
        input.insert_char('i');
        input.insert_newline();
        input.insert_char('t');
        input.insert_char('h');
        input.insert_char('e');
        input.insert_char('r');
        input.insert_char('e');
        input.move_cursor_to_start();
        assert_eq!(input.cursor_line, 1);
        assert_eq!(input.cursor_col, 0);
    }

    #[test]
    fn test_move_cursor_to_end_multi_line() {
        let mut input = InputBar::default();
        input.insert_char('h');
        input.insert_char('i');
        input.insert_newline();
        input.insert_char('t');
        input.move_cursor_to_start();
        input.move_cursor_to_end();
        assert_eq!(input.cursor_line, 1);
        assert_eq!(input.cursor_col, 1);
    }
