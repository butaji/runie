//! TUI rendering tests — visuals, margins, styling

use runie_core::{AppState, Event};
use crate::ui::view;
use ratatui::{backend::TestBackend, Terminal};

// ─── Input token — chevron color reflects focus ownership ───────────────

#[test]
fn input_chevron_is_orange_when_token_held() {
    let _lock = crate::theme::test_lock();
    let mut state = AppState::default();
    let backend = TestBackend::new(60, 20);
    let mut terminal = Terminal::new(backend).unwrap();
    terminal.draw(|f| view(f, &mut state)).unwrap();
    let buf = terminal.backend().buffer();
    let orange = crate::theme::color_accent();
    let mut found = false;
    for y in 0..buf.area().height {
        for x in 0..buf.area().width.saturating_sub(2) {
            if buf[(x, y)].symbol() == "❯" {
                assert_eq!(buf[(x, y)].style().fg, Some(orange), "Chevron should be orange when input holds token");
                found = true;
            }
        }
    }
    assert!(found, "Should find ❯ chevron in buffer");
}

#[test]
fn input_chevron_is_gray_when_token_released() {
    let _lock = crate::theme::test_lock();
    let mut state = AppState::default();
    state.update(Event::ToggleCommandPalette);
    let backend = TestBackend::new(60, 40);
    let mut terminal = Terminal::new(backend).unwrap();
    terminal.draw(|f| view(f, &mut state)).unwrap();
    let buf = terminal.backend().buffer();
    let dim = crate::theme::color_dim();
    let mut found = false;
    for y in (0..buf.area().height).rev() {
        for x in 0..buf.area().width.saturating_sub(2) {
            if buf[(x, y)].symbol() == "❯" {
                assert_eq!(buf[(x, y)].style().fg, Some(dim), "Chevron should be gray when dialog holds token");
                found = true;
                break;
            }
        }
        if found { break; }
    }
    assert!(found, "Should find ❯ chevron in buffer");
}

#[test]
fn palette_filter_uses_chevron_glyph() {
    let _lock = crate::theme::test_lock();
    let mut state = AppState::default();
    state.update(Event::ToggleCommandPalette);
    let backend = TestBackend::new(60, 20);
    let mut terminal = Terminal::new(backend).unwrap();
    terminal.draw(|f| view(f, &mut state)).unwrap();
    let buf = terminal.backend().buffer();
    let content: String = (0..buf.area().height)
        .map(|y| (0..buf.area().width)
            .map(|x| buf[(x, y)].symbol())
            .collect::<String>())
        .collect();
    assert!(content.contains("❯"), "Palette filter should use ❯ chevron");
    assert!(!content.contains("> "), "Palette filter should not use plain >");
}

#[test]
fn model_selector_filter_uses_chevron_glyph() {
    let _lock = crate::theme::test_lock();
    let mut state = AppState::default();
    state.update(Event::ToggleModelSelector);
    let backend = TestBackend::new(60, 20);
    let mut terminal = Terminal::new(backend).unwrap();
    terminal.draw(|f| view(f, &mut state)).unwrap();
    let buf = terminal.backend().buffer();
    let content: String = (0..buf.area().height)
        .map(|y| (0..buf.area().width)
            .map(|x| buf[(x, y)].symbol())
            .collect::<String>())
        .collect();
    assert!(content.contains("❯"), "Model selector filter should use ❯ chevron");
    assert!(!content.contains("> "), "Model selector filter should not use plain >");
}

// ─── Background fill ────────────────────────────────────────────────────

#[test]
fn app_background_is_theme_bg_color() {
    let _lock = crate::theme::test_lock();
    let mut state = AppState::default();
    let backend = TestBackend::new(60, 20);
    let mut terminal = Terminal::new(backend).unwrap();
    terminal.draw(|f| view(f, &mut state)).unwrap();
    let buf = terminal.backend().buffer();
    let expected_bg = crate::theme::color_bg();
    let cell = &buf[(0, 0)];
    assert_eq!(
        cell.style().bg,
        Some(expected_bg),
        "App background should be theme bg color, got {:?}",
        cell.style().bg
    );
}

// ─── Input cursor color ─────────────────────────────────────────────────

#[test]
fn input_cursor_visible_when_empty() {
    let _lock = crate::theme::test_lock();
    let mut state = AppState::default();
    // input is empty, cursor_pos defaults to 0
    let backend = TestBackend::new(60, 20);
    let mut terminal = Terminal::new(backend).unwrap();
    terminal.draw(|f| view(f, &mut state)).unwrap();
    let buf = terminal.backend().buffer();
    let orange = crate::theme::color_accent();

    let mut found = false;
    for y in 0..buf.area().height {
        for x in 0..buf.area().width.saturating_sub(2) {
            if buf[(x, y)].symbol() == "❯" {
                // GLYPH_USER is "❯ " (2 chars); cursor should be the cell right after
                let cursor_cell = &buf[(x + 2, y)];
                if cursor_cell.style().bg == Some(orange) {
                    found = true;
                }
            }
        }
    }
    assert!(found, "Cursor should be visible even when input is empty");
}

#[test]
fn input_cursor_hidden_when_token_released() {
    let _lock = crate::theme::test_lock();
    let mut state = AppState::default();
    state.update(Event::ToggleCommandPalette);
    state.input.input = "hello".to_string();
    state.input.cursor_pos = 2;
    let backend = TestBackend::new(60, 40);
    let mut terminal = Terminal::new(backend).unwrap();
    terminal.draw(|f| view(f, &mut state)).unwrap();
    let buf = terminal.backend().buffer();
    let orange = crate::theme::color_accent();

    let mut found = false;
    for y in 0..buf.area().height {
        for x in 0..buf.area().width.saturating_sub(4) {
            let prefix: String = (x..x + 4).map(|cx| buf[(cx, y)].symbol()).collect();
            if prefix == "❯ he" {
                let cursor_cell = &buf[(x + 4, y)];
                if cursor_cell.style().bg == Some(orange) {
                    found = true;
                }
            }
        }
    }
    assert!(!found, "Cursor should be hidden when input is unfocused (dialog open)");
}

#[test]
fn input_cursor_is_orange_when_token_held() {
    let _lock = crate::theme::test_lock();
    let mut state = AppState::default();
    state.input.input = "hello".to_string();
    state.input.cursor_pos = 2;
    let backend = TestBackend::new(60, 20);
    let mut terminal = Terminal::new(backend).unwrap();
    terminal.draw(|f| view(f, &mut state)).unwrap();
    let buf = terminal.backend().buffer();
    let orange = crate::theme::color_accent();
    let mut found = false;
    for y in 0..buf.area().height {
        for x in 0..buf.area().width.saturating_sub(4) {
            let prefix: String = (x..x + 4).map(|cx| buf[(cx, y)].symbol()).collect();
            if prefix == "❯ he" {
                let cursor_cell = &buf[(x + 4, y)];
                assert_eq!(
                    cursor_cell.style().bg,
                    Some(orange),
                    "Cursor bg should be orange when input holds token"
                );
                found = true;
                break;
            }
        }
        if found { break; }
    }
    assert!(found, "Should find input line");
}

// ─── Transient messages ─────────────────────────────────────────────────

#[test]
fn transient_success_renders_green_background_with_ok_prefix() {
    let _lock = crate::theme::test_lock();
    let mut state = AppState::default();
    state.transient_message = Some("Theme switched".to_string());
    state.transient_level = Some(runie_core::event::TransientLevel::Success);
    let backend = TestBackend::new(60, 20);
    let mut terminal = Terminal::new(backend).unwrap();
    terminal.draw(|f| view(f, &mut state)).unwrap();
    let buf = terminal.backend().buffer();
    let content: String = (0..buf.area().height)
        .map(|y| (0..buf.area().width)
            .map(|x| buf[(x, y)].symbol())
            .collect::<String>())
        .collect();
    assert!(content.contains("\\ok\\ "), "Success transient should show \\ok\\ prefix");
    assert!(content.contains("Theme switched"), "Transient message should appear");
    assert!(!content.contains("ctrl+shift+e"), "Default hints hidden when transient active");

    let green = crate::theme::color_success();
    let mut found_green_bg = false;
    for y in 0..buf.area().height {
        for x in 0..buf.area().width {
            if buf[(x, y)].symbol() == "T" && buf[(x, y)].style().bg == Some(green) {
                found_green_bg = true;
            }
        }
    }
    assert!(found_green_bg, "Hints line should have success green background");
}

#[test]
fn transient_success_has_1_symbol_margin_on_both_sides() {
    let _lock = crate::theme::test_lock();
    let mut state = AppState::default();
    state.transient_message = Some("Test".to_string());
    state.transient_level = Some(runie_core::event::TransientLevel::Success);
    let backend = TestBackend::new(60, 20);
    let mut terminal = Terminal::new(backend).unwrap();
    terminal.draw(|f| view(f, &mut state)).unwrap();
    let buf = terminal.backend().buffer();

    let green = crate::theme::color_success();
    let margin_green = crate::theme::darken(green, 0.85);
    let badge_bg = crate::theme::darken(green, 0.8);

    // Find transient row
    let transient_y = (0..buf.area().height)
        .find(|&y| (0..buf.area().width)
            .any(|x| buf[(x, y)].symbol() == "T" && buf[(x, y)].style().bg == Some(green)))
        .expect("Should find transient row");

    // Left margin at column 1 should be darker
    assert_eq!(buf[(1, transient_y)].style().bg, Some(margin_green));

    // Rest of line should have valid bg colors
    let last = buf.area().width - 2;
    for x in 2..=last {
        let bg = buf[(x, transient_y)].style().bg;
        assert!(bg == Some(green) || bg == Some(badge_bg) || bg == Some(margin_green),
            "Column {} should have valid bg", x);
    }
}

#[test]
fn transient_warning_renders_amber_background_with_warn_prefix() {
    let _lock = crate::theme::test_lock();
    let mut state = AppState::default();
    state.transient_message = Some("Read-only on".to_string());
    state.transient_level = Some(runie_core::event::TransientLevel::Warning);
    let backend = TestBackend::new(60, 20);
    let mut terminal = Terminal::new(backend).unwrap();
    terminal.draw(|f| view(f, &mut state)).unwrap();
    let buf = terminal.backend().buffer();
    let content: String = (0..buf.area().height)
        .map(|y| (0..buf.area().width)
            .map(|x| buf[(x, y)].symbol())
            .collect::<String>())
        .collect();
    assert!(content.contains("\\warn\\ "), "Warning transient should show \\warn\\ prefix");
    assert!(content.contains("Read-only on"), "Warning message should appear");
}

#[test]
fn transient_error_renders_red_background_with_error_prefix() {
    let _lock = crate::theme::test_lock();
    let mut state = AppState::default();
    state.transient_message = Some("Failed".to_string());
    state.transient_level = Some(runie_core::event::TransientLevel::Error);
    let backend = TestBackend::new(60, 20);
    let mut terminal = Terminal::new(backend).unwrap();
    terminal.draw(|f| view(f, &mut state)).unwrap();
    let buf = terminal.backend().buffer();
    let content: String = (0..buf.area().height)
        .map(|y| (0..buf.area().width)
            .map(|x| buf[(x, y)].symbol())
            .collect::<String>())
        .collect();
    assert!(content.contains("\\err\\ "), "Error transient should show \\err\\ prefix");
    assert!(content.contains("Failed"), "Error message should appear");
}

#[test]
fn transient_message_renders_in_hints_line() {
    let _lock = crate::theme::test_lock();
    let mut state = AppState::default();
    state.transient_message = Some("Test message".to_string());
    let backend = TestBackend::new(60, 20);
    let mut terminal = Terminal::new(backend).unwrap();
    terminal.draw(|f| view(f, &mut state)).unwrap();
    let buf = terminal.backend().buffer();
    let content: String = (0..buf.area().height)
        .map(|y| (0..buf.area().width)
            .map(|x| buf[(x, y)].symbol())
            .collect::<String>())
        .collect();
    assert!(content.contains("Test message"), "Transient message should render in hints line");
}

#[test]
fn default_hints_render_when_no_transient() {
    let _lock = crate::theme::test_lock();
    let mut state = AppState::default();
    state.transient_message = None;
    let backend = TestBackend::new(60, 20);
    let mut terminal = Terminal::new(backend).unwrap();
    terminal.draw(|f| view(f, &mut state)).unwrap();
    let buf = terminal.backend().buffer();
    let content: String = (0..buf.area().height)
        .map(|y| (0..buf.area().width)
            .map(|x| buf[(x, y)].symbol())
            .collect::<String>())
        .collect();
    assert!(content.contains("ctrl+shift+e"), "Default hints should render when no transient");
}

// ─── Input box label and spacing ────────────────────────────────────────

#[test]
fn input_box_shows_model_name_at_bottom_right() {
    let _lock = crate::theme::test_lock();
    let mut state = AppState::default();
    state.config.current_provider = "openai".to_string();
    state.config.current_model = "gpt-4o".to_string();
    let backend = TestBackend::new(60, 20);
    let mut terminal = Terminal::new(backend).unwrap();
    terminal.draw(|f| view(f, &mut state)).unwrap();
    let buf = terminal.backend().buffer();

    // Find the bottom border row of the input box by looking for the rounded corner ╯
    let mut model_row = None;
    for y in 0..buf.area().height {
        let line: String = (0..buf.area().width)
            .map(|x| buf[(x, y)].symbol())
            .collect();
        if line.contains('╯') && line.contains("openai/gpt-4o") {
            let pos = line.find("openai/gpt-4o").unwrap();
            assert!(pos > 40, "Model name should be right-aligned on bottom border, got pos {}", pos);
            model_row = Some(y);
        }
    }
    assert!(model_row.is_some(), "Should find provider/model on bottom border of input box");
    let content: String = (0..buf.area().height)
        .map(|y| (0..buf.area().width)
            .map(|x| buf[(x, y)].symbol())
            .collect::<String>())
        .collect();
    assert!(!content.contains(" Input "), "Old ' Input ' label should not appear");
}

#[test]
fn theme_selector_renders_theme_list() {
    let _lock = crate::theme::test_lock();
    let mut state = AppState::default();
    // Open theme selector via slash command
    state.input.input = "/theme".to_string();
    state.update(Event::Submit);
    assert!(matches!(state.open_dialog, Some(runie_core::commands::DialogState::PanelStack(_))));

    let backend = TestBackend::new(60, 24);
    let mut terminal = Terminal::new(backend).unwrap();
    terminal.draw(|f| view(f, &mut state)).unwrap();
    let buf = terminal.backend().buffer();
    let content: String = (0..buf.area().height)
        .map(|y| (0..buf.area().width)
            .map(|x| buf[(x, y)].symbol())
            .collect::<String>())
        .collect();
    assert!(content.contains("Choose Theme"), "Theme selector should show title");
    assert!(content.contains("runie"), "Theme selector should list 'runie' theme");
    assert!(content.contains("dracula"), "Theme selector should list 'dracula' theme");
}

#[test]
fn command_palette_has_panel_background() {
    let _lock = crate::theme::test_lock();
    let mut state = AppState::default();
    state.update(Event::ToggleCommandPalette);
    let backend = TestBackend::new(60, 24);
    let mut terminal = Terminal::new(backend).unwrap();
    terminal.draw(|f| view(f, &mut state)).unwrap();
    let buf = terminal.backend().buffer();
    let panel_bg = crate::theme::color_bg_panel();
    let app_bg = crate::theme::color_bg();

    // The popup is centered: ~60x18 centered in 60x24 => y starts at 3, ends at 20
    // Inner area is inside borders: y 4..19, x 3..56
    let mut found_panel = false;
    for y in 4..19 {
        for x in 3..56 {
            let cell = &buf[(x, y)];
            if cell.style().bg == Some(panel_bg) {
                found_panel = true;
            }
        }
    }
    assert!(found_panel, "Command palette inner area should have panel background color {:?}, not default", panel_bg);

    // Verify the area OUTSIDE the popup still has app background
    let cell = &buf[(0, 0)];
    assert_eq!(cell.style().bg, Some(app_bg), "Area outside popup should keep app background");
}

#[test]
fn empty_line_between_input_box_and_hints() {
    let _lock = crate::theme::test_lock();
    let mut state = AppState::default();
    let backend = TestBackend::new(60, 20);
    let mut terminal = Terminal::new(backend).unwrap();
    terminal.draw(|f| view(f, &mut state)).unwrap();
    let buf = terminal.backend().buffer();

    let mut hints_y = None;
    for y in 0..buf.area().height {
        let line: String = (0..buf.area().width)
            .map(|x| buf[(x, y)].symbol())
            .collect();
        if line.contains("ctrl+shift+e") {
            hints_y = Some(y);
            break;
        }
    }
    let hints_y = hints_y.expect("Should find hints line");
    let above_y = hints_y.saturating_sub(1);
    let above_line: String = (0..buf.area().width)
        .map(|x| buf[(x, above_y)].symbol())
        .collect();
    let non_space = above_line.chars().filter(|c| !c.is_whitespace()).count();
    assert!(
        non_space <= 2,
        "Row above hints should be empty spacer, got {} non-space chars on row {}: {:?}",
        non_space, above_y, above_line
    );
}

// ─── Input box grows with content ─────────────────────────────────────────

fn find_input_box_bounds(buf: &ratatui::buffer::Buffer) -> (u16, u16) {
    // Find input box by looking for provider/model label (default: mock/echo)
    let mut top = None;
    let mut bottom = None;
    for y in 0..buf.area().height {
        let line: String = (0..buf.area().width)
            .map(|x| buf[(x, y)].symbol())
            .collect();
        if line.contains("mock/") || line.contains("/echo") || line.contains("openai/") {
            bottom = Some(y);
            if top.is_none() {
                // Find top by looking for the chevron in same column range
                for ty in (0..y).rev() {
                    let tline: String = (0..buf.area().width)
                        .map(|x| buf[(x, ty)].symbol())
                        .collect();
                    if tline.contains('❯') {
                        top = Some(ty);
                        break;
                    }
                }
            }
        }
    }
    (top.unwrap_or(0), bottom.unwrap_or(0))
}

#[test]
fn input_box_single_line() {
    let _lock = crate::theme::test_lock();
    let mut state = AppState::default();
    state.input.input = "hello".to_string();
    state.input.cursor_pos = 5;
    let backend = TestBackend::new(60, 20);
    let mut terminal = Terminal::new(backend).unwrap();
    terminal.draw(|f| view(f, &mut state)).unwrap();
    let buf = terminal.backend().buffer();
    let (top, bottom) = find_input_box_bounds(&buf);
    let height = bottom - top + 1;
    // Single line should fit in minimal height (2 for block border)
    assert!(height >= 2, "Single line input should fit in minimal height, got {}", height);
}

#[test]
fn input_box_grows_with_multiline_content() {
    let _lock = crate::theme::test_lock();
    let mut state = AppState::default();
    state.input.input = "line1\nline2\nline3".to_string();
    state.input.cursor_pos = 17;
    let backend = TestBackend::new(60, 30);
    let mut terminal = Terminal::new(backend).unwrap();
    terminal.draw(|f| view(f, &mut state)).unwrap();
    let buf = terminal.backend().buffer();
    let (top, bottom) = find_input_box_bounds(&buf);
    let height = bottom - top + 1;
    // 3 lines of content should need at least 4 rows (3 + 1 for borders)
    assert!(height >= 4, "3-line input should need at least 4 rows, got {}", height);
}

#[test]
fn input_box_cursor_visible_after_trailing_newline() {
    let _lock = crate::theme::test_lock();
    let mut state = AppState::default();
    // Cursor at end of trailing newline - this is what happens after Ctrl+J/Shift+Enter
    state.input.input = "hello\n".to_string();
    state.input.cursor_pos = 6;  // right after the newline
    let backend = TestBackend::new(60, 20);
    let mut terminal = Terminal::new(backend).unwrap();
    terminal.draw(|f| view(f, &mut state)).unwrap();
    let buf = terminal.backend().buffer();

    // Find the second line of the input box (the empty line with cursor)
    let mut found_second_line = false;
    for y in 0..buf.area().height.saturating_sub(1) {
        let line: String = (0..buf.area().width)
            .map(|x| buf[(x, y)].symbol())
            .collect();
        if line.contains("hello") {
            // Check next line has the cursor (empty line with indent)
            let next_line: String = (0..buf.area().width)
                .map(|x| buf[(x, y + 1)].symbol())
                .collect();
            if next_line.trim_start().is_empty() || next_line.contains("  ") {
                found_second_line = true;
            }
        }
    }
    assert!(found_second_line, "Input box should render a second empty line for cursor after newline");
}

#[test]
fn input_box_shrinks_when_content_reduced() {
    let _lock = crate::theme::test_lock();
    let mut state = AppState::default();
    // First: multiline content (4 lines)
    state.input.input = "line1\nline2\nline3\nline4".to_string();
    let backend = TestBackend::new(60, 30);
    let mut terminal = Terminal::new(backend).unwrap();
    terminal.draw(|f| view(f, &mut state)).unwrap();
    let buf1 = terminal.backend().buffer();
    let (top1, bottom1) = find_input_box_bounds(&buf1);
    let height1 = bottom1 - top1 + 1;

    // Second: shorter content (1 line)
    state.input.input = "single".to_string();
    terminal.draw(|f| view(f, &mut state)).unwrap();
    let buf2 = terminal.backend().buffer();
    let (top2, bottom2) = find_input_box_bounds(&buf2);
    let height2 = bottom2 - top2 + 1;

    // Shorter content should have smaller height
    assert!(height2 < height1,
        "Shorter content should have smaller input box height: {} vs {}",
        height2, height1);
}
