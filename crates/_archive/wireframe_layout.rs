//! Wireframe tests — verify the layout debugger produces the expected
//! ASCII overlays. Run: `cargo test -p runie-tui --test wireframe_layout`.
//!
//! We use the `_for` variant of the wireframe helpers (which takes the
//! env var explicitly) so we don't fight the multithreaded test
//! harness. The wireframe in the real TUI binary uses `wireframe_box`
//! (the env-reading variant) at runtime.

use ratatui::{
    backend::TestBackend,
    buffer::Buffer,
    layout::{Constraint, Direction, Layout, Rect},
    Terminal,
};

use runie_tui::style::helpers::{wireframe_box_for, wireframe_enabled_for};

fn render_with_wireframe(area: Rect, component: &str, env: Option<String>) -> String {
    let backend = TestBackend::new(area.width, area.height);
    let mut terminal = Terminal::new(backend).unwrap();
    terminal
        .draw(|frame| {
            let area = frame.area();
            wireframe_box_for(frame.buffer_mut(), component, area, &env);
        })
        .unwrap();
    let buf: &Buffer = terminal.backend().buffer();
    buffer_to_string(buf)
}

fn buffer_to_string(buf: &Buffer) -> String {
    let mut s = String::new();
    for y in 0..buf.area.height {
        for x in 0..buf.area.width {
            let cell = buf.cell((x, y)).unwrap();
            s.push_str(cell.symbol());
        }
        if y + 1 < buf.area.height {
            s.push('\n');
        }
    }
    s
}

fn render_layout_with_chunks(component_filter: &str) -> String {
    let env = Some(component_filter.to_string());
    let backend = TestBackend::new(80, 24);
    let mut terminal = Terminal::new(backend).unwrap();
    let _ = terminal.draw(|frame| {
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(2),
                Constraint::Min(0),
                Constraint::Length(3),
                Constraint::Length(1),
            ])
            .split(frame.area());
        let buf = frame.buffer_mut();
        for (name, area) in [
            ("top_bar", chunks[0]),
            ("content", chunks[1]),
            ("input", chunks[2]),
            ("status", chunks[3]),
        ] {
            wireframe_box_for(buf, name, area, &env);
        }
    });
    let buf: &Buffer = terminal.backend().buffer();
    buffer_to_string(buf)
}

#[test]
fn wireframe_disabled_when_env_unset() {
    let env: Option<String> = None;
    let rendered = render_with_wireframe(Rect::new(0, 0, 80, 24), "test", env.clone());
    assert!(
        !rendered.contains('┌'),
        "wireframe should be off when env is unset: {rendered:?}"
    );
    // Also verify the pure predicate
    assert!(!wireframe_enabled_for("test", &env));
}

#[test]
fn wireframe_disabled_when_env_empty() {
    let env = Some("".to_string());
    let rendered = render_with_wireframe(Rect::new(0, 0, 80, 24), "test", env.clone());
    assert!(!rendered.contains('┌'), "empty env should disable wireframe");
    assert!(!wireframe_enabled_for("test", &env));
}

#[test]
fn wireframe_all_overlays_everything() {
    let rendered = render_layout_with_chunks("all");
    eprintln!("RENDERED:\n{rendered}");
    for name in &["top_bar", "content", "input", "status"] {
        assert!(rendered.contains(name), "missing wireframe label {name}");
    }
    assert!(rendered.contains("80x2"), "top_bar should be 80x2");
    assert!(rendered.contains("80x3"), "input should be 80x3");
    assert!(rendered.contains("80x1"), "status should be 80x1");
}

#[test]
fn wireframe_named_filter_selective() {
    let rendered = render_layout_with_chunks("input,status");
    assert!(rendered.contains("input"), "input should be drawn");
    assert!(rendered.contains("status"), "status should be drawn");
    assert!(
        !rendered.contains("top_bar"),
        "top_bar should be filtered out"
    );
    assert!(
        !rendered.contains("content"),
        "content should be filtered out"
    );
    // The status box is on the last line; it should contain "status" and "80x1"
    let last_line = rendered.lines().last().unwrap();
    assert!(
        last_line.contains("status") && last_line.contains("80x1"),
        "status box should have 80x1 in title: {last_line}"
    );
}
