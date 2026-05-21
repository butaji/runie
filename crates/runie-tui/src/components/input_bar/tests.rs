
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
    fn test_move_cursor_left_across_lines() {
        let mut input = InputBar::default();
        input.insert_char('h');
        input.insert_newline();
        input.move_cursor_left();
        assert_eq!(input.cursor_line, 0);
        assert_eq!(input.cursor_col, 1);
    }

    #[test]
    fn test_move_cursor_right() {
        let mut input = InputBar::default();
        input.insert_char('h');
        input.move_cursor_to_start();
        input.move_cursor_right();
        assert_eq!(input.cursor_col, 1);
    }

    #[test]
    fn test_move_cursor_right_across_lines() {
        let mut input = InputBar::default();
        input.insert_char('h');
        input.insert_newline();
        input.move_cursor_right();
        assert_eq!(input.cursor_line, 1);
        assert_eq!(input.cursor_col, 0);
    }

    #[test]
    fn test_move_cursor_up() {
        let mut input = InputBar::default();
        input.insert_char('h');
        input.insert_newline();
        input.insert_char('i');
        input.move_cursor_up();
        assert_eq!(input.cursor_line, 0);
        assert_eq!(input.cursor_col, 1);
    }

    #[test]
    fn test_move_cursor_down() {
        let mut input = InputBar::default();
        input.insert_char('h');
        input.insert_newline();
        input.insert_char('i');
        input.move_cursor_up();
        input.move_cursor_down();
        assert_eq!(input.cursor_line, 1);
        assert_eq!(input.cursor_col, 1);
    }

    #[test]
    fn test_move_cursor_to_start() {
        let mut input = InputBar::default();
        input.insert_char('h');
        input.insert_char('i');
        input.move_cursor_to_start();
        assert_eq!(input.cursor_col, 0);
    }

    #[test]
    fn test_move_cursor_to_end() {
        let mut input = InputBar::default();
        input.insert_char('h');
        input.insert_char('i');
        input.move_cursor_to_start();
        input.move_cursor_to_end();
        assert_eq!(input.cursor_col, 2);
    }

    // ──── Submit/Clear Tests ────

    #[test]
    fn test_submit() {
        let mut input = InputBar::default();
        input.insert_char('h');
        input.insert_char('i');
        input.insert_newline();
        input.insert_char('b');
        let result = input.submit();
        assert_eq!(result, "hi\nb");
        assert_eq!(input.lines, vec![String::new()]);
        assert_eq!(input.cursor_line, 0);
        assert_eq!(input.cursor_col, 0);
    }

    #[test]
    fn test_clear() {
        let mut input = InputBar::default();
        input.insert_char('h');
        input.insert_char('i');
        input.clear();
        assert_eq!(input.lines, vec![String::new()]);
        assert_eq!(input.cursor_line, 0);
        assert_eq!(input.cursor_col, 0);
    }

    // ──── Multi-line Tests ────

    #[test]
    fn test_multi_line_render() {
        let theme = setup_theme();
        let mut input = InputBar::default();
        input.insert_char('l');
        input.insert_char('i');
        input.insert_char('n');
        input.insert_char('e');
        input.insert_char('1');
        input.insert_newline();
        input.insert_char('l');
        input.insert_char('i');
        input.insert_char('n');
        input.insert_char('e');
        input.insert_char('2');

        let area = Rect::new(0, 0, 40, 6);
        let mut buf = Buffer::empty(area);
        input.render_ref(area, &mut buf, &theme);

        assert_eq!(buf.cell((1, 1)).unwrap().symbol(), "❯");
        assert_eq!(buf.cell((2, 1)).unwrap().symbol(), " ");
        assert_eq!(buf.cell((3, 1)).unwrap().symbol(), "l");

        assert_eq!(buf.cell((1, 2)).unwrap().symbol(), " ");
        assert_eq!(buf.cell((2, 2)).unwrap().symbol(), " ");
        assert_eq!(buf.cell((3, 2)).unwrap().symbol(), "l");
    }

    #[test]
    fn test_three_lines() {
        let theme = setup_theme();
        let mut input = InputBar::default();
        input.insert_newline();
        input.insert_newline();

        let area = Rect::new(0, 0, 40, 7);
        let mut buf = Buffer::empty(area);
        input.render_ref(area, &mut buf, &theme);

        assert_eq!(buf.cell((0, 1)).unwrap().symbol(), "│");
        assert_eq!(buf.cell((0, 2)).unwrap().symbol(), "│");
        assert_eq!(buf.cell((0, 3)).unwrap().symbol(), "│");
        assert_eq!(buf.cell((0, 4)).unwrap().symbol(), "╰");
    }

    // ──── Cursor Screen Position Tests ────

    #[test]
    fn test_cursor_screen_pos_empty() {
        let input = InputBar::default();
        let area = Rect::new(0, 0, 40, 5);
        let pos = input.cursor_screen_pos(area);
        assert_eq!(pos.x, 3);
        assert_eq!(pos.y, 1);
    }

    #[test]
    fn test_cursor_screen_pos_with_text() {
        let mut input = InputBar::default();
        input.insert_char('h');
        input.insert_char('i');
        let area = Rect::new(0, 0, 40, 5);
        let pos = input.cursor_screen_pos(area);
        assert_eq!(pos.x, 5);
        assert_eq!(pos.y, 1);
    }

    #[test]
    fn test_cursor_screen_pos_second_line() {
        let mut input = InputBar::default();
        input.insert_newline();
        input.insert_char('x');
        let area = Rect::new(0, 0, 40, 5);
        let pos = input.cursor_screen_pos(area);
        assert_eq!(pos.x, 4);
        assert_eq!(pos.y, 2);
    }

    // ──── delete_word_backward edge cases ────

    #[test]
    fn test_delete_word_backward_with_punctuation() {
        let mut input = InputBar::default();
        for ch in "hello world!".chars() {
            input.insert_char(ch);
        }
        input.delete_word_backward();
        assert_eq!(input.lines[0], "hello ");
        assert_eq!(input.cursor_col, 6);
    }

    #[test]
    fn test_delete_word_backward_punctuation_only() {
        let mut input = InputBar::default();
        for ch in "!!!".chars() {
            input.insert_char(ch);
        }
        input.delete_word_backward();
        assert_eq!(input.lines[0], "");
        assert_eq!(input.cursor_col, 0);
    }

    #[test]
    fn test_delete_word_backward_mixed_punctuation_words() {
        let mut input = InputBar::default();
        for ch in "foo-bar baz".chars() {
            input.insert_char(ch);
        }
        input.delete_word_backward();
        assert_eq!(input.lines[0], "foo-bar ");
        assert_eq!(input.cursor_col, 8);
    }

    #[test]
    fn test_delete_word_backward_at_start_of_line_joins() {
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
        input.delete_word_backward();
        assert_eq!(input.lines.len(), 1);
        assert_eq!(input.lines[0], "hithere");
        assert_eq!(input.cursor_line, 0);
        assert_eq!(input.cursor_col, 2);
    }

    #[test]
    fn test_delete_word_backward_multiple_whitespace() {
        let mut input = InputBar::default();
        for ch in "hello   ".chars() {
            input.insert_char(ch);
        }
        input.delete_word_backward();
        assert_eq!(input.lines[0], "");
        assert_eq!(input.cursor_col, 0);
    }

    #[test]
    fn test_delete_word_backward_single_word() {
        let mut input = InputBar::default();
        for ch in "hello".chars() {
            input.insert_char(ch);
        }
        input.delete_word_backward();
        assert_eq!(input.lines[0], "");
        assert_eq!(input.cursor_col, 0);
    }

    #[test]
    fn test_delete_word_backward_empty() {
        let mut input = InputBar::default();
        input.delete_word_backward();
        assert_eq!(input.lines[0], "");
        assert_eq!(input.cursor_col, 0);
    }

    // ──── delete_to_start edge cases ────

    #[test]
    fn test_delete_to_start_in_middle() {
        let mut input = InputBar::default();
        for ch in "hello".chars() {
            input.insert_char(ch);
        }
        input.move_cursor_left();
        input.move_cursor_left();
        input.delete_to_start();
        assert_eq!(input.lines[0], "lo");
        assert_eq!(input.cursor_col, 0);
    }

    #[test]
    fn test_delete_to_start_at_end() {
        let mut input = InputBar::default();
        for ch in "hello".chars() {
            input.insert_char(ch);
        }
        input.delete_to_start();
        assert_eq!(input.lines[0], "");
        assert_eq!(input.cursor_col, 0);
    }

    // ──── delete_to_end edge cases ────

    #[test]
    fn test_delete_to_end_at_start() {
        let mut input = InputBar::default();
        for ch in "hello".chars() {
            input.insert_char(ch);
        }
        input.move_cursor_to_start();
        input.delete_to_end();
        assert_eq!(input.lines[0], "");
        assert_eq!(input.cursor_col, 0);
    }

    #[test]
    fn test_delete_to_end_at_end() {
        let mut input = InputBar::default();
        for ch in "hello".chars() {
            input.insert_char(ch);
        }
        input.delete_to_end();
        assert_eq!(input.lines[0], "hello");
        assert_eq!(input.cursor_col, 5);
    }

    #[test]
    fn test_delete_to_end_multi_line_no_join() {
        let mut input = InputBar::default();
        input.insert_char('h');
        input.insert_char('i');
        input.insert_newline();
        input.insert_char('t');
        input.insert_char('h');
        input.insert_char('e');
        input.insert_char('r');
        input.insert_char('e');
        input.move_cursor_up();
        input.move_cursor_to_end();
        input.delete_to_end();
        assert_eq!(input.lines.len(), 2);
        assert_eq!(input.lines[0], "hi");
        assert_eq!(input.lines[1], "there");
    }

    // ──── delete_forward edge cases ────

    #[test]
    fn test_delete_forward_at_end_joins_lines() {
        let mut input = InputBar::default();
        input.insert_char('h');
        input.insert_char('i');
        input.insert_newline();
        input.insert_char('t');
        input.insert_char('h');
        input.insert_char('e');
        input.insert_char('r');
        input.insert_char('e');
        input.move_cursor_up();
        input.move_cursor_to_end();
        input.delete_forward();
        assert_eq!(input.lines.len(), 1);
        assert_eq!(input.lines[0], "hithere");
        assert_eq!(input.cursor_line, 0);
        assert_eq!(input.cursor_col, 2);
    }

    #[test]
    fn test_delete_forward_at_end_of_last_line() {
        let mut input = InputBar::default();
        for ch in "hello".chars() {
            input.insert_char(ch);
        }
        input.delete_forward();
        assert_eq!(input.lines[0], "hello");
        assert_eq!(input.cursor_col, 5);
    }

    #[test]
    fn test_delete_forward_in_middle() {
        let mut input = InputBar::default();
        for ch in "hello".chars() {
            input.insert_char(ch);
        }
        input.move_cursor_to_start();
        input.move_cursor_right();
        input.delete_forward();
        assert_eq!(input.lines[0], "hllo");
        assert_eq!(input.cursor_col, 1);
    }

    // ──── Cursor boundary tests ────

    #[test]
    fn test_move_cursor_left_at_start() {
        let mut input = InputBar::default();
        input.move_cursor_left();
        assert_eq!(input.cursor_line, 0);
        assert_eq!(input.cursor_col, 0);
    }

    #[test]
    fn test_move_cursor_right_at_end() {
        let mut input = InputBar::default();
        for ch in "hi".chars() {
            input.insert_char(ch);
        }
        input.move_cursor_right();
        assert_eq!(input.cursor_line, 0);
        assert_eq!(input.cursor_col, 2);
    }

    #[test]
    fn test_move_cursor_up_at_top() {
        let mut input = InputBar::default();
        input.move_cursor_up();
        assert_eq!(input.cursor_line, 0);
        assert_eq!(input.cursor_col, 0);
    }

    #[test]
    fn test_move_cursor_down_at_bottom() {
        let mut input = InputBar::default();
        input.move_cursor_down();
        assert_eq!(input.cursor_line, 0);
        assert_eq!(input.cursor_col, 0);
    }

    #[test]
    fn test_move_cursor_up_clamping() {
        let mut input = InputBar::default();
        for ch in "hello".chars() {
            input.insert_char(ch);
        }
        input.insert_newline();
        input.insert_char('h');
        input.insert_char('i');
        input.move_cursor_up();
        assert_eq!(input.cursor_line, 0);
        assert_eq!(input.cursor_col, 2);
    }

    #[test]
    fn test_move_cursor_down_clamping() {
        let mut input = InputBar::default();
        input.insert_char('h');
        input.insert_char('i');
        input.insert_newline();
        for ch in "hello".chars() {
            input.insert_char(ch);
        }
        input.move_cursor_up();
        input.move_cursor_down();
        assert_eq!(input.cursor_line, 1);
        assert_eq!(input.cursor_col, 2);
    }

    // ──── Multi-line backspace tests ────

    #[test]
    fn test_backspace_joins_multiple_lines() {
        let mut input = InputBar::default();
        input.insert_char('a');
        input.insert_newline();
        input.insert_char('b');
        input.insert_newline();
        input.insert_char('c');
        input.backspace();
        assert_eq!(input.lines.len(), 3);
        assert_eq!(input.lines[0], "a");
        assert_eq!(input.lines[1], "b");
        assert_eq!(input.lines[2], "");
        assert_eq!(input.cursor_line, 2);
        assert_eq!(input.cursor_col, 0);
    }

    #[test]
    fn test_backspace_at_start_of_line_joins() {
        let mut input = InputBar::default();
        input.insert_char('a');
        input.insert_newline();
        input.insert_char('b');
        input.move_cursor_to_start();
        input.backspace();
        assert_eq!(input.lines.len(), 1);
        assert_eq!(input.lines[0], "ab");
        assert_eq!(input.cursor_line, 0);
        assert_eq!(input.cursor_col, 1);
    }

    #[test]
    fn test_backspace_at_start_of_first_line() {
        let mut input = InputBar::default();
        input.backspace();
        assert_eq!(input.lines[0], "");
        assert_eq!(input.cursor_col, 0);
    }

    // ──── Ctrl+A/E on multi-line ────

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
