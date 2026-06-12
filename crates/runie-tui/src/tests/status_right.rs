//! Layer 1 + Layer 3 tests for status line (left: git/folder + thinking, right: context chess piece + token throughput)

use crate::status_bar::{
    build_left_text, build_right_status, context_piece, context_usage, context_window_for,
};
use crate::ui::view;
use ratatui::{backend::TestBackend, Terminal};
use runie_core::{model::ThinkingLevel, snapshot::GitInfo, AppState};

fn flatten_buffer(buf: &ratatui::buffer::Buffer) -> String {
    (0..buf.area().height)
        .map(|y| {
            (0..buf.area().width)
                .map(|x| buf[(x, y)].symbol())
                .collect::<String>()
        })
        .collect()
}

// =============================================================================
// Layer 1: Pure function tests — no ratatui imports
// =============================================================================

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
    let info = GitInfo {
        repo_name: None,
        branch: None,
    };
    assert_eq!(info.format_right("my-project"), "my-project/");
}

#[test]
fn git_info_format_only_branch_shows_branch() {
    let info = GitInfo {
        repo_name: None,
        branch: Some("agent-impl".into()),
    };
    assert_eq!(info.format_right("runie"), "agent-impl");
}

#[test]
fn git_info_format_only_repo_shows_repo_slash() {
    let info = GitInfo {
        repo_name: Some("runie".into()),
        branch: None,
    };
    assert_eq!(info.format_right("runie"), "runie/");
}

// ── Left side tests ────────────────────────────────────────────────────────

#[test]
fn build_left_text_idle_shows_git_info_when_available() {
    let mut state = AppState::default();
    state.git_info = Some(GitInfo {
        repo_name: Some("runie".into()),
        branch: Some("agent-impl".into()),
    });
    state.cwd_name = "runie".into();
    let snap = state.snapshot();
    let left = build_left_text(&snap);
    assert!(
        left.contains("runie/agent-impl"),
        "Should show git info on LEFT, got: {}",
        left
    );
}

#[test]
fn build_left_text_idle_shows_folder_when_no_git() {
    let mut state = AppState::default();
    state.git_info = None;
    state.cwd_name = "my-project".into();
    let snap = state.snapshot();
    let left = build_left_text(&snap);
    assert!(
        left.contains("my-project/"),
        "Should show folder name on LEFT, got: {}",
        left
    );
}

#[test]
fn build_left_text_active_shows_working_not_git() {
    let mut state = AppState::default();
    state.agent.turn_active = true;
    state.agent.turn_started_at = Some(std::time::Instant::now());
    state.git_info = Some(GitInfo {
        repo_name: Some("runie".into()),
        branch: Some("agent-impl".into()),
    });
    let snap = state.snapshot();
    let left = build_left_text(&snap);
    assert!(
        !left.contains("runie/agent-impl"),
        "Active turn should NOT show git info on LEFT, got: {}",
        left
    );
    assert!(
        left.contains("Working"),
        "Should show Working... when active, got: {}",
        left
    );
}

#[test]
fn build_left_text_idle_shows_thinking_level_when_set() {
    let mut state = AppState::default();
    state.config.thinking_level = ThinkingLevel::Low;
    state.git_info = Some(GitInfo {
        repo_name: Some("runie".into()),
        branch: Some("agent-impl".into()),
    });
    let snap = state.snapshot();
    let left = build_left_text(&snap);
    assert!(
        left.contains("runie/agent-impl"),
        "Should show git info, got: {}",
        left
    );
    assert!(
        left.contains("Think: low"),
        "Should show thinking level, got: {}",
        left
    );
}

// ── Right side tests (context chess piece) ───────────────────────────────────

#[test]
fn context_piece_white_king_for_zero_usage() {
    assert_eq!(context_piece(0), '⛀');
    assert_eq!(context_piece(25), '⛀');
}

#[test]
fn context_piece_increments_with_usage() {
    assert_eq!(context_piece(26), '⛁');
    assert_eq!(context_piece(51), '⛂');
    assert_eq!(context_piece(76), '⛃');
    assert_eq!(context_piece(100), '⛃');
}

#[test]
fn build_right_status_idle_shows_piece_last() {
    let mut state = AppState::default();
    state.config.current_provider = "openai".to_string();
    state.config.current_model = "gpt-4o".to_string();
    let snap = state.snapshot();
    let right = build_right_status(&snap);
    // Chess piece must be the LAST character
    assert!(
        right.ends_with('⛀'),
        "Chess piece must be last char, got: {}",
        right
    );
    assert!(
        right.contains("128k ⛀"),
        "Should show 128k limit before piece, got: {}",
        right
    );
}

#[test]
fn build_right_status_active_shows_turn_stats_and_piece_last() {
    let mut state = AppState::default();
    state.config.current_provider = "openai".to_string();
    state.config.current_model = "gpt-4o".to_string();
    state.agent.turn_active = true;
    state.agent.turn_started_at = Some(std::time::Instant::now());
    let snap = state.snapshot();
    let right = build_right_status(&snap);
    assert!(right.contains("↑"), "Should show up arrow, got: {}", right);
    assert!(
        right.contains("↓"),
        "Should show down arrow, got: {}",
        right
    );
    assert!(right.contains("/s"), "Should show speed, got: {}", right);
    // Chess piece must be the LAST character
    assert!(
        right.ends_with('⛀'),
        "Chess piece must be last char when active, got: {}",
        right
    );
}

// ── Context window tests ────────────────────────────────────────────────────

#[test]
fn context_window_openai_gpt4o_is_128k() {
    assert_eq!(context_window_for("openai", "gpt-4o"), 128_000);
}

#[test]
fn context_window_anthropic_is_200k() {
    assert_eq!(
        context_window_for("anthropic", "claude-sonnet-4-6"),
        200_000
    );
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

// ── Context usage struct tests ────────────────────────────────────────────

#[test]
fn context_usage_limit_k_shows_m() {
    let mut state = AppState::default();
    state.config.current_provider = "google".to_string();
    state.config.current_model = "gemini-2.5-pro".to_string();
    let snap = state.snapshot();
    let usage = context_usage(&snap);
    assert_eq!(usage.limit_k(), "1M");
}

#[test]
fn context_usage_limit_k_shows_k() {
    let mut state = AppState::default();
    state.config.current_provider = "openai".to_string();
    state.config.current_model = "gpt-4o".to_string();
    let snap = state.snapshot();
    let usage = context_usage(&snap);
    assert_eq!(usage.limit_k(), "128k");
}

// =============================================================================
// Layer 3: Rendering tests — TestBackend + Buffer assertions
// =============================================================================

#[test]
fn status_left_renders_git_info_when_idle_in_repo() {
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
    assert!(
        content.contains("runie/agent-impl"),
        "Should show git info on left, got: {}",
        content
    );
}

#[test]
fn status_left_renders_folder_when_idle_no_git() {
    let _lock = crate::theme::test_lock();
    let mut state = AppState::default();
    state.config.current_provider = "openai".to_string();
    state.config.current_model = "gpt-4o".to_string();
    state.git_info = None;
    state.cwd_name = "my-project".into();
    let backend = TestBackend::new(60, 20);
    let mut terminal = Terminal::new(backend).unwrap();
    terminal.draw(|f| view(f, &mut state)).unwrap();
    let buf = terminal.backend().buffer();
    let content = flatten_buffer(buf);
    assert!(
        content.contains("my-project/"),
        "Should show folder name on left, got: {}",
        content
    );
}

#[test]
fn status_left_renders_working_when_active() {
    let _lock = crate::theme::test_lock();
    let mut state = AppState::default();
    state.config.current_provider = "openai".to_string();
    state.config.current_model = "gpt-4o".to_string();
    state.agent.turn_active = true;
    state.agent.turn_started_at = Some(std::time::Instant::now());
    state.git_info = Some(GitInfo {
        repo_name: Some("runie".into()),
        branch: Some("agent-impl".into()),
    });
    let backend = TestBackend::new(60, 20);
    let mut terminal = Terminal::new(backend).unwrap();
    terminal.draw(|f| view(f, &mut state)).unwrap();
    let buf = terminal.backend().buffer();
    let content = flatten_buffer(buf);
    assert!(
        content.contains("Working"),
        "Should show Working... on left when active"
    );
    assert!(
        !content.contains("runie/agent-impl"),
        "Should NOT show git info when active"
    );
}

#[test]
fn status_right_renders_piece_when_idle() {
    let _lock = crate::theme::test_lock();
    let mut state = AppState::default();
    state.config.current_provider = "openai".to_string();
    state.config.current_model = "gpt-4o".to_string();
    let backend = TestBackend::new(60, 20);
    let mut terminal = Terminal::new(backend).unwrap();
    terminal.draw(|f| view(f, &mut state)).unwrap();
    let buf = terminal.backend().buffer();
    let content = flatten_buffer(buf);
    assert!(content.contains('⛀'), "Should show chess piece on right");
}

#[test]
fn status_right_renders_turn_stats_and_piece_when_active() {
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
    assert!(content.contains('↑'), "Should show up arrow");
    assert!(content.contains('↓'), "Should show down arrow");
    assert!(content.contains("/s"), "Should show speed");
    assert!(content.contains('⛀'), "Should show chess piece");
}
