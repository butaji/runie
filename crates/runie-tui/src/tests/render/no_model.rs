use super::find_input_box_bounds;
use crate::ui::view;
use ratatui::{backend::TestBackend, Terminal};
use runie_core::AppState;

fn buffer_content(state: &mut AppState, width: u16, height: u16) -> String {
    let backend = TestBackend::new(width, height);
    let mut terminal = Terminal::new(backend).unwrap();
    terminal.draw(|f| view(f, state)).unwrap();
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
fn input_box_hidden_when_no_model_connected() {
    let _lock = crate::theme::test_lock();
    let mut state = AppState::default();
    state.config.current_provider.clear();
    state.config.current_model.clear();

    let backend = TestBackend::new(60, 20);
    let mut terminal = Terminal::new(backend).unwrap();
    terminal.draw(|f| view(f, &mut state)).unwrap();
    let buf = terminal.backend().buffer();

    assert_eq!(
        find_input_box_bounds(buf),
        (0, 0),
        "input box should not render when no model is connected"
    );
}

#[test]
fn status_bar_hidden_when_no_model_connected() {
    let _lock = crate::theme::test_lock();
    let mut state = AppState::default();
    state.config.current_provider.clear();
    state.config.current_model.clear();
    state.cwd_name = "testdir".to_string();

    let content = buffer_content(&mut state, 60, 20);
    assert!(
        !content.contains("testdir/"),
        "status bar should not render when no model is connected: {}",
        content
    );
}

#[test]
fn input_box_and_status_bar_visible_after_model_connected() {
    let _lock = crate::theme::test_lock();
    let mut state = AppState::default();
    state.config.current_provider = "openai".to_string();
    state.config.current_model = "gpt-4o".to_string();
    state.cwd_name = "testdir".to_string();

    let backend = TestBackend::new(60, 20);
    let mut terminal = Terminal::new(backend).unwrap();
    terminal.draw(|f| view(f, &mut state)).unwrap();
    let buf = terminal.backend().buffer();

    let (top, bottom) = find_input_box_bounds(buf);
    assert!(
        bottom > top,
        "input box should render once a model is connected"
    );

    let content = buffer_content(&mut state, 60, 20);
    assert!(
        content.contains("openai/gpt-4o"),
        "status bar should show provider/model: {}",
        content
    );
}
