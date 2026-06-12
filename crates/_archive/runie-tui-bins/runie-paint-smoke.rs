//! `bin/runie-paint-smoke` — a quick check that the paint DSL produces
//! the same buffer as a known Grok reference for the top-bar pattern.
//!
//! Usage: `cargo run -p runie-tui --bin runie-paint-smoke`
//!
//! Iterates over every dump in `ui/dumps/grok/` and prints the top row
//! rendered by `paint()` against the actual top row of the dump, so we
//! can eyeball whether the layout primitives are working.

use runie_tui::paint::{paint, row, text, Node};
use runie_tui::theme::ThemeColors;

fn render_top_bar(width: u16) -> String {
    use ratatui::backend::TestBackend;
    use ratatui::Terminal;
    let backend = TestBackend::new(width, 1);
    let mut terminal = Terminal::new(backend).unwrap();
    use runie_tui::ThemeWrapper;
    let theme_wrapper = ThemeWrapper::crush_grok();
    let theme = ThemeColors::from(&theme_wrapper);
    let node = row(vec![
        Node::T(text(" feat/grok-redesign").accent()),
        Node::T(text(" ")),
        Node::T(text("~/Code/GitHub/runie").secondary()),
        Node::Fill,
        Node::T(text("│ 20K / 512K │").dim()),
    ])
    .fill_trailing()
    .bg(runie_tui::paint::StyleRef::BgBase)
    .build();
    terminal
        .draw(|frame| {
            paint(&node, frame.area(), frame.buffer_mut(), &theme);
        })
        .unwrap();
    let buf = terminal.backend().buffer().clone();
    let mut s = String::with_capacity(width as usize);
    for x in 0..buf.area.width {
        s.push_str(buf.cell((x, 0)).unwrap().symbol());
    }
    s
}

fn main() {
    let widths = [60u16, 78, 80, 120];
    for w in widths {
        let s = render_top_bar(w);
        println!("width={w}: |{s}| (len={})", s.chars().count());
    }
}
