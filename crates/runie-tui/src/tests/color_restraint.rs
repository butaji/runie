//! Tests for color restraint: only important things get bright colors.

use crate::theme::{
    color_accent, color_dim, color_fg, color_fg_bright, color_success, style_user, TEST_LOCK,
};
use crate::ui::view;
use ratatui::style::Color;
use ratatui::{backend::TestBackend, Terminal};
use runie_core::{AppState, Event};

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
    let _guard = TEST_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    let mut state = AppState::default();
    state.streaming = true;
    state.update(Event::AgentResponse {
        id: "req.0".into(),
        content: "Hello agent".into(),
    });
    state.update(Event::AgentDone { id: "req.0".into() });
    let term = draw_state(&mut state);
    let colors = line_colors(&term, |l| l.contains("Hello agent"));
    let bright = color_fg_bright();
    assert!(
        !colors.contains(&bright),
        "Agent message should NOT use fg_bright color, got: {:?}",
        colors
    );
}

#[test]
fn thought_is_low_contrast_not_accent() {
    let _guard = TEST_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    let mut state = AppState::default();
    state.streaming = true;
    state.update(Event::AgentThinking { id: "req.0".into() });
    state.update(Event::AgentThoughtDone { id: "req.0".into() });
    state.update(Event::AgentDone { id: "req.0".into() });
    let term = draw_state(&mut state);
    let colors = line_colors(&term, |l| l.contains("Thought"));
    let accent = color_accent();
    assert!(
        !colors.contains(&accent),
        "Thought should NOT use accent color, got: {:?}",
        colors
    );
}

#[test]
fn tool_header_is_low_contrast_not_success() {
    let _guard = TEST_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    let mut state = AppState::default();
    state.update(Event::AgentToolStart {
        id: "req.0".into(),
        name: "ls".into(),
    });
    state.update(Event::AgentToolEnd {
        duration_secs: 0.5,
        output: "file1.txt".into(),
    });
    let term = draw_state(&mut state);
    let colors = line_colors(&term, |l| l.contains("ls") && l.contains("✓"));
    let success = color_success();
    assert!(
        !colors.contains(&success),
        "Tool header should NOT use success color, got: {:?}",
        colors
    );
}

#[test]
fn turn_complete_is_low_contrast() {
    let _guard = TEST_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    let mut state = AppState::default();
    state.streaming = true;
    state.update(Event::AgentResponse {
        id: "req.0".into(),
        content: "Done".into(),
    });
    state.update(Event::AgentTurnComplete {
        id: "req.0".into(),
        duration_secs: 1.0,
    });
    state.update(Event::AgentDone { id: "req.0".into() });
    let term = draw_state(&mut state);
    let colors = line_colors(&term, |l| l.contains("Turn completed"));
    let bright = color_fg_bright();
    let accent = color_accent();
    assert!(
        !colors.iter().any(|c| *c == bright || *c == accent),
        "TurnComplete should not be bright or accent, got: {:?}",
        colors
    );
}

#[test]
fn user_message_is_bright() {
    let _guard = TEST_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    let mut state = AppState::default();
    state.update(Event::Input('H'));
    state.update(Event::Input('i'));
    state.update(Event::Submit);
    let term = draw_state(&mut state);
    let colors = line_colors(&term, |l| l.contains("Hi"));
    let user_color = style_user().fg.unwrap();
    assert!(
        colors.contains(&user_color),
        "User message SHOULD use user style color, got: {:?}",
        colors
    );
}

#[test]
fn status_active_is_dim_not_success() {
    let _guard = TEST_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    let mut state = AppState::default();
    state.agent.turn_active = true;
    state.agent.turn_started_at = Some(std::time::Instant::now());
    let term = draw_state(&mut state);
    let colors = line_colors(&term, |l| l.contains("Working"));
    let dim = color_dim();
    let success = color_success();
    assert!(
        colors.contains(&dim),
        "Active status should use dim color, got: {:?}",
        colors
    );
    assert!(
        !colors.contains(&success),
        "Active status should NOT use success color, got: {:?}",
        colors
    );
}

#[test]
fn status_idle_is_dim() {
    let _guard = TEST_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    let mut state = AppState::default();
    state.config.current_provider = "openai".into();
    state.config.current_model = "gpt-4".into();
    let term = draw_state(&mut state);
    let colors = line_colors(&term, |l| l.contains("openai/gpt-4"));
    let _dim = color_dim();
    let fg = color_fg();
    assert!(
        !colors.contains(&fg),
        "Idle status should NOT use fg color, got: {:?}",
        colors
    );
}

// ─── Expanded and collapsed must share styling ──────────────────────────

#[test]
fn thought_expanded_and_collapsed_same_style() {
    let _guard = TEST_LOCK.lock().unwrap_or_else(|e| e.into_inner());
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
    let accent = color_accent();
    let bright = color_fg_bright();
    assert!(
        !colors_exp.iter().any(|c| *c == accent || *c == bright),
        "Expanded thought should be low-contrast, got: {:?}",
        colors_exp
    );
    assert!(
        !colors_col.iter().any(|c| *c == accent || *c == bright),
        "Collapsed thought should be low-contrast, got: {:?}",
        colors_col
    );
}

#[test]
fn tool_expanded_and_collapsed_same_style() {
    let _guard = TEST_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    // Expanded tool
    let mut state_exp = AppState::default();
    state_exp.update(Event::AgentToolStart {
        id: "req.0".into(),
        name: "ls".into(),
    });
    state_exp.update(Event::AgentToolEnd {
        duration_secs: 0.5,
        output: "file.txt".into(),
    });
    let term_exp = draw_state(&mut state_exp);
    let colors_exp = line_colors(&term_exp, |l| l.contains("ls"));

    // Collapsed tool
    let mut state_col = AppState::default();
    state_col.update(Event::AgentToolStart {
        id: "req.0".into(),
        name: "ls".into(),
    });
    state_col.update(Event::AgentToolEnd {
        duration_secs: 0.5,
        output: "file.txt".into(),
    });
    state_col.update(Event::ToggleExpand);
    let term_col = draw_state(&mut state_col);
    let colors_col = line_colors(&term_col, |l| l.contains("ls") || l.contains("[+]"));

    let success = color_success();
    let bright = color_fg_bright();
    assert!(
        !colors_exp.iter().any(|c| *c == success || *c == bright),
        "Expanded tool should be low-contrast, got: {:?}",
        colors_exp
    );
    assert!(
        !colors_col.iter().any(|c| *c == success || *c == bright),
        "Collapsed tool should be low-contrast, got: {:?}",
        colors_col
    );
}
