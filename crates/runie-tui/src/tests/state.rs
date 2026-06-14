//! State-driven rendering tests that exercise the full view pipeline.

use crate::tests::draw_state;
use crate::ui::view;
use ratatui::{backend::TestBackend, Terminal};
use runie_core::{AppState, ChatMessage, Event, Role};

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

fn find_status_line_with_piece(buf: &ratatui::buffer::Buffer) -> Option<String> {
    for y in 0..buf.area().height {
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
            return Some(line);
        }
    }
    None
}

#[test]
fn piece_is_last_character_on_right_side() {
    let mut state = AppState::default();
    state.config.current_provider = "openai".to_string();
    state.config.current_model = "gpt-4o".to_string();
    state.session.messages.push(runie_core::ChatMessage {
        role: runie_core::Role::User,
        content: "hello".to_string(),
        timestamp: 0.0,
        id: "req.0".to_string(),
        ..Default::default()
    });
    state.agent.turn_active = true;
    state.agent.turn_started_at = Some(std::time::Instant::now());
    let backend = TestBackend::new(60, 10);
    let mut terminal = Terminal::new(backend).unwrap();
    terminal.draw(|f| view(f, &mut state)).unwrap();
    let buf = terminal.backend().buffer();
    let right_line = find_status_line_with_piece(buf).expect("status line with piece not found");
    let last_char = right_line.trim_end().chars().last().unwrap();
    assert!(
        matches!(last_char, '⛀' | '⛁' | '⛂' | '⛃'),
        "Chess piece must be last character, got last char: '{}' in '{}'",
        last_char,
        right_line
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
    let mut cursor_cell = None;
    for y in 0..buf.area().height {
        for x in 0..buf.area().width.saturating_sub(4) {
            let prefix: String = (x..x + 4).map(|cx| buf[(cx, y)].symbol()).collect();
            if prefix == "❯ he" {
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
fn dialog_title_has_single_space_inside_border() {
    let mut state = AppState::default();
    state.update(Event::ToggleCommandPalette);
    let backend = TestBackend::new(60, 20);
    let mut terminal = Terminal::new(backend).unwrap();
    terminal.draw(|f| view(f, &mut state)).unwrap();
    let buf = terminal.backend().buffer();

    let mut title_row = None;
    for y in 0..buf.area().height {
        let line: String = (0..buf.area().width)
            .map(|x| buf[(x, y)].symbol().to_string())
            .collect();
        if line.contains("Commands") {
            title_row = Some(line);
            break;
        }
    }
    let row = title_row.expect("dialog title row not found");
    assert!(
        row.contains("╭ Commands ") || row.contains(" Commands "),
        "dialog title should have exactly one space inside the border, got: {:?}",
        row
    );
    assert!(
        !row.contains("╭  Commands") && !row.contains("Commands  ─"),
        "dialog title must not have double spaces inside the border, got: {:?}",
        row
    );
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
    let has_selected_bg = (0..buf.area().height)
        .any(|y| (0..buf.area().width).any(|x| buf[(x, y)].style().bg.is_some()));
    assert!(
        has_selected_bg,
        "Selected palette item should have background color"
    );
}

fn state_with_tool_flow() -> AppState {
    let mut state = AppState::default();
    let messages = [
        (Role::User, "list files", "req.0"),
        (Role::Thought, "Thinking...", "req.1#thought.0"),
        (Role::Tool, "Ran list_files 0.5s", "tool.req.1.1"),
        (Role::Assistant, "Here are the files", "req.1"),
        (Role::TurnComplete, "Turn completed in 3.2s", "req.1"),
    ];
    for (role, content, id) in messages {
        state.session.messages.push(ChatMessage {
            role,
            content: content.to_string(),
            timestamp: 0.0,
            id: id.to_string(),
            ..Default::default()
        });
    }
    state
}

#[test]
fn turn_complete_renders_after_tool_flow() {
    let mut state = state_with_tool_flow();
    let content = draw_state(&mut state);
    assert!(
        content.contains("Turn completed in 3.2s"),
        "TurnComplete should render in TUI after tool flow"
    );
}
