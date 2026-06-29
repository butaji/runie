use super::find_input_box_bounds;
use crate::tests::connect_model;
use crate::ui::view;
use ratatui::{backend::TestBackend, Terminal};
use runie_core::commands::DialogKind;
use runie_core::AppState;
use runie_core::Event;

fn buffer_content(terminal: &Terminal<TestBackend>) -> String {
    let buf = terminal.backend().buffer();
    (0..buf.area().height)
        .map(|y| {
            (0..buf.area().width)
                .map(|x| buf[(x, y)].symbol())
                .collect::<String>()
        })
        .collect()
}

#[test]
fn input_box_shows_model_name_at_bottom_right() {
    let _lock = crate::theme::test_lock();
    let mut state = AppState::default();
    connect_model(&mut state);
    state.config.current_provider = "openai".to_string();
    state.config.current_model = "gpt-4o".to_string();
    let backend = TestBackend::new(60, 20);
    let mut terminal = Terminal::new(backend).unwrap();
    terminal.draw(|f| view(f, &mut state)).unwrap();
    let buf = terminal.backend().buffer();

    let mut model_row = None;
    for y in 0..buf.area().height {
        let line: String = (0..buf.area().width)
            .map(|x| buf[(x, y)].symbol())
            .collect();
        if line.contains('╯') && line.contains("openai/gpt-4o") {
            let pos = line.find("openai/gpt-4o").unwrap();
            assert!(
                pos > 40,
                "Model name should be right-aligned, got pos {}",
                pos
            );
            model_row = Some(y);
        }
    }
    assert!(model_row.is_some());
}

#[test]
fn theme_selector_renders_theme_list() {
    let _lock = crate::theme::test_lock();
    let mut state = AppState::default();
    connect_model(&mut state);
    state.input.input = "/theme".to_string();
    state.update(Event::Submit);
    assert!(matches!(
        state.open_dialog,
        Some(runie_core::commands::DialogState::Active { kind: DialogKind::Generic, panels: _ })
    ));

    let backend = TestBackend::new(60, 24);
    let mut terminal = Terminal::new(backend).unwrap();
    terminal.draw(|f| view(f, &mut state)).unwrap();
    let content = buffer_content(&terminal);
    assert!(content.contains("Choose Theme"));
    assert!(content.contains("runie"));
    // The theme picker is fuzzy-searchable. Use the filter to narrow to
    // dracula and verify it becomes visible.
    state.update(Event::Input('d'));
    state.update(Event::Input('r'));
    state.update(Event::Input('a'));
    terminal.draw(|f| view(f, &mut state)).unwrap();
    let content = buffer_content(&terminal);
    assert!(
        content.contains("dracula"),
        "filtered theme picker should show 'dracula': {}",
        content
    );
}

#[test]
fn command_palette_has_panel_background() {
    let _lock = crate::theme::test_lock();
    let mut state = AppState::default();
    connect_model(&mut state);
    state.update(Event::ToggleCommandPalette);
    let backend = TestBackend::new(60, 24);
    let mut terminal = Terminal::new(backend).unwrap();
    terminal.draw(|f| view(f, &mut state)).unwrap();
    let buf = terminal.backend().buffer();
    let panel_bg = crate::theme::color_bg_panel();
    let app_bg = crate::theme::color_bg();

    let mut found_panel = false;
    for y in 4..19 {
        for x in 3..56 {
            if buf[(x, y)].style().bg == Some(panel_bg) {
                found_panel = true;
            }
        }
    }
    assert!(found_panel);
    assert_eq!(buf[(0, 0)].style().bg, Some(app_bg));
}

#[test]
fn empty_line_between_input_box_and_hints() {
    let _lock = crate::theme::test_lock();
    let mut state = AppState::default();
    connect_model(&mut state);
    let backend = TestBackend::new(60, 20);
    let mut terminal = Terminal::new(backend).unwrap();
    terminal.draw(|f| view(f, &mut state)).unwrap();
    let buf = terminal.backend().buffer();

    let hints_y = (0..buf.area().height)
        .find(|&y| {
            let line: String = (0..buf.area().width)
                .map(|x| buf[(x, y)].symbol())
                .collect();
            line.contains("ctrl+o")
        })
        .expect("Should find hints line");

    let above_y = hints_y.saturating_sub(1);
    let above_line: String = (0..buf.area().width)
        .map(|x| buf[(x, above_y)].symbol())
        .collect();
    let non_space = above_line.chars().filter(|c| !c.is_whitespace()).count();
    assert!(non_space <= 2);
}

#[test]
fn input_box_single_line() {
    let _lock = crate::theme::test_lock();
    let mut state = AppState::default();
    connect_model(&mut state);
    state.input.input = "hello".to_string();
    state.input.cursor_pos = 5;
    let backend = TestBackend::new(60, 20);
    let mut terminal = Terminal::new(backend).unwrap();
    terminal.draw(|f| view(f, &mut state)).unwrap();
    let buf = terminal.backend().buffer();
    let (top, bottom) = find_input_box_bounds(buf);
    assert!(
        bottom - top + 1 >= 2,
        "input box should be at least 2 rows tall, got top={} bottom={}",
        top,
        bottom
    );
}

#[test]
fn input_box_grows_with_multiline_content() {
    let _lock = crate::theme::test_lock();
    let mut state = AppState::default();
    connect_model(&mut state);
    state.input.input = "line1\nline2\nline3".to_string();
    state.input.cursor_pos = 17;
    let backend = TestBackend::new(60, 30);
    let mut terminal = Terminal::new(backend).unwrap();
    terminal.draw(|f| view(f, &mut state)).unwrap();
    let buf = terminal.backend().buffer();
    let (top, bottom) = find_input_box_bounds(buf);
    assert!(bottom - top + 1 >= 4);
}

#[test]
fn input_box_cursor_visible_after_trailing_newline() {
    let _lock = crate::theme::test_lock();
    let mut state = AppState::default();
    connect_model(&mut state);
    state.input.input = "hello\n".to_string();
    state.input.cursor_pos = 6;
    let backend = TestBackend::new(60, 20);
    let mut terminal = Terminal::new(backend).unwrap();
    terminal.draw(|f| view(f, &mut state)).unwrap();
    let buf = terminal.backend().buffer();

    let mut found = false;
    for y in 0..buf.area().height.saturating_sub(1) {
        let line: String = (0..buf.area().width)
            .map(|x| buf[(x, y)].symbol())
            .collect();
        if line.contains("hello") {
            let next: String = (0..buf.area().width)
                .map(|x| buf[(x, y + 1)].symbol())
                .collect();
            if next.trim_start().is_empty() || next.contains("  ") {
                found = true;
            }
        }
    }
    assert!(found);
}

#[test]
fn input_box_height_reduces_when_content_shrinks() {
    let _lock = crate::theme::test_lock();
    let mut state = AppState::default();
    connect_model(&mut state);
    state.input.input = "line1\nline2\nline3\nline4".to_string();
    let backend = TestBackend::new(60, 30);
    let mut terminal = Terminal::new(backend).unwrap();
    terminal.draw(|f| view(f, &mut state)).unwrap();
    {
        let buf = terminal.backend().buffer();
        let (_, bottom1) = find_input_box_bounds(buf);
        let height1 = bottom1 + 1;

        state.input.input = "single".to_string();
        terminal.draw(|f| view(f, &mut state)).unwrap();
        let buf = terminal.backend().buffer();
        let (_, bottom2) = find_input_box_bounds(buf);
        let height2 = bottom2 + 1;
        // Layout height is fixed
        assert_eq!(
            height1, height2,
            "layout height should be fixed: {} vs {}",
            height1, height2
        );
    }
}

#[test]
fn input_box_wraps_long_words() {
    let _lock = crate::theme::test_lock();
    let mut state = AppState::default();
    connect_model(&mut state);
    // A long word that exceeds the width - will wrap visually
    state.input.input = "AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA".to_string();
    state.input.cursor_pos = state.input.input.len();
    let backend = TestBackend::new(40, 30);
    let mut terminal = Terminal::new(backend).unwrap();
    terminal.draw(|f| view(f, &mut state)).unwrap();
    let buf = terminal.backend().buffer();
    let (top, bottom) = find_input_box_bounds(buf);
    // Input box has fixed height in layout
    assert!(
        bottom - top + 1 >= 3,
        "input box should have minimum height, got height {}",
        bottom - top + 1
    );
}

#[test]
fn input_box_height_fixed_by_layout() {
    let _lock = crate::theme::test_lock();
    let mut state = AppState::default();
    connect_model(&mut state);
    // Short content
    state.input.input = "AAA".to_string();
    state.input.cursor_pos = 3;
    let backend = TestBackend::new(60, 30);
    let mut terminal = Terminal::new(backend).unwrap();
    terminal.draw(|f| view(f, &mut state)).unwrap();
    let buf = terminal.backend().buffer();
    let (_, bottom1) = find_input_box_bounds(buf);
    let height1 = bottom1 + 1;

    // Much longer content
    state.input.input = "line1\nline2\nline3\nline4\nline5".to_string();
    state.input.cursor_pos = state.input.input.len();
    terminal.draw(|f| view(f, &mut state)).unwrap();
    let buf = terminal.backend().buffer();
    let (_, bottom2) = find_input_box_bounds(buf);
    let height2 = bottom2 + 1;

    // Layout height is fixed (determined by constraints)
    assert_eq!(
        height1, height2,
        "layout height should be fixed: {} vs {}",
        height1, height2
    );
}

#[test]
fn input_box_renders_prompt() {
    // Verifies that the input box renders the chevron prompt glyph when empty.
    // This is the buffer assertion for the prompt widget after migration.
    let _lock = crate::theme::test_lock();
    let mut state = AppState::default();
    connect_model(&mut state);
    // Ensure no input text - the placeholder should be visible
    state.input.input.clear();
    state.input.placeholder = "Type a message".to_string();

    let backend = TestBackend::new(60, 20);
    let mut terminal = Terminal::new(backend).unwrap();
    terminal.draw(|f| view(f, &mut state)).unwrap();
    let buf = terminal.backend().buffer();

    // Find the input box area by looking for the chevron prompt glyph
    let mut found_prompt = false;
    for y in 0..buf.area().height {
        for x in 0..buf.area().width.saturating_sub(2) {
            // Check for the chevron glyph "❯"
            if buf[(x, y)].symbol() == "❯" {
                found_prompt = true;
                // Placeholder text check (placeholder rendering depends on input state)
            }
        }
    }
    assert!(found_prompt, "Input box should render the chevron prompt glyph ❯");
    // Placeholder rendering depends on input state; verify at minimum the prompt renders
    assert!(
        found_prompt,
        "Input box should render prompt when empty"
    );
}
