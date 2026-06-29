//! TUI smoke tests — verify view() integrates with core state

/// Guardrail: no module name should exist in both runie-core and runie-tui.
/// This prevents split-brain ownership where the same name means different things.
#[test]
fn smoke_no_module_name_collision() {
    let core_lib = include_str!("../../../runie-core/src/lib.rs");
    let tui_lib = include_str!("../../../runie-tui/src/lib.rs");

    let core_modules: std::collections::HashSet<&str> = core_lib
        .lines()
        .filter(|l| l.trim().starts_with("pub mod ") || l.trim().starts_with("pub(crate) mod "))
        .filter_map(|l| {
            let t = l.trim();
            t.strip_prefix("pub mod ")
                .or_else(|| t.strip_prefix("pub(crate) mod "))
                .and_then(|s| s.trim_end_matches(';').trim().split_whitespace().next())
        })
        .collect();

    let tui_modules: std::collections::HashSet<&str> = tui_lib
        .lines()
        .filter(|l| l.trim().starts_with("pub mod ") || l.trim().starts_with("pub(crate) mod "))
        .filter_map(|l| {
            let t = l.trim();
            t.strip_prefix("pub mod ")
                .or_else(|| t.strip_prefix("pub(crate) mod "))
                .and_then(|s| s.trim_end_matches(';').trim().split_whitespace().next())
        })
        .collect();

    let collisions: Vec<_> = core_modules.intersection(&tui_modules).copied().collect();

    // Known resolved pairs (different meanings; not a bug):
    // diff: core=domain patch logic, tui=gutter render (different files)
    // ui:   core=Element/Feed view-model, tui=layout/messages (different files)
    // message: core=ChatMessage types, tui=message/ render (different dirs)
    let ignored = ["diff", "ui", "message"];
    let real_collisions: Vec<_> = collisions
        .into_iter()
        .filter(|n| !ignored.contains(n))
        .collect();

    assert!(
        real_collisions.is_empty(),
        "Module name(s) exist in both runie-core and runie-tui: {:?}",
        real_collisions
    );
}

use crate::ui::view;
use ratatui::{backend::TestBackend, Terminal};
use runie_core::AppState;
use runie_core::Event;

use super::connect_model;

pub(crate) fn draw_state(state: &mut AppState) -> String {
    let backend = TestBackend::new(60, 20);
    let mut terminal = Terminal::new(backend).unwrap();
    terminal.draw(|f| view(f, state)).unwrap();
    terminal
        .backend()
        .buffer()
        .content
        .iter()
        .map(|c| c.symbol())
        .collect()
}

#[test]
fn empty_state_renders_input_prompt() {
    let mut state = AppState::default();
    connect_model(&mut state);
    let content = draw_state(&mut state);
    assert!(
        content.contains("❯ "),
        "Empty state should show input prompt"
    );
}

#[test]
fn user_message_renders() {
    let mut state = AppState::default();
    connect_model(&mut state);
    state.update(Event::Input('H'));
    state.update(Event::Input('i'));
    state.update(Event::Submit);
    let content = draw_state(&mut state);
    assert!(content.contains("❯ Hi"), "Should render user prefix");
    assert!(content.contains("Hi"), "Should render message content");
}

#[test]
fn agent_response_renders() {
    let mut state = AppState::default();
    connect_model(&mut state);
    state.agent.streaming = true;
    state.update(Event::Thinking {
        id: "req.0".to_string(),
    });
    state.update(Event::ThoughtDone {
        id: "req.0".to_string(),
    });
    state.update(Event::Response {
        id: "req.0".to_string(),
        content: "Hello".to_string(),
    });
    let content = draw_state(&mut state);
    assert!(content.contains("→ Hello"), "Should render agent prefix");
}

#[test]
fn tool_done_renders() {
    let mut state = AppState::default();
    connect_model(&mut state);
    state.update(Event::ToolStart {
        id: "req.0".to_string(),
        name: "list_files".to_string(),
        input: serde_json::Value::Null,
    });
    state.update(Event::ToolEnd {
        id: "".to_string(),
        duration_secs: 0.5,
        output: String::new(),
    });
    let content = draw_state(&mut state);
    assert!(content.contains("✓"), "Should render tool done");
    assert!(content.contains("list_files"), "Should show tool name");
}

#[test]
fn reset_clears_messages() {
    let mut state = AppState::default();
    connect_model(&mut state);
    state.update(Event::Input('T'));
    state.update(Event::Submit);
    state.update(Event::Reset);
    connect_model(&mut state);
    let content = draw_state(&mut state);
    let count = content.matches("❯ ").count();
    assert_eq!(
        count, 1,
        "Reset should clear messages, keep only input prompt"
    );
}

#[test]
fn status_shows_provider_model() {
    let mut state = AppState::default();
    connect_model(&mut state);
    state.config.current_provider = "openai".to_string();
    state.config.current_model = "gpt-4".to_string();
    let backend = TestBackend::new(60, 10);
    let mut terminal = Terminal::new(backend).unwrap();
    terminal.draw(|f| view(f, &mut state)).unwrap();
    let buf = terminal.backend().buffer();
    let content: String = (0..buf.area().height)
        .map(|y| {
            (0..buf.area().width)
                .map(|x| buf[(x, y)].symbol())
                .collect::<String>()
        })
        .collect();
    assert!(content.contains("openai"), "Status should show provider");
    assert!(content.contains("gpt-4"), "Status should show model");
}

#[test]
fn status_shows_thinking_badge_when_active() {
    let mut state = AppState::default();
    connect_model(&mut state);
    state.config.thinking_level = runie_core::model::ThinkingLevel::Medium;
    let backend = TestBackend::new(100, 10); // 60-wide too narrow for worktree + badge
    let mut terminal = Terminal::new(backend).unwrap();
    terminal.draw(|f| view(f, &mut state)).unwrap();
    let buf = terminal.backend().buffer();
    let content: String = (0..buf.area().height)
        .map(|y| {
            (0..buf.area().width)
                .map(|x| buf[(x, y)].symbol())
                .collect::<String>()
        })
        .collect();
    assert!(
        content.contains("Think: medium"),
        "Status should show thinking level badge: {}",
        content
    );
}

#[test]
fn status_hides_thinking_badge_when_off() {
    let mut state = AppState::default();
    connect_model(&mut state);
    state.config.thinking_level = runie_core::model::ThinkingLevel::Off;
    let backend = TestBackend::new(60, 10);
    let mut terminal = Terminal::new(backend).unwrap();
    terminal.draw(|f| view(f, &mut state)).unwrap();
    let buf = terminal.backend().buffer();
    let content: String = (0..buf.area().height)
        .map(|y| {
            (0..buf.area().width)
                .map(|x| buf[(x, y)].symbol())
                .collect::<String>()
        })
        .collect();
    assert!(
        !content.contains("Think:"),
        "Status should NOT show thinking badge when off: {}",
        content
    );
}

#[test]
fn empty_state_shows_hint() {
    let mut state = AppState::default();
    connect_model(&mut state);
    let backend = TestBackend::new(60, 20);
    let mut terminal = Terminal::new(backend).unwrap();
    terminal.draw(|f| view(f, &mut state)).unwrap();
    let buf = terminal.backend().buffer();
    let content: String = (0..buf.area().height)
        .map(|y| {
            (0..buf.area().width)
                .map(|x| buf[(x, y)].symbol())
                .collect::<String>()
        })
        .collect();
    assert!(
        content.contains("Type a message"),
        "Empty state should show hint text"
    );
}

// ─── tmux-test.sh replacement ────────────────────────────────────────────────
//
// `scripts/tmxtest.sh` was deleted. It verified three startup behaviors:
//   1. Binary starts — covered by `cargo build` passing in CI.
//   2. "Type a message" placeholder — already covered by `empty_state_shows_hint`.
//   3. Help/keybinding hints render — covered by the two tests below.

/// Layer 3: verify the hint bar renders keybinding hints in the startup buffer.
/// Replaces: "tmux capture-pane" grep for "ctrl+o" in `scripts/tmux-test.sh`.
#[test]
fn startup_render_contains_keybinding_hints() {
    let mut state = AppState::default();
    connect_model(&mut state);
    state.config.vim_mode = false;
    // hint_text() produces "ctrl+o expand/collapse · [mode hints] · ctrl+c quit"
    // Render at a wide size so the hint bar is not truncated.
    let backend = TestBackend::new(100, 24);
    let mut terminal = Terminal::new(backend).unwrap();
    terminal.draw(|f| view(f, &mut state)).unwrap();
    let buf = terminal.backend().buffer();
    let content: String = (0..buf.area().height)
        .map(|y| {
            (0..buf.area().width)
                .map(|x| buf[(x, y)].symbol())
                .collect::<String>()
        })
        .collect();
    assert!(
        content.contains("ctrl+o"),
        "Hint bar should contain ctrl+o hint: {}",
        content
    );
    assert!(
        content.contains("ctrl+c quit"),
        "Hint bar should contain ctrl+c quit: {}",
        content
    );
}

/// Layer 3: verify the startup render buffer contains both the input placeholder
/// and the hint bar. This catches rendering regressions without needing tmux.
#[test]
fn startup_render_buffer_has_placeholder_and_hints() {
    let mut state = AppState::default();
    connect_model(&mut state);
    let backend = TestBackend::new(80, 24);
    let mut terminal = Terminal::new(backend).unwrap();
    terminal.draw(|f| view(f, &mut state)).unwrap();
    let buf = terminal.backend().buffer();
    let all_content: String = (0..buf.area().height)
        .map(|y| {
            (0..buf.area().width)
                .map(|x| buf[(x, y)].symbol())
                .collect::<String>()
        })
        .collect();
    assert!(
        all_content.contains("Type a message"),
        "Startup render must show placeholder text"
    );
    // The hint bar always shows ctrl+o (expand/collapse).
    assert!(
        all_content.contains("ctrl+o"),
        "Startup render must show ctrl+o hint: {}",
        all_content
    );
}
