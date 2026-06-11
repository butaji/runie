//! Layer 1 + Layer 3 tests for status line right-side (context usage + radial bar + git info)

use runie_core::{AppState, snapshot::GitInfo};
use crate::ui::view;
use crate::status_bar::{radial_bar, context_window_for, build_right_status, ContextUsage};
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
// ── GitInfo Layer 1 tests ──────────────────────────────────────────────────

#[test]
fn git_info_format_with_repo_and_branch() {
    let info = GitInfo {
        repo_name: Some("runie".into()),
        branch: Some("agent-impl".into()),
    };
    assert_eq!(info.format_right("runie"), "runie/agent-impl");
}

#[test]
fn git_info_format_without_git_shows_folder() {
    let info = GitInfo { repo_name: None, branch: None };
    assert_eq!(info.format_right("my-project"), "my-project/");
}

#[test]
fn git_info_format_empty_branch_shows_folder() {
    // repo_name present but branch missing — fallback to folder
    let info = GitInfo { repo_name: Some("runie".into()), branch: None };
    assert_eq!(info.format_right("runie"), "runie/");
}

#[test]
fn build_right_status_idle_shows_git_info_when_available() {
    let mut state = AppState::default();
    state.config.current_provider = "openai".to_string();
    state.config.current_model = "gpt-4o".to_string();
    // Simulate being in a git repo
    state.git_info = Some(GitInfo {
        repo_name: Some("runie".into()),
        branch: Some("agent-impl".into()),
    });
    state.cwd_name = "runie".into();
    let snap = state.snapshot();
    let right = build_right_status(&snap);
    assert!(right.contains("runie/agent-impl"), "Should show git info, got: {}", right);
    assert!(right.contains("0%/128k"), "Should show context usage, got: {}", right);
    assert!(right.contains('○'), "Should show radial bar, got: {}", right);
}

#[test]
fn build_right_status_idle_shows_folder_when_no_git() {
    let mut state = AppState::default();
    state.config.current_provider = "openai".to_string();
    state.config.current_model = "gpt-4o".to_string();
    state.cwd_name = "my-project".into();
    let snap = state.snapshot();
    let right = build_right_status(&snap);
    assert!(right.contains("my-project/"), "Should show folder name, got: {}", right);
    assert!(right.contains("0%/128k"), "Should show context usage, got: {}", right);
}

#[test]
fn build_right_status_active_ignores_git_shows_turn_stats() {
    let mut state = AppState::default();
    state.config.current_provider = "openai".to_string();
    state.config.current_model = "gpt-4o".to_string();
    state.agent.turn_active = true;
    state.agent.turn_started_at = Some(std::time::Instant::now());
    state.git_info = Some(GitInfo {
        repo_name: Some("runie".into()),
        branch: Some("agent-impl".into()),
    });
    let snap = state.snapshot();
    let right = build_right_status(&snap);
    // When turn is active, git info should NOT appear
    assert!(!right.contains("runie/agent-impl"), "Active turn should NOT show git info, got: {}", right);
    assert!(right.contains("↑"), "Should show up arrow, got: {}", right);
}

fn radial_bar_0_percent_is_empty_circle() {
    assert_eq!(radial_bar(0), '○');
}

#[test]
fn radial_bar_12_percent_is_empty_circle() {
    assert_eq!(radial_bar(12), '○');
}

#[test]
fn radial_bar_13_percent_is_quarter() {
    assert_eq!(radial_bar(13), '◔');
}

#[test]
fn radial_bar_37_percent_is_quarter() {
    assert_eq!(radial_bar(37), '◔');
}

#[test]
fn radial_bar_38_percent_is_half() {
    assert_eq!(radial_bar(38), '◑');
}

#[test]
fn radial_bar_62_percent_is_half() {
    assert_eq!(radial_bar(62), '◑');
}

#[test]
fn radial_bar_63_percent_is_three_quarters() {
    assert_eq!(radial_bar(63), '◕');
}

#[test]
fn radial_bar_87_percent_is_three_quarters() {
    assert_eq!(radial_bar(87), '◕');
}

#[test]
fn radial_bar_88_percent_is_full() {
    assert_eq!(radial_bar(88), '●');
}

#[test]
fn radial_bar_100_percent_is_full() {
    assert_eq!(radial_bar(100), '●');
}

#[test]
fn context_window_openai_gpt4o_is_128k() {
    assert_eq!(context_window_for("openai", "gpt-4o"), 128_000);
}

#[test]
fn context_window_anthropic_is_200k() {
    assert_eq!(context_window_for("anthropic", "claude-sonnet-4-6"), 200_000);
}

#[test]
fn context_window_google_is_1m() {
    assert_eq!(context_window_for("google", "gemini-2.5-pro"), 1_000_000);
}

#[test]
fn context_window_unknown_defaults_to_128k() {
    assert_eq!(context_window_for("unknown", "model"), 128_000);
}

#[test]
fn context_window_openai_o1_is_200k() {
    assert_eq!(context_window_for("openai", "o1"), 200_000);
}

#[test]
fn limit_k_shows_k_for_thousands() {
    let ctx = ContextUsage { used: 1000, limit: 128_000, percent: 0 };
    assert_eq!(ctx.limit_k(), "128k");
}

#[test]
fn limit_k_shows_m_for_millions() {
    let ctx = ContextUsage { used: 1000, limit: 1_000_000, percent: 0 };
    assert_eq!(ctx.limit_k(), "1M");
}

#[test]
fn limit_k_shows_raw_for_small() {
    let ctx = ContextUsage { used: 100, limit: 500, percent: 0 };
    assert_eq!(ctx.limit_k(), "500");
}

#[test]
fn build_right_status_idle_shows_context_and_bar() {
    let mut state = AppState::default();
    state.config.current_provider = "openai".to_string();
    state.config.current_model = "gpt-4o".to_string();
    let snap = state.snapshot();
    let right = build_right_status(&snap);
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
    let right = build_right_status(&snap);
    assert!(!right.contains('⏵'), "Right side must NOT show extra timer, got: {}", right);
    // With animated values, tokens start at 0
    assert!(right.contains("↑"), "Should show up arrow, got: {}", right);
    assert!(right.contains("↓"), "Should show down arrow, got: {}", right);
    assert!(right.contains("/s"), "Should show speed, got: {}", right);
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
    // With animated values, tokens start at 0
    assert!(content.contains('↑'), "Should show up arrow");
    assert!(content.contains('↓'), "Should show down arrow");
    assert!(content.contains("/s"), "Should show speed");
    assert!(content.contains("0%/128k"), "Should show context usage");
}

#[test]
fn radial_bar_has_1_cell_right_margin() {
    let _lock = crate::theme::test_lock();
    let mut state = AppState::default();
    state.config.current_provider = "openai".to_string();
    state.config.current_model = "gpt-4o".to_string();
    let backend = TestBackend::new(60, 20);
    let mut terminal = Terminal::new(backend).unwrap();
    terminal.draw(|f| view(f, &mut state)).unwrap();
    let buf = terminal.backend().buffer();
    let w = buf.area().width;

    // Find the row containing the radial bar (status line)
    let status_y = (0..buf.area().height)
        .find(|&y| {
            (0..buf.area().width).any(|x| buf[(x, y)].symbol() == "○")
        })
        .expect("Should find radial bar in status line");

    let bar_x = (0..buf.area().width)
        .find(|&x| buf[(x, status_y)].symbol() == "○")
        .expect("Should find ○");

    // ○ should NOT be at the absolute right edge — there must be 1 cell margin
    assert!(
        bar_x < w - 1,
        "Radial bar at {} should not be at right edge (width={})",
        bar_x, w
    );

    // The column immediately to the right of ○ must be empty (the margin space)
    assert_eq!(
        buf[(bar_x + 1, status_y)].symbol().trim(),
        "",
        "Column right of ○ should be empty margin"
    );
}

#[test]
fn status_right_renders_git_info_when_idle_in_repo() {
    let _lock = crate::theme::test_lock();
    let mut state = AppState::default();
    state.config.current_provider = "openai".to_string();
    state.config.current_model = "gpt-4o".to_string();
    state.git_info = Some(GitInfo {
        repo_name: Some("runie".into()),
        branch: Some("agent-impl".into()),
    });
    state.cwd_name = "runie".into();
    let backend = TestBackend::new(60, 20);
    let mut terminal = Terminal::new(backend).unwrap();
    terminal.draw(|f| view(f, &mut state)).unwrap();
    let buf = terminal.backend().buffer();
    let content = flatten_buffer(buf);
    assert!(content.contains("runie/agent-impl"), "Should show git info, got: {}", content);
    assert!(content.contains("0%/128k"), "Should show context usage");
}

#[test]
fn status_right_renders_folder_when_idle_no_git() {
    let _lock = crate::theme::test_lock();
    let mut state = AppState::default();
    state.config.current_provider = "openai".to_string();
    state.config.current_model = "gpt-4o".to_string();
    state.cwd_name = "my-project".into();
    let backend = TestBackend::new(60, 20);
    let mut terminal = Terminal::new(backend).unwrap();
    terminal.draw(|f| view(f, &mut state)).unwrap();
    let buf = terminal.backend().buffer();
    let content = flatten_buffer(buf);
    assert!(content.contains("my-project/"), "Should show folder name, got: {}", content);
    assert!(content.contains("0%/128k"), "Should show context usage");
}
