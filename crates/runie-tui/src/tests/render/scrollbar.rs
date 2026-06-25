use super::*;
use crate::ui::view;
use ratatui::{backend::TestBackend, Terminal};
use runie_core::Event;

/// Scrollbar should render in the rightmost column when content overflows.
#[test]
fn scrollbar_renders_in_rightmost_column() {
    let _lock = crate::theme::test_lock();
    let mut state = AppState::default();
    for i in 0..30 {
        state.session.messages.push(ChatMessage {
            role: Role::User,
            parts: vec![Part::Text { content: format!("msg{}", i) }],
            timestamp: i as f64,
            id: format!("u{}", i),
            ..Default::default()
        });
    }
    state.messages_changed();

    let backend = TestBackend::new(40, 10);
    let mut terminal = Terminal::new(backend).unwrap();
    terminal.draw(|f| view(f, &mut state)).unwrap();
    let buf = terminal.backend().buffer();

    let thumb = crate::theme::SCROLLBAR_THUMB;
    let mut found_at_right_edge = false;
    for y in 0..buf.area().height {
        let rightmost = buf[(39, y)].symbol();
        if rightmost == thumb {
            found_at_right_edge = true;
        }
    }
    assert!(
        found_at_right_edge,
        "Scrollbar thumb '{}' should be in rightmost column (x=39)",
        thumb
    );
}

/// Content should use full width — no premature wrapping due to scrollbar.
#[test]
fn content_uses_full_width_when_scrollbar_present() {
    let _lock = crate::theme::test_lock();
    let mut state = AppState::default();
    // Add enough messages to overflow and trigger scrollbar
    for i in 0..20 {
        state.session.messages.push(ChatMessage {
            role: Role::User,
            parts: vec![Part::Text { content: "ABCDEFGHIJ".into() }],
            timestamp: i as f64,
            id: format!("u{}", i),
            ..Default::default()
        });
    }
    state.messages_changed();

    let backend = TestBackend::new(20, 10);
    let mut terminal = Terminal::new(backend).unwrap();
    terminal.draw(|f| view(f, &mut state)).unwrap();
    let buf = terminal.backend().buffer();

    // With a 20-column width, content should span almost the entire width.
    // The scrollbar sits in the margin column (x=19), not reducing content area.
    // Verify: the last content column (x=18) has non-space characters.
    let mut found_content_near_edge = false;
    for y in 0..buf.area().height {
        let sym = buf[(18, y)].symbol();
        if sym != " " && sym != crate::theme::SCROLLBAR_THUMB {
            found_content_near_edge = true;
        }
    }
    assert!(
        found_content_near_edge,
        "Content should reach near right edge (col 18) — scrollbar is in margin, not content area"
    );
}

/// No scrollbar rendered when content fits in viewport.
#[test]
fn no_scrollbar_when_content_fits() {
    let _lock = crate::theme::test_lock();
    let mut state = AppState::default();
    state.session.messages.push(ChatMessage {
        role: Role::User,
        parts: vec![Part::Text { content: "hello".into() }],
        timestamp: 0.0,
        id: "u0".into(),
        ..Default::default()
    });
    state.messages_changed();

    let backend = TestBackend::new(40, 20);
    let mut terminal = Terminal::new(backend).unwrap();
    terminal.draw(|f| view(f, &mut state)).unwrap();
    let buf = terminal.backend().buffer();

    let thumb = crate::theme::SCROLLBAR_THUMB;
    for y in 0..buf.area().height {
        let sym = buf[(39, y)].symbol();
        assert_ne!(
            sym, thumb,
            "No scrollbar thumb at y={} when content fits",
            y
        );
    }
}

/// Scrollbar thumb uses dimmed color style.
#[test]
fn scrollbar_thumb_uses_dimmed_style() {
    let _lock = crate::theme::test_lock();
    let mut state = AppState::default();
    for i in 0..30 {
        state.session.messages.push(ChatMessage {
            role: Role::User,
            parts: vec![Part::Text { content: format!("msg{}", i) }],
            timestamp: i as f64,
            id: format!("u{}", i),
            ..Default::default()
        });
    }
    state.messages_changed();

    let backend = TestBackend::new(40, 10);
    let mut terminal = Terminal::new(backend).unwrap();
    terminal.draw(|f| view(f, &mut state)).unwrap();
    let buf = terminal.backend().buffer();

    let thumb = crate::theme::SCROLLBAR_THUMB;
    let dim_fg = crate::theme::color_dim();
    let mut found_thumb = false;
    for y in 0..buf.area().height {
        let cell = &buf[(39, y)];
        if cell.symbol() == thumb {
            assert_eq!(
                cell.style().fg,
                Some(dim_fg),
                "Scrollbar thumb should use dimmed fg color"
            );
            found_thumb = true;
        }
    }
    assert!(found_thumb, "Should find scrollbar thumb with dimmed style");
}

/// Scrollbar track is invisible — at least some cells are plain spaces.
/// (Other UI elements may overwrite the scrollbar column in their rows.)
#[test]
fn scrollbar_track_is_invisible() {
    let _lock = crate::theme::test_lock();
    let mut state = AppState::default();
    for i in 0..30 {
        state.session.messages.push(ChatMessage {
            role: Role::User,
            parts: vec![Part::Text { content: format!("msg{}", i) }],
            timestamp: i as f64,
            id: format!("u{}", i),
            ..Default::default()
        });
    }
    state.messages_changed();

    let backend = TestBackend::new(40, 10);
    let mut terminal = Terminal::new(backend).unwrap();
    terminal.draw(|f| view(f, &mut state)).unwrap();
    let buf = terminal.backend().buffer();

    let thumb = crate::theme::SCROLLBAR_THUMB;
    let track = crate::theme::SCROLLBAR_TRACK;
    let mut found_track = false;
    for y in 0..buf.area().height {
        let cell = &buf[(39, y)];
        if cell.symbol() != thumb && cell.symbol() == track {
            found_track = true;
        }
    }
    assert!(
        found_track,
        "Should find at least one track ('{}') cell in scrollbar column",
        track
    );
}
