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
        content.contains("j/k scroll"),
        "nav-mode hint should show j/k scroll. Got: {}",
        content
    );
    assert!(
        content.contains("space input"),
        "nav-mode hint should show space input. Got: {}",
        content
    );
}
