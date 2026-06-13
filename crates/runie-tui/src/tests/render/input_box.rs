use super::find_input_box_bounds;
use crate::ui::view;
use ratatui::{backend::TestBackend, Terminal};
use runie_core::{AppState, Event};

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
    state.input.input = "/theme".to_string();
    state.update(Event::Submit);
    assert!(matches!(
        state.open_dialog,
        Some(runie_core::commands::DialogState::PanelStack(_))
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
    let backend = TestBackend::new(60, 20);
    let mut terminal = Terminal::new(backend).unwrap();
    terminal.draw(|f| view(f, &mut state)).unwrap();
    let buf = terminal.backend().buffer();

    let hints_y = (0..buf.area().height)
        .find(|&y| {
            let line: String = (0..buf.area().width)
                .map(|x| buf[(x, y)].symbol())
                .collect();
            line.contains("ctrl+shift+e")
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
fn input_box_shrinks_when_content_reduced() {
    let _lock = crate::theme::test_lock();
    let mut state = AppState::default();
    state.input.input = "line1\nline2\nline3\nline4".to_string();
    let backend = TestBackend::new(60, 30);
    let mut terminal = Terminal::new(backend).unwrap();
    terminal.draw(|f| view(f, &mut state)).unwrap();
    {
        let buf = terminal.backend().buffer();
        let (top1, bottom1) = find_input_box_bounds(buf);
        assert!(bottom1 > top1);

        state.input.input = "single".to_string();
        terminal.draw(|f| view(f, &mut state)).unwrap();
        let buf = terminal.backend().buffer();
        let (top2, _bottom2) = find_input_box_bounds(buf);
        assert!(
            top2 > top1,
            "top should move down when input shrinks: top1={} top2={}",
            top1,
            top2
        );
    }
}
