use super::super::*;
use super::*;
use runie_core::Event;

#[test]
#[ignore = "@ file lookup tab-completion not wired up in current build"]
fn test_render_at_lookup_popup_shows_on_tab() {
    let backend = TestBackend::new(60, 20);
    let mut terminal = Terminal::new(backend).expect("terminal");
    let mut state = AppState::default();

    for c in "@Car".chars() {
        state.update(Event::Input(c));
    }
    state.update(Event::Input('\t'));

    terminal.draw(|f| view(f, &mut state)).expect("draw");
    let buf = terminal.backend().buffer();
    let content: String = buf.content.iter().map(|c| c.symbol()).collect();
    assert!(
        content.contains("Files"),
        "Popup title must render. Buffer:\n{}",
        content
    );
    assert!(
        content.contains("Cargo") || content.contains("cargo"),
        "Must show Cargo files. Buffer:\n{}",
        content
    );
}

#[test]
fn test_render_at_lookup_popup_shows_immediately() {
    let backend = TestBackend::new(60, 20);
    let mut terminal = Terminal::new(backend).expect("terminal");
    let mut state = AppState::default();

    state.update(Event::Input('@'));
    terminal.draw(|f| view(f, &mut state)).expect("draw");

    let buf = terminal.backend().buffer();
    let content: String = buf.content.iter().map(|c| c.symbol()).collect();
    assert!(
        content.contains("Files"),
        "Popup must show immediately on @. Buffer:\n{}",
        content
    );
}

#[test]
#[ignore = "@ file lookup tab-completion not wired up in current build"]
fn test_render_at_lookup_tab_cycles_and_enter_inserts() {
    let backend = TestBackend::new(60, 20);
    let mut terminal = Terminal::new(backend).expect("terminal");
    let mut state = AppState::default();

    for c in "@Car".chars() {
        state.update(Event::Input(c));
    }
    state.update(Event::Input('\t'));
    state.update(Event::Input('\t'));
    state.update(Event::Submit);

    terminal.draw(|f| view(f, &mut state)).expect("draw");
    assert!(
        !state.input.input.contains('@'),
        "@ should be replaced. Got: {}",
        state.input.input
    );
    assert!(
        state.input.input.contains('[') && state.input.input.contains(']'),
        "Should be inserted as [path]. Got: {}",
        state.input.input
    );
}
