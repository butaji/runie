//! Layer 1 + Layer 3 tests for status line right-side (context usage + radial bar)

use runie_core::{AppState, Element, ChatMessage, Role};
use crate::ui::view;
use ratatui::{backend::TestBackend, Terminal};

fn flatten_buffer(buf: &ratatui::buffer::Buffer) -> String {
    (0..buf.area().height)
        .map(|y| (0..buf.area().width)
            .map(|x| buf[(x, y)].symbol())
            .collect::<String>())
        .collect()
}

// =============================================================================
// Layer 1: Pure function tests — no ratatui imports
// =============================================================================

#[test]
fn radial_bar_0_percent_is_empty_circle() {
    assert_eq!(crate::ui::radial_bar(0), '○');
}

#[test]
fn radial_bar_12_percent_is_empty_circle() {
    assert_eq!(crate::ui::radial_bar(12), '○');
}

#[test]
fn radial_bar_13_percent_is_quarter() {
    assert_eq!(crate::ui::radial_bar(13), '◔');
}

#[test]
fn radial_bar_37_percent_is_quarter() {
    assert_eq!(crate::ui::radial_bar(37), '◔');
}

#[test]
fn radial_bar_38_percent_is_half() {
    assert_eq!(crate::ui::radial_bar(38), '◑');
}

#[test]
fn radial_bar_62_percent_is_half() {
    assert_eq!(crate::ui::radial_bar(62), '◑');
}

#[test]
fn radial_bar_63_percent_is_three_quarters() {
    assert_eq!(crate::ui::radial_bar(63), '◕');
}

#[test]
fn radial_bar_87_percent_is_three_quarters() {
    assert_eq!(crate::ui::radial_bar(87), '◕');
}

#[test]
fn radial_bar_88_percent_is_full() {
    assert_eq!(crate::ui::radial_bar(88), '●');
}

#[test]
fn radial_bar_100_percent_is_full() {
    assert_eq!(crate::ui::radial_bar(100), '●');
}

#[test]
fn context_window_openai_gpt4o_is_128k() {
    assert_eq!(crate::ui::context_window_for("openai", "gpt-4o"), 128_000);
}

#[test]
fn context_window_anthropic_is_200k() {
    assert_eq!(crate::ui::context_window_for("anthropic", "claude-sonnet-4-6"), 200_000);
}

#[test]
fn context_window_google_is_1m() {
    assert_eq!(crate::ui::context_window_for("google", "gemini-2.5-pro"), 1_000_000);
}

#[test]
fn context_window_unknown_defaults_to_128k() {
    assert_eq!(crate::ui::context_window_for("unknown", "model"), 128_000);
}

#[test]
fn context_window_openai_o1_is_200k() {
    assert_eq!(crate::ui::context_window_for("openai", "o1"), 200_000);
}

#[test]
fn limit_k_shows_k_for_thousands() {
    let ctx = crate::ui::ContextUsage { used: 1000, limit: 128_000, percent: 0 };
    assert_eq!(ctx.limit_k(), "128k");
}

#[test]
fn limit_k_shows_m_for_millions() {
    let ctx = crate::ui::ContextUsage { used: 1000, limit: 1_000_000, percent: 0 };
    assert_eq!(ctx.limit_k(), "1M");
}

#[test]
fn limit_k_shows_raw_for_small() {
    let ctx = crate::ui::ContextUsage { used: 100, limit: 500, percent: 0 };
    assert_eq!(ctx.limit_k(), "500");
}

#[test]
fn build_right_status_idle_shows_context_and_bar() {
    let mut state = AppState::default();
    state.config.current_provider = "openai".to_string();
    state.config.current_model = "gpt-4o".to_string();
    let snap = state.snapshot();
    let right = crate::ui::build_right_status(&snap);
    assert!(right.contains("0%/128k"), "Should show context usage, got: {}", right);
    assert!(right.contains('○'), "Should show radial bar, got: {}", right);
}

#[test]
fn build_right_status_active_shows_turn_stats() {
    let mut state = AppState::default();
    state.config.current_provider = "openai".to_string();
    state.config.current_model = "gpt-4o".to_string();
    state.agent.turn_active = true;
    state.agent.turn_started_at = Some(std::time::Instant::now());
    let snap = state.snapshot();
    let right = crate::ui::build_right_status(&snap);
    assert!(!right.contains('⏵'), "Right side must NOT show extra timer, got: {}", right);
    assert!(right.contains("↑- ↓- -/s"), "Should show ↑/↓/speed when active, got: {}", right);
    assert!(right.contains("0%/128k"), "Should show context, got: {}", right);
}

// =============================================================================
// Layer 3: Rendering tests — TestBackend + Buffer assertions
// =============================================================================

#[test]
fn status_right_renders_context_usage_when_idle() {
    let _lock = crate::theme::test_lock();
    let mut state = AppState::default();
    state.config.current_provider = "openai".to_string();
    state.config.current_model = "gpt-4o".to_string();
    let backend = TestBackend::new(60, 20);
    let mut terminal = Terminal::new(backend).unwrap();
    terminal.draw(|f| view(f, &mut state)).unwrap();
    let buf = terminal.backend().buffer();
    let content = flatten_buffer(buf);
    assert!(content.contains("0%/128k"), "Should show context usage");
    assert!(content.contains('○'), "Should show empty radial bar");
}

#[test]
fn status_right_renders_turn_stats_when_active() {
    let _lock = crate::theme::test_lock();
    let mut state = AppState::default();
    state.config.current_provider = "openai".to_string();
    state.config.current_model = "gpt-4o".to_string();
    state.agent.turn_active = true;
    state.agent.turn_started_at = Some(std::time::Instant::now());
    let backend = TestBackend::new(60, 20);
    let mut terminal = Terminal::new(backend).unwrap();
    terminal.draw(|f| view(f, &mut state)).unwrap();
    let buf = terminal.backend().buffer();
    let content = flatten_buffer(buf);
    assert!(!content.contains('⏵'), "Right side must NOT show extra timer");
    assert!(content.contains("↑- ↓- -/s"), "Should show ↑/↓/speed when active");
    assert!(content.contains("0%/128k"), "Should show context usage");
}

#[test]
fn radial_bar_is_at_rightmost_column() {
    let _lock = crate::theme::test_lock();
    let mut state = AppState::default();
    state.config.current_provider = "openai".to_string();
    state.config.current_model = "gpt-4o".to_string();
    let backend = TestBackend::new(60, 20);
    let mut terminal = Terminal::new(backend).unwrap();
    terminal.draw(|f| view(f, &mut state)).unwrap();
    let buf = terminal.backend().buffer();

    // Find the row containing the radial bar (status line)
    let status_y = (0..buf.area().height)
        .find(|&y| {
            (0..buf.area().width).any(|x| buf[(x, y)].symbol() == "○")
        })
        .expect("Should find radial bar in status line");

    // Find the rightmost non-empty cell on that row
    let rightmost = (0..buf.area().width)
        .rev()
        .find(|&x| !buf[(x, status_y)].symbol().trim().is_empty())
        .expect("Status line should have content");

    assert_eq!(
        buf[(rightmost, status_y)].symbol(),
        "○",
        "Radial bar should be at the very end of the status line"
    );
}
