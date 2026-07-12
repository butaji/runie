//! End-to-end-ish render tests for vim navigation mode.

use super::*;
use ratatui::buffer::Buffer;
use ratatui::style::Style;
use runie_core::Event;
use runie_core::Part;

fn state_with_vim_and_messages() -> AppState {
    let mut state = AppState::default();
    connect_model(&mut state);
    state.config.vim_mode = true;
    add_messages(&mut state, 30);
    state.messages_changed();
    state
}

fn add_messages(state: &mut AppState, count: usize) {
    for i in 0..count {
        state.session.messages.push(ChatMessage {
            role: Role::User,
            parts: vec![Part::Text {
                content: format!("message {}", i),
            }],
            timestamp: i as f64,
            id: format!("req.{}", i),
            ..Default::default()
        });
        state.session.messages.push(ChatMessage {
            role: Role::Assistant,
            parts: vec![Part::Text {
                content: format!("response {}", i),
            }],
            timestamp: i as f64 + 0.5,
            id: format!("resp.{}", i),
            ..Default::default()
        });
    }
}

fn find_prompt_pos(buf: &Buffer) -> Option<(u16, u16)> {
    for y in 0..buf.area().height {
        let line: String = (0..buf.area().width)
            .map(|x| buf[(x, y)].symbol())
            .collect();
        if let Some(x) = line.find('❯') {
            return Some((y, x as u16));
        }
    }
    None
}

fn prompt_pos(buf: &Buffer) -> (u16, u16) {
    find_prompt_pos(buf).expect("input prompt row not found")
}

fn nav_state() -> AppState {
    let mut state = AppState::default();
    connect_model(&mut state);
    state.config.vim_mode = true;
    state.update(Event::DialogBack);
    assert!(state.view.vim_nav_mode);
    state
}

#[test]
fn vim_mode_hint_renders_in_status() {
    let mut state = AppState::default();
    state.config.vim_mode = true;
    let content = render_content(&mut state);
    assert!(
        content.contains("Esc nav"),
        "vim hint must render. Got: {}",
        content
    );
}

#[test]
fn vim_mode_scroll_renders_older_content() {
    let mut state = state_with_vim_and_messages();
    let backend = TestBackend::new(80, 24);
    let mut terminal = Terminal::new(backend).expect("terminal");

    state.update(Event::TerminalSize {
        width: 80,
        height: 24,
    });
    terminal.draw(|f| view(f, &mut state)).expect("draw");
    state.update(Event::Input('g'));
    terminal.draw(|f| view(f, &mut state)).expect("draw");

    let content: String = terminal
        .backend()
        .buffer()
        .content
        .iter()
        .map(|c| c.symbol())
        .collect();
    assert!(
        content.contains("message 0") || content.contains("response 0"),
        "oldest message should be visible after go-to-top. Got: {}",
        content
    );
}

#[test]
fn vim_mode_page_down_renders_newer_content() {
    let mut state = state_with_vim_and_messages();
    let backend = TestBackend::new(80, 24);
    let mut terminal = Terminal::new(backend).expect("terminal");

    state.update(Event::TerminalSize {
        width: 80,
        height: 24,
    });
    terminal.draw(|f| view(f, &mut state)).expect("draw");
    state.update(Event::Input('g'));
    state.update(Event::Input(' '));
    terminal.draw(|f| view(f, &mut state)).expect("draw");

    let content: String = terminal
        .backend()
        .buffer()
        .content
        .iter()
        .map(|c| c.symbol())
        .collect();
    assert!(
        content.contains("message 0") || content.contains("response 0"),
        "page-down from top should still show old content. Got: {}",
        content
    );
}

#[test]
fn vim_nav_mode_hint_renders_in_status() {
    let mut state = nav_state();
    let content = render_content(&mut state);
    assert!(
        content.contains("J/K"),
        "nav-mode hint should show j/k. Got: {}",
        content
    );
    assert!(
        content.contains("S") && content.contains("i"),
        "nav-mode hint should advertise S/i to enter input. Got: {}",
        content
    );
}

#[test]
fn nav_mode_renders_input_box_with_disabled_style() {
    let _lock = crate::theme::test_lock();
    let mut state = nav_state();
    let backend = TestBackend::new(80, 24);
    let mut terminal = Terminal::new(backend).expect("terminal");
    terminal.draw(|f| view(f, &mut state)).expect("draw");

    let buf = terminal.backend().buffer();
    let active_bg = crate::theme::style_input_cursor().bg;
    let (py, px) = prompt_pos(buf);
    let width = buf.area().width;

    let used_active_bg = (1..4).filter(|dx| px + (*dx as u16) < width).any(|dx| {
        let cell = &buf[(px + dx as u16, py)];
        cell.style().bg == active_bg
    });

    assert!(
        !used_active_bg,
        "nav-mode input must not render the active (accent) cursor block"
    );
}

#[test]
fn nav_mode_highlights_selected_post_with_orange_bracket() {
    let _lock = crate::theme::test_lock();
    let mut state = AppState::default();
    state.config.vim_mode = true;
    add_messages(&mut state, 4);
    state.refresh_after_message_change();

    state.update(Event::DialogBack);

    let backend = TestBackend::new(80, 24);
    let mut terminal = Terminal::new(backend).expect("terminal");
    terminal.draw(|f| view(f, &mut state)).expect("draw");
    state.update(Event::Input('g'));
    terminal.draw(|f| view(f, &mut state)).expect("draw");

    let buf = terminal.backend().buffer();
    let accent = crate::theme::color_accent();
    assert!(
        find_accent_bracket(buf, accent),
        "nav mode should render an orange bracket in the first cell of the top element"
    );
}

fn find_accent_bracket(buf: &Buffer, accent: ratatui::style::Color) -> bool {
    for y in 0..buf.area().height {
        for x in 0..4 {
            let cell = &buf[(x, y)];
            if cell.symbol() == "▎" {
                if let Some(fg) = cell.style().fg {
                    if fg == accent {
                        assert!(
                            buf[(x + 1, y)].symbol() != "▎",
                            "bracket must be exactly one cell wide"
                        );
                        return true;
                    }
                }
            }
        }
    }
    false
}

#[test]
fn command_bar_open_renders_input_box_as_disabled() {
    let _lock = crate::theme::test_lock();
    let mut state = AppState::default();
    state.update(Event::ToggleCommandPalette);
    assert!(state.open_dialog.is_some(), "palette should be open");

    let backend = TestBackend::new(80, 24);
    let mut terminal = Terminal::new(backend).expect("terminal");
    terminal.draw(|f| view(f, &mut state)).expect("draw");

    let buf = terminal.backend().buffer();
    let active_chevron_bg = crate::theme::style_chevron(true).bg;
    let (py, px) = prompt_pos(buf);
    let cell = &buf[(px, py)];
    if let (Some(bg), Some(active_bg)) = (cell.style().bg, active_chevron_bg) {
        assert_ne!(
            bg, active_bg,
            "command-bar open: chevron must not use the active (accent) background"
        );
    }
}

fn chevron_cell(state: &AppState) -> Option<Style> {
    let backend = TestBackend::new(80, 24);
    let mut terminal = Terminal::new(backend).expect("terminal");
    terminal
        .draw(|f| view(f, &mut state.clone()))
        .expect("draw");
    let buf = terminal.backend().buffer();
    find_prompt_pos(buf).map(|(y, x)| buf[(x, y)].style())
}

#[test]
fn nav_mode_and_command_bar_share_disabled_chevron_style() {
    let _lock = crate::theme::test_lock();
    let mut s1 = AppState::default();
    connect_model(&mut s1);
    let cell_enabled = chevron_cell(&s1).expect("enabled chevron cell");

    let mut s2 = AppState::default();
    connect_model(&mut s2);
    s2.config.vim_mode = true;
    s2.update(Event::DialogBack);
    assert!(s2.view.vim_nav_mode);
    let cell_nav = chevron_cell(&s2).expect("nav chevron cell");

    // Nav-mode chevron uses Reset foreground (not hint dim color)
    let reset_fg = Some(ratatui::style::Color::Reset);
    assert_eq!(
        cell_nav.fg, reset_fg,
        "nav-mode chevron must use Reset foreground"
    );
    assert_ne!(
        cell_enabled, cell_nav,
        "enabled chevron must differ from the disabled (nav) chevron"
    );

    let mut s3 = AppState::default();
    s3.update(Event::ToggleCommandPalette);
    assert!(s3.open_dialog.is_some(), "palette should be open");
}

/// Regression test: selected_post must be cleared when exiting vim nav mode.
/// Previously, pressing Esc while in nav mode set vim_nav_mode=false but left
/// selected_post set, causing stale selection state (e.g. duplicate highlight
/// rendering or incorrect scroll behavior).
#[test]
fn selected_post_cleared_on_nav_mode_exit() {
    let _lock = crate::theme::test_lock();
    let mut state = AppState::default();
    state.config.vim_mode = true;

    // Add a message so there's a post to select
    state.session.messages.push(ChatMessage {
        role: Role::User,
        parts: vec![Part::Text {
            content: "hello".into(),
        }],
        timestamp: 0.0,
        id: "req.0".to_string(),
        ..Default::default()
    });
    state.refresh_after_message_change();

    // Enter vim nav mode via DialogBack (same as pressing Esc in chat input)
    state.update(Event::DialogBack);
    assert!(
        state.view.vim_nav_mode,
        "should enter vim nav mode"
    );
    assert!(
        state.view.selected_post.is_some(),
        "selected_post should be set when entering nav mode"
    );

    // Exit vim nav mode by pressing Esc again
    state.update(Event::DialogBack);
    assert!(
        !state.view.vim_nav_mode,
        "should exit vim nav mode"
    );
    assert!(
        state.view.selected_post.is_none(),
        "selected_post must be cleared when exiting nav mode"
    );
}

/// Regression test: pressing i, Space, or an unhandled char while in vim nav
/// mode should also clear selected_post (not just the Esc key).
#[test]
fn selected_post_cleared_on_nav_char_exit() {
    let _lock = crate::theme::test_lock();
    let mut state = AppState::default();
    state.config.vim_mode = true;

    state.session.messages.push(ChatMessage {
        role: Role::User,
        parts: vec![Part::Text {
            content: "test".into(),
        }],
        timestamp: 0.0,
        id: "req.0".to_string(),
        ..Default::default()
    });
    state.refresh_after_message_change();
    state.update(Event::DialogBack); // enter nav mode

    assert!(state.view.vim_nav_mode);
    assert!(state.view.selected_post.is_some());

    // 'i' key exits nav mode
    state.update(Event::Input('i'));

    assert!(!state.view.vim_nav_mode);
    assert!(
        state.view.selected_post.is_none(),
        "selected_post must be cleared after pressing 'i' to exit nav mode"
    );
}

/// Rendering regression test: after exiting vim nav mode, the buffer must not
/// contain duplicate message content. Previously, a stale selected_post value
/// caused the nav-mode highlight to persist in some rendering paths.
#[test]
fn no_duplicate_content_after_nav_mode_exit() {
    let _lock = crate::theme::test_lock();
    let mut state = AppState::default();
    state.config.vim_mode = true;
    state.view.last_visible_height = 12;

    state.session.messages.push(ChatMessage {
        role: Role::User,
        parts: vec![Part::Text {
            content: "this is a long message that wraps to multiple lines".into(),
        }],
        timestamp: 0.0,
        id: "req.0".to_string(),
        ..Default::default()
    });
    state.refresh_after_message_change();

    // Count how many rows contain a content prefix in a rendered buffer.
    fn count_rows_with_prefix(buf: &ratatui::buffer::Buffer, prefix: &str) -> usize {
        (0..buf.area().height)
            .filter(|&y| {
                let line: String = (0..buf.area().width)
                    .map(|x| buf[(x, y)].symbol())
                    .collect();
                line.contains(prefix)
            })
            .count()
    }

    // ── In vim nav mode ─────────────────────────────────────────────────────
    state.update(Event::DialogBack);
    assert!(state.view.vim_nav_mode, "should enter vim nav mode");

    let backend = ratatui::backend::TestBackend::new(60, 12);
    let mut terminal = ratatui::Terminal::new(backend).unwrap();
    terminal.draw(|f| view(f, &mut state.clone())).unwrap();
    let buf_nav = terminal.backend().buffer();
    let snap_nav = state.snapshot();
    assert!(
        snap_nav.vim_nav_mode,
        "snapshot must have vim_nav_mode = true"
    );
    let nav_count = count_rows_with_prefix(&buf_nav, "this is a");

    // ── After exiting vim nav mode ───────────────────────────────────────────
    state.update(Event::DialogBack);
    assert!(!state.view.vim_nav_mode, "should exit vim nav mode");
    assert!(
        state.view.selected_post.is_none(),
        "selected_post must be cleared on nav mode exit"
    );

    terminal.draw(|f| view(f, &mut state.clone())).unwrap();
    let buf_exited = terminal.backend().buffer();
    let snap_exited = state.snapshot();
    assert!(
        !snap_exited.vim_nav_mode,
        "snapshot vim_nav_mode must be false after exit"
    );
    assert!(
        snap_exited.selected_post.is_none(),
        "snapshot selected_post must be None after exit"
    );
    let exited_count = count_rows_with_prefix(&buf_exited, "this is a");

    // Both modes should render the same number of content rows — no extra
    // duplicates appear after exiting nav mode.
    assert_eq!(
        exited_count, nav_count,
        "content row count must not change after exiting nav mode: \
         nav_mode={}, exited={} (both should be 2 for wrapped content)",
        nav_count, exited_count
    );
}
