//! Tests for the paint module.

#[cfg(test)]
mod tests {
    use crate::paint::{col, text, row, Node, paint};
    use ratatui::backend::TestBackend;
    use ratatui::Terminal;

    fn paint_to_text(node: &Node, w: u16, h: u16) -> String {
        let backend = TestBackend::new(w, h);
        let mut terminal = Terminal::new(backend).unwrap();
        terminal.draw(|frame| {
            paint(node, frame.area(), frame.buffer_mut(), &crate::theme::ThemeColors::default_for_test());
        }).unwrap();
        let mut s = String::new();
        let buf = terminal.backend().buffer().clone();
        for y in 0..buf.area.height {
            for x in 0..buf.area.width {
                s.push_str(buf.cell((x, y)).unwrap().symbol());
            }
            s.push('\n');
        }
        s
    }

    #[test]
    fn paints_text() {
        let s = paint_to_text(&text("hi").dim(), 5, 1);
        assert!(s.starts_with("hi"));
    }

    #[test]
    fn paints_row_with_gap() {
        let s = paint_to_text(
            &row(vec![Node::T(text("ab")), Node::T(text("cd"))])
                .gap(1)
                .build(),
            6, 1,
        );
        assert!(s.contains("ab"));
        assert!(s.contains("cd"));
    }

    #[test]
    fn fill_takes_remaining() {
        let s = paint_to_text(
            &row(vec![Node::T(text("ab")), Node::Fill]).build(),
            5, 1,
        );
        assert!(s.starts_with("ab"));
    }

    #[test]
    fn col_stacks() {
        let s = paint_to_text(
            &col(vec![Node::T(text("a")), Node::T(text("b"))]).build(),
            1, 2,
        );
        assert_eq!(s.lines().next().unwrap(), "a");
        assert_eq!(s.lines().nth(1).unwrap(), "b");
    }
}
