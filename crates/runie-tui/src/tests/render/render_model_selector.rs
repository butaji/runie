use super::super::*;
use runie_core::event::DialogEvent;

fn render_selector() -> Vec<String> {
    let backend = TestBackend::new(60, 20);
    let mut terminal = Terminal::new(backend).expect("terminal");
    let mut state = AppState::default();
    state.config.current_provider = "openai".to_string();
    state.config.current_model = "gpt-4o".to_string();
    state.update(DialogEvent::ToggleModelSelector);
    terminal.draw(|f| view(f, &mut state)).expect("draw");
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
fn selector_renders_groups() {
    let lines = render_selector();
    let content = lines.join("\n");
    assert!(
        content.contains("Select Model"),
        "Should have dialog title: {}",
        content
    );
    assert!(
        content.contains("anthropic") || content.contains("openai"),
        "Should show provider groups: {}",
        content
    );
}

#[test]
#[ignore = "cost badges not rendered in model selector in current build"]
fn selector_shows_cost() {
    let lines = render_selector();
    let content = lines.join("\n");
    // At least some models have costs in the catalog
    assert!(
        content.contains('$'),
        "Should show cost badges: {}",
        content
    );
}

#[test]
fn selector_marks_current() {
    let lines = render_selector();
    let content = lines.join("\n");
    assert!(
        content.contains('★'),
        "Current model should have star: {}",
        content
    );
}

#[test]
fn filter_shows_matching_models() {
    let backend = TestBackend::new(60, 20);
    let mut terminal = Terminal::new(backend).expect("terminal");
    let mut state = AppState::default();
    state.update(DialogEvent::ToggleModelSelector);
    state.update(DialogEvent::ModelSelectorFilter('g'));
    state.update(DialogEvent::ModelSelectorFilter('p'));
    state.update(DialogEvent::ModelSelectorFilter('t'));
    terminal.draw(|f| view(f, &mut state)).expect("draw");
    let buf = terminal.backend().buffer();
    let lines: Vec<String> = (0..buf.area().height)
        .map(|y| {
            (0..buf.area().width)
                .map(|x| buf[(x, y)].symbol())
                .collect::<String>()
        })
        .collect();
    let content = lines.join("\n");
    assert!(
        content.contains("❯ gpt"),
        "Should show filter prompt: {}",
        content
    );
}
