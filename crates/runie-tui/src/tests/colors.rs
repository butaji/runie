//! Tests for chat feed color assignments

use ratatui::style::Color;
use runie_core::{AppState, Event};
use crate::ui::view;
use ratatui::{backend::TestBackend, Terminal};
use crate::theme::{color_fg, color_dim, color_accent, TEST_LOCK};

fn draw_state(state: &mut AppState) -> Terminal<TestBackend> {
    let backend = TestBackend::new(60, 20);
    let mut terminal = Terminal::new(backend).unwrap();
    terminal.draw(|f| view(f, state)).unwrap();
    terminal
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
fn agent_message_uses_fg() {
    let _guard = TEST_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    let mut state = AppState::default();
    state.streaming = true;
    state.update(Event::AgentResponse { id: "req.0".into(), content: "Hello agent".into() });
    state.update(Event::AgentDone { id: "req.0".into() });
    let term = draw_state(&mut state);
    let colors = line_colors(&term, |l| l.contains("Hello agent"));
    let fg = color_fg();
    assert!(
        colors.contains(&fg),
        "Agent message should use fg color, got colors: {:?}", colors
    );
}

#[test]
fn turn_complete_uses_dim() {
    let _guard = TEST_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    let mut state = AppState::default();
    state.streaming = true;
    state.update(Event::AgentThinking { id: "req.0".into() });
    state.update(Event::AgentThoughtDone { id: "req.0".into() });
    state.update(Event::AgentToolStart { id: "req.0".into(), name: "ls".into() });
    state.update(Event::AgentToolEnd { duration_secs: 0.5, output: "a".into() });
    state.update(Event::AgentResponse { id: "req.0".into(), content: "Done".into() });
    state.update(Event::AgentTurnComplete { id: "req.0".into(), duration_secs: 1.0 });
    state.update(Event::AgentDone { id: "req.0".into() });
    let term = draw_state(&mut state);
    let colors = line_colors(&term, |l| l.contains("Turn completed"));
    let dim = color_dim();
    assert!(
        colors.contains(&dim),
        "TurnComplete should use dim color, got colors: {:?}", colors
    );
}

#[test]
fn status_idle_uses_dim() {
    let _guard = TEST_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    let mut state = AppState::default();
    state.config.current_provider = "openai".into();
    state.config.current_model = "gpt-4".into();
    let term = draw_state(&mut state);
    let colors = line_colors(&term, |l| l.contains("openai/gpt-4"));
    let dim = color_dim();
    assert!(
        colors.contains(&dim),
        "Idle status should use dim color, got colors: {:?}", colors
    );
}

#[test]
fn tool_done_output_uses_fg() {
    let _guard = TEST_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    let mut state = AppState::default();
    state.update(Event::AgentToolStart { id: "req.0".into(), name: "ls".into() });
    state.update(Event::AgentToolEnd { duration_secs: 0.5, output: "file1.txt".into() });
    let term = draw_state(&mut state);
    let colors = line_colors(&term, |l| l.contains("file1.txt"));
    let fg = color_fg();
    assert!(
        colors.contains(&fg),
        "Tool output should use fg color, got colors: {:?}", colors
    );
}

#[test]
fn thought_uses_dim() {
    let _guard = TEST_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    let mut state = AppState::default();
    state.streaming = true;
    state.update(Event::AgentThinking { id: "req.0".into() });
    state.update(Event::AgentThoughtDone { id: "req.0".into() });
    state.update(Event::AgentDone { id: "req.0".into() });
    let term = draw_state(&mut state);
    let colors = line_colors(&term, |l| l.contains("Thought"));
    let dim = color_dim();
    assert!(
        colors.contains(&dim),
        "Thought marker should use dim color, got colors: {:?}", colors
    );
}

#[test]
fn empty_state_uses_dim() {
    let _guard = TEST_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    let mut state = AppState::default();
    let term = draw_state(&mut state);
    let colors = line_colors(&term, |l| l.contains("Type a message"));
    let dim = color_dim();
    assert!(
        colors.contains(&dim),
        "Empty state hint should use dim color, got colors: {:?}", colors
    );
}

#[test]
fn inline_code_parsed_in_markdown() {
    let _guard = TEST_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    use crate::markdown::parse_inline_markdown;
    let spans = parse_inline_markdown("use `cargo test` to run");
    let code_span = spans.iter().find(|s| s.content == "cargo test");
    assert!(code_span.is_some(), "Should have code span 'cargo test'");
    let accent = color_accent();
    assert_eq!(
        code_span.unwrap().style.fg.unwrap_or(Color::Reset),
        accent,
        "Inline code should use accent color"
    );
}

#[test]
fn inline_code_with_bg_highlight() {
    let _guard = TEST_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    use crate::markdown::parse_inline_markdown;
    let spans = parse_inline_markdown("`hello`");
    let code_span = spans.iter().find(|s| s.content == "hello").expect("code span");
    assert!(
        code_span.style.bg.is_some(),
        "Inline code should have background color"
    );
}
