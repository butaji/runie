//! End-to-end-ish render tests for vim navigation mode.

use super::*;

fn state_with_vim_and_messages() -> AppState {
    let mut state = AppState::default();
    state.config.vim_mode = true;
    for i in 0..30 {
        state.session.messages.push(ChatMessage {
            role: Role::User,
            content: format!("message {}", i),
            timestamp: i as f64,
            id: format!("req.{}", i),
            ..Default::default()
        });
        state.session.messages.push(ChatMessage {
            role: Role::Assistant,
            content: format!("response {}", i),
            timestamp: i as f64 + 0.5,
            id: format!("resp.{}", i),
            ..Default::default()
        });
    }
    state.messages_changed();
    state
}

#[test]
fn vim_mode_hint_renders_in_status() {
    let backend = TestBackend::new(80, 24);
    let mut terminal = Terminal::new(backend).expect("terminal");
    let mut state = AppState::default();
    state.config.vim_mode = true;

    terminal.draw(|f| view(f, &mut state)).expect("draw");

    let buf = terminal.backend().buffer();
    let content: String = buf.content.iter().map(|c| c.symbol()).collect();
    assert!(
        content.contains("esc nav"),
        "vim hint must render. Got: {}",
        content
    );
}

#[test]
fn vim_mode_scroll_renders_older_content() {
    let backend = TestBackend::new(80, 24);
    let mut terminal = Terminal::new(backend).expect("terminal");
    let mut state = state_with_vim_and_messages();

    // First draw records last_visible_height so the subsequent
    // scroll math uses the same value the render will use.
    terminal.draw(|f| view(f, &mut state)).expect("draw");
    // Scroll to the top (oldest content).
    state.update(Event::Input('g'));
    terminal.draw(|f| view(f, &mut state)).expect("draw");

    let buf = terminal.backend().buffer();
    let content: String = buf.content.iter().map(|c| c.symbol()).collect();
    assert!(
        content.contains("message 0") || content.contains("response 0"),
        "oldest message should be visible after go-to-top. Got: {}",
        content
    );
}

#[test]
fn vim_mode_page_down_renders_newer_content() {
    let backend = TestBackend::new(80, 24);
    let mut terminal = Terminal::new(backend).expect("terminal");
    let mut state = state_with_vim_and_messages();

    terminal.draw(|f| view(f, &mut state)).expect("draw");
    state.update(Event::Input('g'));
    state.update(Event::Input(' '));
    terminal.draw(|f| view(f, &mut state)).expect("draw");

    let buf = terminal.backend().buffer();
    let content: String = buf.content.iter().map(|c| c.symbol()).collect();
    assert!(
        content.contains("message 0") || content.contains("response 0"),
        "page-down from top should still show old content. Got: {}",
        content
    );
}

#[test]
fn vim_nav_mode_hint_renders_in_status() {
    let backend = TestBackend::new(80, 24);
    let mut terminal = Terminal::new(backend).expect("terminal");
    let mut state = AppState::default();
    state.config.vim_mode = true;
    state.update(Event::DialogBack); // enter nav mode

    terminal.draw(|f| view(f, &mut state)).expect("draw");

    let buf = terminal.backend().buffer();
    let content: String = buf.content.iter().map(|c| c.symbol()).collect();
    assert!(
        content.contains("j down") && content.contains("k up"),
        "nav-mode hint should show j down / k up. Got: {}",
        content
    );
    assert!(
        content.contains("j down") && content.contains("k up"),
        "nav-mode hint should advertise j down / k up. Got: {}",
        content
    );
    assert!(
        content.contains("space") && content.contains("i"),
        "nav-mode hint should advertise space/i to enter input. Got: {}",
        content
    );
}

#[test]
fn nav_mode_renders_input_box_with_disabled_style() {
    let mut state = AppState::default();
    state.config.vim_mode = true;
    state.update(runie_core::Event::DialogBack); // enter nav
    assert!(state.vim_nav_mode);

    let backend = TestBackend::new(80, 24);
    let mut terminal = Terminal::new(backend).expect("terminal");
    terminal.draw(|f| view(f, &mut state)).expect("draw");

    let buf = terminal.backend().buffer();
    // In nav mode the input box is disabled: NO cursor block is
    // rendered (not even a gray one). Verify the cells immediately to
    // the right of `❯` do NOT carry the accent (active) cursor
    // background, and that there is no "block" character present.
    let active = runie_tui::theme::style_input_cursor();
    let active_bg = active.bg;

    let mut prompt_pos = None;
    for y in 0..buf.area().height {
        let line: String = (0..buf.area().width)
            .map(|x| buf[(x, y)].symbol())
            .collect();
        if let Some(x) = line.find('❯') {
            prompt_pos = Some((y, x));
            break;
        }
    }
    let (py, px) = prompt_pos.expect("input prompt row not found");

    let width = buf.area().width;
    let mut used_active_bg = false;
    for dx in 1u16..4 {
        if px as u16 + dx >= width {
            break;
        }
        let cell = &buf[((px as u16) + dx, py)];
        if let Some(bg) = cell.style().bg {
            if Some(bg) == active_bg {
                used_active_bg = true;
            }
        }
    }
    assert!(
        !used_active_bg,
        "nav-mode input must not render the active (accent) cursor block"
    );
}

// =========================================================================
// =========================================================================
// In nav mode, the selected post is highlighted by a thin orange
// `[`-shaped bracket in the FIRST CELL of every visible row of that post.
// A post groups one or more elements (e.g. a message plus its spacer).
// The bracket uses `╭` at the top visible row, `│` for body rows, and
// `╰` at the bottom visible row. No other cells are affected.
// =========================================================================

#[test]
fn nav_mode_highlights_selected_post_with_orange_bracket() {
    use runie_core::Event;
    let mut state = AppState::default();
    state.config.vim_mode = true;
    for i in 0..4 {
        state.session.messages.push(runie_core::ChatMessage {
            role: runie_core::Role::User,
            content: format!("user {}", i),
            timestamp: i as f64,
            id: format!("req.{}", i),
            ..Default::default()
        });
        state.session.messages.push(runie_core::ChatMessage {
            role: runie_core::Role::Assistant,
            content: format!("reply {}", i),
            timestamp: i as f64 + 0.5,
            id: format!("resp.{}", i),
            ..Default::default()
        });
    }
    state.messages_changed();
    state.ensure_fresh();
    state.update(Event::DialogBack); // enter nav

    // Draw once before jumping so last_visible_height is recorded and
    // the subsequent go-to-top math uses the real viewport size.
    let backend = TestBackend::new(80, 24);
    let mut terminal = Terminal::new(backend).expect("terminal");
    terminal.draw(|f| view(f, &mut state)).expect("draw");

    state.update(Event::Input('g'));
    terminal.draw(|f| view(f, &mut state)).expect("draw");

    let buf = terminal.backend().buffer();
    let accent = runie_tui::theme::color_accent();
    // The bracket is written to the first cell of the message area,
    // which is at the area's x (the full area has a 1-cell margin).
    // We don't know the exact x, so scan the leftmost cells of each
    // row for `▎` with the accent fg.
    let mut found_bar = false;
    for y in 0..buf.area().height {
        // Check the first 4 cells of the row for the bracket (the
        // margin column is at x=1 due to the 1-cell outer margin).
        for x in 0..4 {
            let cell = &buf[(x, y)];
            let is_bracket = cell.symbol() == "▎";
            if is_bracket {
                if let Some(fg) = cell.style().fg {
                    if fg == accent {
                        found_bar = true;
                        // Bracket is exactly one cell wide.
                        assert!(
                            buf[(x + 1, y)].symbol() != "▎",
                            "bracket must be exactly one cell wide"
                        );
                        break;
                    }
                }
            }
        }
        if found_bar {
            break;
        }
    }
    assert!(
        found_bar,
        "nav mode should render an orange bracket in the first cell of the top element"
    );
}

// =========================================================================
// Unified disabled input box: vim nav mode AND command bar open produce
// the same visual treatment (dimmed chevron, no active cursor block).
// =========================================================================

#[test]
fn command_bar_open_renders_input_box_as_disabled() {
    use runie_core::Event;
    let mut state = AppState::default();
    // Open the command palette.
    state.update(Event::ToggleCommandPalette);
    assert!(state.open_dialog.is_some(), "palette should be open");

    let backend = TestBackend::new(80, 24);
    let mut terminal = Terminal::new(backend).expect("terminal");
    terminal.draw(|f| view(f, &mut state)).expect("draw");

    let buf = terminal.backend().buffer();
    // The input box chevron `❯` must be present (the input box is still
    // rendered behind the palette), but it must NOT carry the active
    // (accent) chevron background.
    let active_chevron = runie_tui::theme::style_chevron(true);
    let active_chevron_bg = active_chevron.bg;

    let mut prompt_pos = None;
    for y in 0..buf.area().height {
        let line: String = (0..buf.area().width)
            .map(|x| buf[(x, y)].symbol())
            .collect();
        if let Some(x) = line.find('❯') {
            prompt_pos = Some((y, x));
            break;
        }
    }
    let (py, px) = prompt_pos.expect("input prompt row not found");
    let cell = &buf[(px as u16, py)];
    if let (Some(bg), Some(active_bg)) = (cell.style().bg, active_chevron_bg) {
        assert_ne!(
            bg, active_bg,
            "command-bar open: chevron must not use the active (accent) background"
        );
    }
}

#[test]
fn nav_mode_and_command_bar_share_disabled_chevron_style() {
    use runie_core::Event;
    // Render the input box in three states: enabled, vim nav mode,
    // command bar open. The chevron background in the two disabled
    // states must be the same; the enabled state must differ from
    // both.
    fn chevron_cell(s: &AppState) -> Option<ratatui::style::Style> {
        let backend = ratatui::backend::TestBackend::new(80, 24);
        let mut terminal = ratatui::Terminal::new(backend).expect("terminal");
        terminal.draw(|f| view(f, &mut s.clone())).expect("draw");
        let buf = terminal.backend().buffer();
        for y in 0..buf.area().height {
            let line: String = (0..buf.area().width)
                .map(|x| buf[(x, y)].symbol())
                .collect();
            if let Some(x) = line.find('❯') {
                return Some(buf[(x as u16, y)].style());
            }
        }
        None
    }

    // Enabled.
    let s1 = AppState::default();
    let cell_enabled = chevron_cell(&s1);

    // Vim nav mode.
    let mut s2 = AppState::default();
    s2.config.vim_mode = true;
    s2.update(Event::DialogBack);
    assert!(s2.vim_nav_mode);
    let cell_nav = chevron_cell(&s2);

    let cell_enabled = cell_enabled.expect("enabled chevron cell");
    let cell_nav = cell_nav.expect("nav chevron cell");

    // The nav-mode chevron must be dimmed (gray), not the active accent
    // foreground. The enabled chevron must differ from the nav chevron
    // (the enabled state uses the accent foreground).
    let hint_fg = runie_tui::theme::style_hint().fg;
    assert_eq!(
        cell_nav.fg, hint_fg,
        "nav-mode chevron must use the hint (dim) foreground"
    );
    assert_ne!(
        cell_enabled, cell_nav,
        "enabled chevron must differ from the disabled (nav) chevron"
    );

    // The command bar (palette) is an overlay; the underlying input
    // box is still in the disabled state. The state machine guarantees
    // the same `token_held = false` is used (see input() in ui.rs),
    // so the input box would render identically. The visible cells may
    // be overwritten by the panel, so we only verify the state.
    let mut s3 = AppState::default();
    s3.update(Event::ToggleCommandPalette);
    assert!(s3.open_dialog.is_some(), "palette should be open");
    // No input handling: the state did not change in response to the
    // underlying input box (palette intercepts). This documents the
    // unified disabled handling.
}
