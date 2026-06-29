//! Tests that row-to-element mapping stays aligned with Ratatui's wrap pass.

use std::sync::Arc;

use ratatui::widgets::{Paragraph, Wrap};
use ratatui::{backend::TestBackend, Terminal};
use runie_core::Snapshot;
use runie_core::{
    view::{Post, PostKind},
    Element,
};

use crate::ui::messages::build_lines_with_mapping;

fn snapshot_with_element(element: Element, width: u16) -> Snapshot {
    Snapshot {
        elements: Arc::new([element]),
        line_counts: Arc::new([1]),
        total_lines: 1,
        last_visible_height: 30,
        content_width: width,
        posts: Arc::new([Post {
            index: 0,
            start: 0,
            end: 1,
            kind: PostKind::AgentResponse,
            expanded: true,
        }]),
        ..Snapshot::default()
    }
}

fn long_code_element() -> Element {
    Element::agent(
        "```rust\n\
         let very_long = \"this is a very long code line that wraps in a narrow viewport\";\n\
         ```",
    )
    .at(0.0)
}

#[test]
fn row_to_element_len_equals_visible_rows() {
    let _lock = crate::theme::test_lock();
    let width = 40u16;
    let snap = snapshot_with_element(long_code_element(), width);
    let (lines, mapping) = build_lines_with_mapping(&snap, width);
    let visible_rows = Paragraph::new(lines.as_slice())
        .wrap(Wrap { trim: false })
        .line_count(width);
    assert_eq!(
        mapping.len(),
        visible_rows,
        "row_to_element should contain one entry per visible row after wrap"
    );
}

#[test]
fn vim_nav_selects_all_rows_of_wrapped_code_block() {
    let _lock = crate::theme::test_lock();
    let width = 40u16;
    let mut snap = snapshot_with_element(long_code_element(), width);
    snap.vim_nav_mode = true;
    snap.selected_post = Some(0);

    let (_, mapping) = build_lines_with_mapping(&snap, width);
    let expected_rows = mapping.iter().filter(|&&idx| idx == 0).count();

    let backend = TestBackend::new(width, 30);
    let mut terminal = Terminal::new(backend).unwrap();
    terminal
        .draw(|f| crate::ui::messages::render_messages(f, &snap, f.area()))
        .unwrap();

    let buf = terminal.backend().buffer();
    let accent = crate::theme::color_accent();
    let bracket_rows: Vec<u16> = (0..buf.area().height)
        .filter(|&y| {
            let cell = &buf[(0, y)];
            cell.symbol() == "▎" && cell.style().fg == Some(accent)
        })
        .collect();

    assert_eq!(
        bracket_rows.len(),
        expected_rows,
        "bracket should cover every visible row of the wrapped code block"
    );
}
