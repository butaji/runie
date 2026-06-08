//! Tests for color restraint: only important things get bright colors.

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

// ─── Hierarchy: only user + active status are bright ────────────────────

#[test]
fn agent_message_is_low_contrast_not_bright() {
    let mut state = AppState::default();
    state.streaming = true;
    state.update(Event::AgentResponse { id: "req.0".into(), content: "Hello agent".into() });
    state.update(Event::AgentDone { id: "req.0".into() });
    let term = draw_state(&mut state);
    let colors = line_colors(&term, |l| l.contains("Hello agent"));
    let bright = Color::Rgb(208, 208, 208);
    assert!(
        !colors.iter().any(|c| *c == bright),
        "Agent message should NOT use fg_bright (#d0d0d0), got: {:?}", colors
    );
}

#[test]
fn thought_is_low_contrast_not_accent() {
    let mut state = AppState::default();
    state.streaming = true;
    state.update(Event::AgentThinking { id: "req.0".into() });
    state.update(Event::AgentThoughtDone { id: "req.0".into() });
    state.update(Event::AgentDone { id: "req.0".into() });
    let term = draw_state(&mut state);
    let colors = line_colors(&term, |l| l.contains("Thought"));
    let accent = Color::Rgb(139, 124, 244);
    assert!(
        !colors.iter().any(|c| *c == accent),
        "Thought should NOT use accent (#8b7cf4), got: {:?}", colors
    );
}

#[test]
fn tool_header_is_low_contrast_not_success() {
    let mut state = AppState::default();
    state.update(Event::AgentToolStart { id: "req.0".into(), name: "ls".into() });
    state.update(Event::AgentToolEnd { duration_secs: 0.5, output: "file1.txt".into() });
    let term = draw_state(&mut state);
    let colors = line_colors(&term, |l| l.contains("ls") && l.contains("✓"));
    let success = Color::Rgb(62, 189, 106);
    assert!(
        !colors.iter().any(|c| *c == success),
        "Tool header should NOT use success (#3ebd6a), got: {:?}", colors
    );
}

#[test]
fn turn_complete_is_low_contrast() {
    let mut state = AppState::default();
    state.streaming = true;
    state.update(Event::AgentResponse { id: "req.0".into(), content: "Done".into() });
    state.update(Event::AgentTurnComplete { id: "req.0".into(), duration_secs: 1.0 });
    state.update(Event::AgentDone { id: "req.0".into() });
    let term = draw_state(&mut state);
    let colors = line_colors(&term, |l| l.contains("Turn completed"));
    let fg = Color::Rgb(138, 138, 138);
    let dim = Color::Rgb(74, 74, 74);
    let fg_mid = Color::Rgb(168, 168, 168);
    let bright = Color::Rgb(208, 208, 208);
    let accent = Color::Rgb(139, 124, 244);
    assert!(
        !colors.iter().any(|c| *c == bright || *c == accent),
        "TurnComplete should not be bright or accent, got: {:?}", colors
    );
}

#[test]
fn user_message_is_bright() {
    let mut state = AppState::default();
    state.update(Event::Input('H'));
    state.update(Event::Input('i'));
    state.update(Event::Submit);
    let term = draw_state(&mut state);
    let colors = line_colors(&term, |l| l.contains("Hi"));
    let bright = Color::Rgb(208, 208, 208);
    assert!(
        colors.iter().any(|c| *c == bright),
        "User message SHOULD use fg_bright (#d0d0d0), got: {:?}", colors
    );
}

#[test]
fn status_active_is_success() {
    let mut state = AppState::default();
    state.turn_active = true;
    state.turn_started_at = Some(std::time::Instant::now());
    let term = draw_state(&mut state);
    let colors = line_colors(&term, |l| l.contains("Working"));
    let success = Color::Rgb(62, 189, 106);
    assert!(
        colors.iter().any(|c| *c == success),
        "Active status SHOULD use success (#3ebd6a), got: {:?}", colors
    );
}

#[test]
fn status_idle_is_dim() {
    let mut state = AppState::default();
    state.current_provider = "openai".into();
    state.current_model = "gpt-4".into();
    let term = draw_state(&mut state);
    let colors = line_colors(&term, |l| l.contains("openai/gpt-4"));
    let dim = Color::Rgb(74, 74, 74);
    let fg = Color::Rgb(138, 138, 138);
    assert!(
        !colors.iter().any(|c| *c == fg),
        "Idle status should NOT use fg (#8a8a8a), got: {:?}", colors
    );
}

// ─── Expanded and collapsed must share styling ──────────────────────────

#[test]
fn thought_expanded_and_collapsed_same_style() {
    // Expanded thought
    let mut state_exp = AppState::default();
    state_exp.streaming = true;
    state_exp.update(Event::AgentThinking { id: "req.0".into() });
    state_exp.update(Event::AgentThoughtDone { id: "req.0".into() });
    state_exp.update(Event::AgentDone { id: "req.0".into() });
    let term_exp = draw_state(&mut state_exp);
    let colors_exp = line_colors(&term_exp, |l| l.contains("Thought"));

    // Collapsed thought
    let mut state_col = AppState::default();
    state_col.streaming = true;
    state_col.update(Event::AgentThinking { id: "req.0".into() });
    state_col.update(Event::AgentThoughtDone { id: "req.0".into() });
    state_col.update(Event::AgentDone { id: "req.0".into() });
    state_col.update(Event::ToggleExpand);
    let term_col = draw_state(&mut state_col);
    let colors_col = line_colors(&term_col, |l| l.contains("Thought") || l.contains("[+]"));

    // Both should use the same non-accent, non-bright palette
    let accent = Color::Rgb(139, 124, 244);
    let bright = Color::Rgb(208, 208, 208);
    assert!(
        !colors_exp.iter().any(|c| *c == accent || *c == bright),
        "Expanded thought should be low-contrast, got: {:?}", colors_exp
    );
    assert!(
        !colors_col.iter().any(|c| *c == accent || *c == bright),
        "Collapsed thought should be low-contrast, got: {:?}", colors_col
    );
}

#[test]
fn tool_expanded_and_collapsed_same_style() {
    // Expanded tool
    let mut state_exp = AppState::default();
    state_exp.update(Event::AgentToolStart { id: "req.0".into(), name: "ls".into() });
    state_exp.update(Event::AgentToolEnd { duration_secs: 0.5, output: "file.txt".into() });
    let term_exp = draw_state(&mut state_exp);
    let colors_exp = line_colors(&term_exp, |l| l.contains("ls"));

    // Collapsed tool
    let mut state_col = AppState::default();
    state_col.update(Event::AgentToolStart { id: "req.0".into(), name: "ls".into() });
    state_col.update(Event::AgentToolEnd { duration_secs: 0.5, output: "file.txt".into() });
    state_col.update(Event::ToggleExpand);
    let term_col = draw_state(&mut state_col);
    let colors_col = line_colors(&term_col, |l| l.contains("ls") || l.contains("[+]"));

    let success = Color::Rgb(62, 189, 106);
    let bright = Color::Rgb(208, 208, 208);
    assert!(
        !colors_exp.iter().any(|c| *c == success || *c == bright),
        "Expanded tool should be low-contrast, got: {:?}", colors_exp
    );
    assert!(
        !colors_col.iter().any(|c| *c == success || *c == bright),
        "Collapsed tool should be low-contrast, got: {:?}", colors_col
    );
}
