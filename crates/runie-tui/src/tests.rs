//! TUI smoke tests — verify view() integrates with core state

#[cfg(test)]
mod code_blocks;
#[cfg(test)]
mod color_restraint;
#[cfg(test)]
mod colors;
#[cfg(test)]
mod markdown;
#[cfg(test)]
mod status_right;
#[cfg(test)]
mod style_dsl;
#[cfg(test)]
mod theme;

use crate::ui::view;
use ratatui::{backend::TestBackend, Terminal};
use runie_core::{AppState, Event};

fn draw_state(state: &mut AppState) -> String {
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

fn line_text(buf: &ratatui::buffer::Buffer, y: u16) -> String {
    (0..buf.area().width)
        .map(|x| buf[(x, y)].symbol().to_string())
        .collect()
}

fn push_message(state: &mut AppState, role: runie_core::Role, content: &str, id: &str) {
    state.session.messages.push(runie_core::ChatMessage {
        role,
        content: content.to_string(),
        timestamp: 0.0,
        id: id.to_string(),
        ..Default::default()
    });
}

#[test]
fn empty_state_renders_input_prompt() {
    let mut state = AppState::default();
    let content = draw_state(&mut state);
    // Empty state should show input prompt with $
    assert!(
        content.contains("❯ "),
        "Empty state should show input prompt"
    );
}

#[test]
fn user_message_renders() {
    let mut state = AppState::default();
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
    state.streaming = true;
    state.update(Event::AgentThinking {
        id: "req.0".to_string(),
    });
    state.update(Event::AgentThoughtDone {
        id: "req.0".to_string(),
    });
    state.update(Event::AgentResponse {
        id: "req.0".to_string(),
        content: "Hello".to_string(),
    });
    let content = draw_state(&mut state);
    assert!(content.contains("→ Hello"), "Should render agent prefix");
}

#[test]
fn tool_done_renders() {
    let mut state = AppState::default();
    state.update(Event::AgentToolStart {
        id: "req.0".to_string(),
        name: "list_files".to_string(),
    });
    state.update(Event::AgentToolEnd {
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
    state.update(Event::Input('T'));
    state.update(Event::Submit);
    state.update(Event::Reset);
    let content = draw_state(&mut state);
    // After reset, should only show input prompt, no user messages
    // Count occurrences of "❯ " - should be exactly 1 (input prompt)
    let count = content.matches("❯ ").count();
    assert_eq!(
        count, 1,
        "Reset should clear messages, keep only input prompt"
    );
}

#[test]
fn at_file_picker_panel_renders() {
    let mut state = AppState::default();
    state.update(Event::Input('@'));
    assert!(
        matches!(
            state.open_dialog,
            Some(runie_core::commands::DialogState::PanelStack(_))
        ),
        "@ should open PanelStack dialog"
    );
    let backend = TestBackend::new(40, 15);
    let mut terminal = Terminal::new(backend).unwrap();
    terminal.draw(|f| view(f, &mut state)).unwrap();
    let buf = terminal.backend().buffer();
    let has_dialog = (0..buf.area().height as usize).any(|y| {
        let line: String = (0..buf.area().width)
            .map(|x| buf[(x, y as u16)].symbol().to_string())
            .collect();
        line.contains("Files")
    });
    assert!(has_dialog, "Should render Files dialog");
}

#[test]
fn long_message_wraps_to_multiple_lines() {
    let mut state = AppState::default();
    let long = "a".repeat(100);
    state.session.messages.push(runie_core::ChatMessage {
        role: runie_core::Role::User,
        content: long.clone(),
        timestamp: 0.0,
        id: "req.0".to_string(),
        ..Default::default()
    });
    let backend = TestBackend::new(30, 20);
    let mut terminal = Terminal::new(backend).unwrap();
    terminal.draw(|f| view(f, &mut state)).unwrap();
    let buf = terminal.backend().buffer();
    let content_lines: Vec<usize> = (0..buf.area().height as usize)
        .filter(|y| {
            let line: String = (0..buf.area().width)
                .map(|x| buf[(x, *y as u16)].symbol().to_string())
                .collect();
            line.contains('a')
        })
        .collect();
    assert!(
        content_lines.len() >= 2,
        "Long message should wrap to multiple lines, got {} lines with content",
        content_lines.len()
    );
}

#[test]
fn wrapping_preserves_prefix_on_first_line_only() {
    let mut state = AppState::default();
    state.session.messages.push(runie_core::ChatMessage {
        role: runie_core::Role::Assistant,
        content: "word ".repeat(20),
        timestamp: 0.0,
        id: "req.0".to_string(),
        ..Default::default()
    });
    let backend = TestBackend::new(25, 20);
    let mut terminal = Terminal::new(backend).unwrap();
    terminal.draw(|f| view(f, &mut state)).unwrap();
    let buf = terminal.backend().buffer();
    let lines_with_agent: Vec<String> = (0..buf.area().height as usize)
        .filter_map(|y| {
            let line: String = (0..buf.area().width)
                .map(|x| buf[(x, y as u16)].symbol().to_string())
                .collect();
            if line.contains("→ ") {
                Some(line.trim().to_string())
            } else {
                None
            }
        })
        .collect();
    assert_eq!(
        lines_with_agent.len(),
        1,
        "Only first wrapped line should contain → prefix"
    );
}

#[test]
fn wrapping_respects_panel_width() {
    let mut state = AppState::default();
    state.session.messages.push(runie_core::ChatMessage {
        role: runie_core::Role::User,
        content: "x".repeat(50),
        timestamp: 0.0,
        id: "req.0".to_string(),
        ..Default::default()
    });
    let width = 20u16;
    let backend = TestBackend::new(width, 10);
    let mut terminal = Terminal::new(backend).unwrap();
    terminal.draw(|f| view(f, &mut state)).unwrap();
    let buf = terminal.backend().buffer();
    for y in 0..buf.area().height {
        let line: String = (0..buf.area().width)
            .map(|x| buf[(x, y)].symbol().to_string())
            .collect();
        if line.trim().starts_with("$") || line.trim().starts_with("x") {
            let visible_len = line.trim_end().chars().count();
            assert!(
                visible_len <= width as usize,
                "Wrapped line {} chars exceeds width {}: {:?}",
                visible_len,
                width,
                line
            );
        }
    }
}

#[test]
fn piece_is_last_character_on_right_side() {
    let mut state = AppState::default();
    state.config.current_provider = "openai".to_string();
    state.config.current_model = "gpt-4o".to_string();
    push_message(&mut state, runie_core::Role::User, "hello", "req.0");
    state.agent.turn_active = true;
    state.agent.turn_started_at = Some(std::time::Instant::now());
    let backend = TestBackend::new(60, 10);
    let mut terminal = Terminal::new(backend).unwrap();
    terminal.draw(|f| view(f, &mut state)).unwrap();
    let buf = terminal.backend().buffer();
    // Find the status bar line and check chess piece is the last character
    let mut right_line: String = String::new();
    for y in 0..buf.area().height {
<<<<<<< HEAD
        let line: String = (0..buf.area().width)
            .map(|x| buf[(x, y)].symbol().to_string())
            .collect::<String>()
            .trim_end()
            .to_string();
        if line.rfind('⛀').is_some()
            || line.rfind('⛁').is_some()
            || line.rfind('⛂').is_some()
            || line.rfind('⛃').is_some()
        {
            right_line = line.clone();
            break;
        }
    }
    // Check piece is last non-space character
    let line_trimmed = right_line.trim_end();
    let last_char = line_trimmed.chars().last().unwrap();
    assert!(
        last_char == '⛀' || last_char == '⛁' || last_char == '⛂' || last_char == '⛃',
        "Chess piece must be last character (before any trailing spaces), got last char: '{}' in '{}'",
        last_char,
        right_line
=======
        let line = line_text(buf, y);
        if working_pos.is_none() && line.find("Working").is_some() {
            working_pos = line.find("Working");
        }
        if ctx_pos.is_none() && line.find("/128k").is_some() {
            ctx_pos = line.find("/128k");
        }
    }
    let working_pos = working_pos.expect("Should find 'Working' in status bar");
    let ctx_pos = ctx_pos.expect("Should find '/128k' context usage in status bar");
    assert!(working_pos < ctx_pos, "Working ({}) should be left of context ({})", working_pos, ctx_pos);
    assert!(
        ctx_pos > 30,
        "Context usage should appear on right side of status bar, got pos {}",
        ctx_pos
>>>>>>> review
    );
}

#[test]
fn empty_state_shows_hint() {
    let mut state = AppState::default();
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

#[test]
fn input_cursor_renders_at_position() {
    let mut state = AppState::default();
    state.input.input = "hello".to_string();
    state.input.cursor_pos = 2;
    let backend = TestBackend::new(60, 20);
    let mut terminal = Terminal::new(backend).unwrap();
    terminal.draw(|f| view(f, &mut state)).unwrap();
    let buf = terminal.backend().buffer();
    // Find the input line by looking for the "❯ he" prefix, then verify cursor
    let mut cursor_cell = None;
    for y in 0..buf.area().height {
        for x in 0..buf.area().width.saturating_sub(4) {
            let prefix: String = (x..x + 4).map(|cx| buf[(cx, y)].symbol()).collect();
            if prefix == "❯ he" {
                // cursor is at position 2, so 'l' is the next char after "❯ he"
                cursor_cell = Some(buf[(x + 4, y)].clone());
                break;
            }
        }
        if cursor_cell.is_some() {
            break;
        }
    }
    let cell = cursor_cell.expect("should find input line with ❯ he");
    assert_eq!(cell.symbol(), "l", "Cursor should be on 'l' at position 2");
    assert!(
        cell.style().bg.is_some(),
        "Cursor should have background color set"
    );
}

#[test]
fn status_shows_provider_model() {
    let mut state = AppState::default();
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
    state.config.thinking_level = runie_core::model::ThinkingLevel::Medium;
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
        content.contains("Think: medium"),
        "Status should show thinking level badge: {}",
        content
    );
}

#[test]
fn status_hides_thinking_badge_when_off() {
    let mut state = AppState::default();
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
fn palette_renders_centered() {
    let mut state = AppState::default();
    state.update(Event::ToggleCommandPalette);
    let backend = TestBackend::new(60, 20);
    let mut terminal = Terminal::new(backend).unwrap();
    terminal.draw(|f| view(f, &mut state)).unwrap();
    let buf = terminal.backend().buffer();
    let has_title = (0..buf.area().height).any(|y| {
        let line: String = (0..buf.area().width)
            .map(|x| buf[(x, y)].symbol().to_string())
            .collect();
        line.contains("Commands")
    });
    assert!(has_title, "Palette should render with 'Commands' title");
}

#[test]
fn palette_shows_categories() {
    let mut state = AppState::default();
    state.update(Event::ToggleCommandPalette);
    let backend = TestBackend::new(60, 20);
    let mut terminal = Terminal::new(backend).unwrap();
    terminal.draw(|f| view(f, &mut state)).unwrap();
    let buf = terminal.backend().buffer();
    let has_category = (0..buf.area().height).any(|y| {
        let line: String = (0..buf.area().width)
            .map(|x| buf[(x, y)].symbol().to_string())
            .collect();
        line.contains("Session") || line.contains("Model") || line.contains("System")
    });
    assert!(has_category, "Palette should show category headers");
}

#[test]
fn palette_highlights_selected() {
    let mut state = AppState::default();
    state.update(Event::ToggleCommandPalette);
    let backend = TestBackend::new(60, 20);
    let mut terminal = Terminal::new(backend).unwrap();
    terminal.draw(|f| view(f, &mut state)).unwrap();
    let buf = terminal.backend().buffer();
    // The first item should be selected; check that some cell has bg set
    let has_selected_bg = (0..buf.area().height)
        .any(|y| (0..buf.area().width).any(|x| buf[(x, y)].style().bg.is_some()));
    assert!(
        has_selected_bg,
        "Selected palette item should have background color"
    );
}

#[test]
fn turn_complete_renders_after_tool_flow() {
    let mut state = AppState::default();
    push_message(&mut state, runie_core::Role::User, "list files", "req.0");
    push_message(&mut state, runie_core::Role::Thought, "Thinking...", "req.1#thought.0");
    push_message(&mut state, runie_core::Role::Tool, "Ran list_files 0.5s", "tool.req.1.1");
    push_message(&mut state, runie_core::Role::Assistant, "Here are the files", "req.1");
    push_message(
        &mut state,
        runie_core::Role::TurnComplete,
        "Turn completed in 3.2s",
        "req.1",
    );
    let content = draw_state(&mut state);
    assert!(
        content.contains("Turn completed in 3.2s"),
        "TurnComplete should render in TUI after tool flow"
    );
}

#[cfg(test)]
mod render;
