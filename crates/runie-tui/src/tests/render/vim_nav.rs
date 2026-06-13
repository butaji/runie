use crate::ui::view;
use ratatui::{backend::TestBackend, Terminal};
use runie_core::{AppState, ChatMessage, Event, Role};

#[test]
fn vim_nav_mode_shows_orange_bracket_around_selected_post() {
    let _lock = crate::theme::test_lock();
    let mut state = AppState::default();
    state.config.vim_mode = true;

    // Add several messages so post-level navigation has work to do.
    for i in 0..10 {
        state.session.messages.push(ChatMessage {
            role: Role::User,
            content: format!("message {}", i),
            timestamp: i as f64,
            id: format!("req.{}", i),
            ..Default::default()
        });
        state.session.messages.push(ChatMessage {
            role: Role::Assistant,
            content: format!("reply {}", i),
            timestamp: i as f64 + 0.5,
            id: format!("resp.{}", i),
            ..Default::default()
        });
    }
    state.messages_changed();
    state.ensure_fresh();
    state.view.last_visible_height = 10;

    // Enter vim nav mode and jump to the top post.
    state.update(Event::DialogBack);
    assert!(state.vim_nav_mode);
    state.update(Event::Input('g'));

    let backend = TestBackend::new(60, 20);
    let mut terminal = Terminal::new(backend).unwrap();
    terminal.draw(|f| view(f, &mut state)).unwrap();
    let buf = terminal.backend().buffer();
    let accent = crate::theme::color_accent();

    // Find the bracket in the leftmost column of the terminal (x == 0). A thin accent vertical line is rendered on the left edge of the leftmost column of every selected-post row.
    let mut found_line = false;
    for y in 0..buf.area().height {
        let cell = &buf[(0, y)];
        let is_bracket = cell.symbol() == "▎"
            || cell.symbol() == "╰"
            || cell.symbol() == "╭"
            || cell.symbol() == "├";
        if is_bracket && cell.style().fg == Some(accent) {
            found_line = true;
            // Make sure we do not paint beyond the first cell: the
            // second column of the same row must keep its original
            // text styling, not the orange bracket.
            let next = &buf[(1, y)];
            assert!(
                next.symbol() != "▎",
                "orange bracket must stay in the first cell only"
            );
        }
    }
    assert!(
        found_line,
        "vim nav mode should render an orange bracket around the selected post"
    );
}

#[test]
fn vim_nav_mode_bracket_spans_post_elements() {
    let _lock = crate::theme::test_lock();
    let mut state = AppState::default();
    state.config.vim_mode = true;

    state.session.messages.push(ChatMessage {
        role: Role::User,
        content: "hello".to_string(),
        timestamp: 0.0,
        id: "req.0".to_string(),
        ..Default::default()
    });
    state.messages_changed();
    state.ensure_fresh();
    state.view.last_visible_height = 10;

    // Enter vim nav mode. The user message post is selected.
    state.update(Event::DialogBack);
    assert!(state.vim_nav_mode);

    let backend = TestBackend::new(60, 20);
    let mut terminal = Terminal::new(backend).unwrap();
    terminal.draw(|f| view(f, &mut state)).unwrap();
    let buf = terminal.backend().buffer();
    let accent = crate::theme::color_accent();

    // The selected post (user message + following spacer) should have
    // bracket cells in the first column. We do not assert exact rows
    // because wrapping depends on width, but at least two rows should
    // be highlighted (message body + spacer).
    let bracket_rows: Vec<u16> = (0..buf.area().height)
        .filter(|&y| {
            let cell = &buf[(0, y)];
            (cell.symbol() == "▎") && cell.style().fg == Some(accent)
        })
        .collect();
    assert!(
        bracket_rows.len() >= 2,
        "bracket should span at least the message and spacer rows of the selected post"
    );
}

#[test]
fn vim_nav_mode_bracket_around_long_system_welcome_post() {
    let _lock = crate::theme::test_lock();
    let mut state = AppState::default();
    state.config.vim_mode = true;

    state.session.messages.push(ChatMessage {
        role: Role::System,
        content: "Welcome to runie in someproject.\n\nThis project is not yet trusted. \
                  Run /trust to enable write tools, or /untrust to enforce read-only mode."
            .to_string(),
        timestamp: 0.0,
        id: "trust_welcome".to_string(),
        ..Default::default()
    });
    state.session.messages.push(ChatMessage {
        role: Role::User,
        content: "list files".to_string(),
        timestamp: 1.0,
        id: "req.0".to_string(),
        ..Default::default()
    });
    state.messages_changed();
    state.ensure_fresh();
    state.view.last_visible_height = 10;

    // Enter nav mode at the bottom, then move up to the system welcome post.
    state.update(Event::DialogBack);
    assert!(state.vim_nav_mode);
    state.update(Event::Input('k'));
    assert_eq!(state.view.selected_post, Some(0));

    let backend = TestBackend::new(60, 20);
    let mut terminal = Terminal::new(backend).unwrap();
    terminal.draw(|f| view(f, &mut state)).unwrap();
    let buf = terminal.backend().buffer();
    let accent = crate::theme::color_accent();

    let bracket_rows: Vec<u16> = (0..buf.area().height)
        .filter(|&y| {
            let cell = &buf[(0, y)];
            (cell.symbol() == "▎") && cell.style().fg == Some(accent)
        })
        .collect();
    assert!(
        !bracket_rows.is_empty(),
        "selected system welcome post should have an orange bracket"
    );
}

#[test]
fn vim_nav_mode_bracket_absent_when_not_in_nav_mode() {
    let _lock = crate::theme::test_lock();
    let mut state = AppState::default();
    state.config.vim_mode = true;

    state.session.messages.push(ChatMessage {
        role: Role::User,
        content: "hello".to_string(),
        timestamp: 0.0,
        id: "req.0".to_string(),
        ..Default::default()
    });
    state.messages_changed();

    let backend = TestBackend::new(60, 20);
    let mut terminal = Terminal::new(backend).unwrap();
    terminal.draw(|f| view(f, &mut state)).unwrap();
    let buf = terminal.backend().buffer();

    let accent = crate::theme::color_accent();
    let has_bracket = (0..buf.area().height).any(|y| {
        let cell = &buf[(0, y)];
        let is_bracket = cell.symbol() == "▎"
            || cell.symbol() == "╰"
            || cell.symbol() == "╭"
            || cell.symbol() == "├";
        is_bracket && cell.style().fg == Some(accent)
    });
    assert!(
        !has_bracket,
        "orange selection bracket should only appear in vim nav mode"
    );
}

#[test]
fn nav_mode_bracket_matches_wrapped_post_height() {
    let _lock = crate::theme::test_lock();
    let mut state = AppState::default();
    state.config.vim_mode = true;

    // The system welcome post is a long ThoughtMarker. At 40 columns it
    // wraps to multiple terminal rows, so this is a regression guard for
    // the bracket height matching the *rendered* height of the post.
    state.session.messages.push(ChatMessage {
        role: Role::System,
        content: "Welcome to runie in someproject.\n\nThis project is not yet trusted. \
                  Run /trust to enable write tools, or /untrust to enforce read-only mode."
            .to_string(),
        timestamp: 0.0,
        id: "trust_welcome".to_string(),
        ..Default::default()
    });
    state.session.messages.push(ChatMessage {
        role: Role::User,
        content: "hi".to_string(),
        timestamp: 1.0,
        id: "req.0".to_string(),
        ..Default::default()
    });
    state.messages_changed();
    state.ensure_fresh();
    state.view.last_visible_height = 12;

    // Enter nav mode and move up to the system welcome post.
    state.update(Event::DialogBack);
    assert!(state.vim_nav_mode);
    state.update(Event::Input('k'));
    assert_eq!(state.view.selected_post, Some(0));

    // Use a tall terminal so the bracket's bottom corner (drawn in the
    // trailing spacer row) is fully visible.
    let backend = TestBackend::new(40, 24);
    let mut terminal = Terminal::new(backend).unwrap();
    terminal.draw(|f| view(f, &mut state)).unwrap();
    let buf = terminal.backend().buffer();
    let accent = crate::theme::color_accent();

    let bracket_rows: Vec<u16> = (0..buf.area().height)
        .filter(|&y| {
            let cell = &buf[(0, y)];
            (cell.symbol() == "▎") && cell.style().fg == Some(accent)
        })
        .collect();
    assert!(
        bracket_rows.len() >= 2,
        "wrapped system welcome post should have a multi-row bracket, got {:?}",
        bracket_rows
    );

    // The bracket should form a `[`: top corner, body bars, bottom corner.
    let first = *bracket_rows.first().unwrap();
    let last = *bracket_rows.last().unwrap();
    assert_eq!(
        buf[(0, first)].symbol(),
        "▎",
        "first visible row of a fully visible post should use the top corner glyph"
    );
    assert_eq!(
        buf[(0, last)].symbol(),
        "▎",
        "last visible row of a fully visible post should use the bottom corner glyph"
    );

    // The bracket must be exactly one cell wide.
    for &y in &bracket_rows {
        let next = &buf[(1, y)];
        assert!(
            next.symbol() != "▎",
            "bracket must not spill into the second column"
        );
    }
}

#[test]
fn nav_mode_bracket_for_one_line_user_post_is_three_rows() {
    let _lock = crate::theme::test_lock();
    let mut state = AppState::default();
    state.config.vim_mode = true;

    state.session.messages.push(ChatMessage {
        role: Role::User,
        content: "x".to_string(),
        timestamp: 0.0,
        id: "req.0".to_string(),
        ..Default::default()
    });
    state.messages_changed();
    state.ensure_fresh();

    state.update(Event::DialogBack);
    assert!(state.vim_nav_mode);

    let backend = TestBackend::new(40, 12);
    let mut terminal = Terminal::new(backend).unwrap();
    terminal.draw(|f| view(f, &mut state)).unwrap();
    let buf = terminal.backend().buffer();
    let accent = crate::theme::color_accent();

    let rows: Vec<u16> = (0..buf.area().height)
        .filter(|&y| {
            let cell = &buf[(0, y)];
            (cell.symbol() == "▎") && cell.style().fg == Some(accent)
        })
        .collect();
    assert_eq!(
        rows.len(),
        3,
        "one-line user post should have a 3-row bracket from margins"
    );
    assert_eq!(buf[(0, rows[0])].symbol(), "▎");
    assert_eq!(buf[(0, rows[1])].symbol(), "▎");
    assert_eq!(buf[(0, rows[2])].symbol(), "▎");
}

#[test]
fn nav_mode_bracket_for_one_line_non_user_post_is_three_rows() {
    let _lock = crate::theme::test_lock();
    let mut state = AppState::default();
    state.config.vim_mode = true;

    state.session.messages.push(ChatMessage {
        role: Role::User,
        content: "hi".to_string(),
        timestamp: 0.0,
        id: "req.0".to_string(),
        ..Default::default()
    });
    state.session.messages.push(ChatMessage {
        role: Role::Assistant,
        content: "x".to_string(),
        timestamp: 1.0,
        id: "resp.0".to_string(),
        provider: "mock".to_string(),
    });
    state.messages_changed();
    state.ensure_fresh();

    state.update(Event::DialogBack);
    assert!(state.vim_nav_mode);
    // The newest post is the one-line agent response.

    let backend = TestBackend::new(40, 12);
    let mut terminal = Terminal::new(backend).unwrap();
    terminal.draw(|f| view(f, &mut state)).unwrap();
    let buf = terminal.backend().buffer();
    let accent = crate::theme::color_accent();

    let rows: Vec<u16> = (0..buf.area().height)
        .filter(|&y| {
            let cell = &buf[(0, y)];
            (cell.symbol() == "▎") && cell.style().fg == Some(accent)
        })
        .collect();
    assert_eq!(
        rows.len(),
        3,
        "one-line non-user post should have a 3-row bracket (spacer + content + spacer)"
    );
    assert_eq!(buf[(0, rows[0])].symbol(), "▎");
    assert_eq!(buf[(0, rows[1])].symbol(), "▎");
    assert_eq!(buf[(0, rows[2])].symbol(), "▎");
}

#[test]
fn nav_mode_selected_post_has_accent_background() {
    let _lock = crate::theme::test_lock();
    let mut state = AppState::default();
    state.config.vim_mode = true;

    state.session.messages.push(ChatMessage {
        role: Role::User,
        content: "hello".to_string(),
        timestamp: 0.0,
        id: "req.0".to_string(),
        ..Default::default()
    });
    state.session.messages.push(ChatMessage {
        role: Role::Assistant,
        content: "world".to_string(),
        timestamp: 1.0,
        id: "resp.0".to_string(),
        provider: "mock".to_string(),
    });
    state.messages_changed();
    state.ensure_fresh();

    state.update(Event::DialogBack);
    assert!(state.vim_nav_mode);

    let backend = TestBackend::new(40, 12);
    let mut terminal = Terminal::new(backend).unwrap();
    terminal.draw(|f| view(f, &mut state)).unwrap();
    let buf = terminal.backend().buffer();

    let accent_bg = crate::theme::color_accent_bg();
    let mut line_rows = Vec::new();
    for y in 0..buf.area().height {
        // The thin selection line at column 0 marks the selected post rows.
        if buf[(0, y)].symbol() == "▎" {
            line_rows.push(y);
        }
    }
    assert!(
        !line_rows.is_empty(),
        "selected post should have a visible left line"
    );
    // The accent background must cover exactly the same rows as the left line,
    // spanning the full terminal width so no margins remain uncolored.
    let width = buf.area().width;
    for y in line_rows {
        let left_bg = buf[(0, y)].style().bg == Some(accent_bg);
        let right_bg = buf[(width - 1, y)].style().bg == Some(accent_bg);
        assert!(
            left_bg && right_bg,
            "row {y} selection background must cover the whole line, including margins"
        );
    }
}

#[test]
fn user_post_in_feed_has_background_color() {
    let _lock = crate::theme::test_lock();
    let mut state = AppState::default();
    state.session.messages.push(ChatMessage {
        role: Role::User,
        content: "hello".to_string(),
        timestamp: 0.0,
        id: "req.0".to_string(),
        ..Default::default()
    });
    state.messages_changed();
    state.ensure_fresh();

    let backend = TestBackend::new(60, 12);
    let mut terminal = Terminal::new(backend).unwrap();
    terminal.draw(|f| view(f, &mut state)).unwrap();
    let buf = terminal.backend().buffer();

    let accent_bg = crate::theme::color_accent_bg();
    assert_ne!(
        accent_bg,
        ratatui::style::Color::Reset,
        "user post background must be a non-default color"
    );

    // Find a cell that is part of the user message content and verify
    // it carries the same accent background used for selected posts.
    let mut found = false;
    for y in 0..buf.area().height {
        for x in 0..buf.area().width {
            if buf[(x, y)].symbol() == "h" && buf[(x, y)].style().bg == Some(accent_bg) {
                found = true;
                break;
            }
        }
        if found {
            break;
        }
    }
    assert!(
        found,
        "user message content should render on the selected-post accent background"
    );
}

#[test]
fn input_box_chevron_has_no_accent_background() {
    let _lock = crate::theme::test_lock();
    let state = AppState::default();

    let backend = TestBackend::new(60, 12);
    let mut terminal = Terminal::new(backend).unwrap();
    terminal.draw(|f| view(f, &mut state.clone())).unwrap();
    let buf = terminal.backend().buffer();

    let accent_bg = crate::theme::color_accent_bg();
    let mut found = false;
    for y in 0..buf.area().height {
        for x in 0..buf.area().width {
            if buf[(x, y)].symbol() == "❯" {
                assert_ne!(
                    buf[(x, y)].style().bg,
                    Some(accent_bg),
                    "input box chevron must not carry the selected-post accent background"
                );
                found = true;
            }
        }
    }
    assert!(found, "input chevron not found");
}
