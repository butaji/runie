//! Tests for chat feed color assignments

use ratatui::style::Color;
use runie_core::{AppState, ChatMessage, Event, Role};
use crate::ui::view;
use ratatui::{backend::TestBackend, Terminal};

fn draw_state(state: &mut AppState) -> Terminal<TestBackend> {
    let backend = TestBackend::new(60, 20);
    let mut terminal = Terminal::new(backend).unwrap();
    terminal.draw(|f| view(f, state)).unwrap();
    terminal
}

/// Get the fg color of the first cell matching the predicate.
fn first_cell_color(term: &Terminal<TestBackend>, predicate: impl Fn(&str) -> bool) -> Option<Color> {
    let buf = term.backend().buffer();
    for y in 0..buf.area().height {
        let line: String = (0..buf.area().width)
            .map(|x| buf[(x, y)].symbol().to_string())
            .collect();
        if predicate(&line) {
            for x in 0..buf.area().width {
                let cell = &buf[(x, y)];
                if cell.symbol() != " " {
                    return cell.style().fg;
                }
            }
        }
    }
    None
}

/// Collect all non-space fg colors on a line matching the predicate.
fn line_colors(term: &Terminal<TestBackend>, predicate: impl Fn(&str) -> bool) -> Vec<Color> {
    let buf = term.backend().buffer();
    let mut colors = Vec::new();
    for y in 0..buf.area().height {
        let line: String = (0..buf.area().width)
            .map(|x| buf[(x, y)].symbol().to_string())
            .collect();
        if predicate(&line) {
            for x in 0..buf.area().width {
                let cell = &buf[(x, y)];
                if cell.symbol() != " " {
                    if let Some(c) = cell.style().fg {
                        colors.push(c);
                    }
                }
            }
            break;
        }
    }
    colors
}

#[test]
fn agent_message_uses_fg_bright_not_fg() {
    let mut state = AppState::default();
    state.streaming = true;
    state.update(Event::AgentResponse { id: "req.0".into(), content: "Hello agent".into() });
    state.update(Event::AgentDone { id: "req.0".into() });
    let term = draw_state(&mut state);
    let colors = line_colors(&term, |l| l.contains("Hello agent"));
    let bright = Color::Rgb(208, 208, 208);
    assert!(
        colors.iter().any(|c| *c == bright),
        "Agent message should use fg_bright (#d0d0d0), got colors: {:?}", colors
    );
}

#[test]
fn turn_complete_uses_fg_mid_not_dim() {
    let mut state = AppState::default();
    state.streaming = true;
    state.update(Event::AgentResponse { id: "req.0".into(), content: "Done".into() });
    state.update(Event::AgentTurnComplete { id: "req.0".into(), duration_secs: 1.0 });
    state.update(Event::AgentDone { id: "req.0".into() });
    let term = draw_state(&mut state);
    let colors = line_colors(&term, |l| l.contains("Turn completed"));
    let fg = Color::Rgb(138, 138, 138);
    assert!(
        colors.iter().any(|c| *c == fg),
        "TurnComplete should use fg (#8a8a8a) for readability, got colors: {:?}", colors
    );
}

#[test]
fn status_idle_uses_fg_not_dim() {
    let mut state = AppState::default();
    state.current_provider = "openai".into();
    state.current_model = "gpt-4".into();
    let term = draw_state(&mut state);
    let colors = line_colors(&term, |l| l.contains("openai/gpt-4"));
    let fg = Color::Rgb(138, 138, 138);
    assert!(
        colors.iter().any(|c| *c == fg),
        "Idle status should use fg (#8a8a8a) for readability, got colors: {:?}", colors
    );
}

#[test]
fn tool_done_output_uses_fg_not_fg_mid() {
    let mut state = AppState::default();
    state.update(Event::AgentToolStart { id: "req.0".into(), name: "ls".into() });
    state.update(Event::AgentToolEnd { duration_secs: 0.5, output: "file1.txt".into() });
    let term = draw_state(&mut state);
    let colors = line_colors(&term, |l| l.contains("file1.txt"));
    let fg = Color::Rgb(138, 138, 138);
    let fg_mid = Color::Rgb(168, 168, 168);
    assert!(
        colors.iter().any(|c| *c == fg),
        "Tool output should use fg (#8a8a8a) for clarity, got colors: {:?}", colors
    );
}

#[test]
fn thought_uses_accent_not_fg_mid() {
    let mut state = AppState::default();
    state.streaming = true;
    state.update(Event::AgentThinking { id: "req.0".into() });
    state.update(Event::AgentThoughtDone { id: "req.0".into() });
    state.update(Event::AgentDone { id: "req.0".into() });
    let term = draw_state(&mut state);
    let colors = line_colors(&term, |l| l.contains("Thought"));
    let accent = Color::Rgb(139, 124, 244);
    assert!(
        colors.iter().any(|c| *c == accent),
        "Thought marker should use accent (#8b7cf4) to distinguish from tools, got colors: {:?}", colors
    );
}

#[test]
fn empty_state_uses_fg_mid_not_dim() {
    let mut state = AppState::default();
    let term = draw_state(&mut state);
    let colors = line_colors(&term, |l| l.contains("Type a message"));
    let fg_mid = Color::Rgb(168, 168, 168);
    assert!(
        colors.iter().any(|c| *c == fg_mid),
        "Empty state hint should use fg_mid (#a8a8a8), got colors: {:?}", colors
    );
}

#[test]
fn inline_code_parsed_in_markdown() {
    use crate::markdown::parse_inline_markdown;
    let spans = parse_inline_markdown("use `cargo test` to run");
    let code_span = spans.iter().find(|s| s.content == "cargo test");
    assert!(code_span.is_some(), "Should have code span 'cargo test'");
    let accent = Color::Rgb(139, 124, 244);
    assert_eq!(
        code_span.unwrap().style.fg.unwrap_or(Color::Reset),
        accent,
        "Inline code should use accent color"
    );
}

#[test]
fn inline_code_with_bg_highlight() {
    use crate::markdown::parse_inline_markdown;
    let spans = parse_inline_markdown("`hello`");
    let code_span = spans.iter().find(|s| s.content == "hello").expect("code span");
    assert!(
        code_span.style.bg.is_some(),
        "Inline code should have background color"
    );
}
