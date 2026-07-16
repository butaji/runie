//! Layer 3 rendering tests for swarm subagent lifecycle rows (GROK.md §26).

use ratatui::{backend::TestBackend, widgets::Paragraph, Terminal};
use runie_core::model::PatternWorkerStatus;
use runie_core::Element;

use crate::message::render_subagent_row;

fn render_to_string(lines: Vec<ratatui::text::Line<'static>>, width: u16, height: u16) -> String {
    let backend = TestBackend::new(width, height);
    let mut terminal = Terminal::new(backend).unwrap();
    terminal
        .draw(|f| {
            let paragraph = Paragraph::new(lines);
            f.render_widget(paragraph, f.area());
        })
        .unwrap();

    let buf = terminal.backend().buffer();
    (0..height)
        .map(|y| {
            (0..buf.area().width)
                .map(|x| buf[(x, y)].symbol())
                .collect::<String>()
        })
        .collect::<Vec<_>>()
        .join("\n")
}

fn subagent_row(
    status: PatternWorkerStatus,
    started: Option<std::time::Instant>,
    duration_ms: Option<u64>,
    output: &str,
    expanded: bool,
) -> Element {
    Element::SubagentRow {
        id: "w.1".into(),
        description: "find callers".into(),
        model: "echo".into(),
        status,
        started,
        duration_ms,
        activity: "Waiting for response…".into(),
        output: output.into(),
        expanded,
        timestamp: 0.0,
    }
}

// ─── Running ────────────────────────────────────────────────────────────────

#[test]
fn running_row_shows_grok_style_bar_diamond_and_format() {
    let elem = subagent_row(
        PatternWorkerStatus::Running,
        Some(std::time::Instant::now()),
        None,
        "",
        false,
    );
    let output = render_to_string(render_subagent_row(&elem, 0), 100, 3);
    assert!(
        output.contains("❙"),
        "running row should show the left bar: {output}"
    );
    assert!(
        output.contains("◆"),
        "running row should show the diamond bullet: {output}"
    );
    assert!(
        output.contains("Subagent running: “find callers” — Waiting for response… (echo)"),
        "running row format: {output}"
    );
}

// ─── Completed ──────────────────────────────────────────────────────────────

#[test]
fn completed_row_shows_check_and_duration() {
    let elem = subagent_row(
        PatternWorkerStatus::Completed,
        None,
        Some(2500),
        "out",
        false,
    );
    let output = render_to_string(render_subagent_row(&elem, 0), 100, 3);
    assert!(
        output.contains("◆ Subagent completed in 2.5s: “find callers”"),
        "completed row format: {output}"
    );
}

// ─── Failed ─────────────────────────────────────────────────────────────────

#[test]
fn failed_row_shows_x_and_duration() {
    let elem = subagent_row(PatternWorkerStatus::Failed, None, Some(1000), "boom", false);
    let output = render_to_string(render_subagent_row(&elem, 0), 100, 3);
    assert!(
        output.contains("◆ Subagent failed in 1.0s: “find callers”"),
        "failed row format: {output}"
    );
}

// ─── Expanded body ──────────────────────────────────────────────────────────

#[test]
fn expanded_completed_row_renders_output_body() {
    let elem = subagent_row(
        PatternWorkerStatus::Completed,
        None,
        Some(2500),
        "first line\nsecond line",
        true,
    );
    let output = render_to_string(render_subagent_row(&elem, 0), 100, 5);
    assert!(output.contains("first line"), "body line 1: {output}");
    assert!(output.contains("second line"), "body line 2: {output}");
}

#[test]
fn collapsed_completed_row_hides_output_body() {
    let elem = subagent_row(
        PatternWorkerStatus::Completed,
        None,
        Some(2500),
        "hidden body",
        false,
    );
    let output = render_to_string(render_subagent_row(&elem, 0), 100, 3);
    assert!(
        !output.contains("hidden body"),
        "collapsed row must not render the output body: {output}"
    );
}
