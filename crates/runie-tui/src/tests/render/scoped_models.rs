//! Scoped models dialog rendering tests (Layer 3)

use crate::ui::view;
use ratatui::{backend::TestBackend, Terminal};
use runie_core::{model::ScopedModel, AppState, Event};

fn sm(provider: &str, name: &str, enabled: bool) -> ScopedModel {
    ScopedModel {
        provider: provider.into(),
        name: name.into(),
        enabled,
    }
}

fn rect_contains_text(
    buf: &ratatui::buffer::Buffer,
    rect: ratatui::layout::Rect,
    text: &str,
) -> bool {
    for y in rect.y..rect.y + rect.height {
        let line: String = (rect.x..rect.x + rect.width)
            .map(|x| buf[(x, y)].symbol())
            .collect();
        if line.contains(text) {
            return true;
        }
    }
    false
}

fn make_overflowing_models() -> Vec<ScopedModel> {
    let providers = ["openai", "anthropic", "google", "meta", "mistral", "cohere"];
    let mut models: Vec<ScopedModel> = Vec::new();
    for (i, provider) in providers.iter().enumerate() {
        for j in 0..22 {
            models.push(sm(provider, &format!("model-{}-{}", i, j), true));
        }
    }
    models
}

fn popup_outer_rect() -> ratatui::layout::Rect {
    ratatui::layout::Rect {
        x: 10,
        y: 3,
        width: 60,
        height: 18,
    }
}

fn popup_inner_rect() -> ratatui::layout::Rect {
    ratatui::layout::Rect {
        x: 11,
        y: 4,
        width: 58,
        height: 16,
    }
}

fn render_dialog(state: &mut AppState) -> ratatui::buffer::Buffer {
    let backend = TestBackend::new(80, 24);
    let mut terminal = Terminal::new(backend).unwrap();
    terminal.draw(|f| view(f, state)).unwrap();
    terminal.backend().buffer().clone()
}

/// Hotkey hint must remain visible at the bottom of the dialog even when
/// the model list is long enough to exceed the popup height.
///
/// Bug: the hotkey line was just appended to the same `lines` vector as the
/// list, so it was pushed off the visible area by the long list.
#[test]
fn hotkeys_visible_when_list_overflows() {
    let _lock = crate::theme::test_lock();
    let mut state = AppState::default();
    state.config.scoped_models = make_overflowing_models();
    state.update(Event::ToggleScopedModelsDialog);

    let buf = render_dialog(&mut state);
    let popup_rect = popup_outer_rect();

    assert!(
        rect_contains_text(&buf, popup_rect, "navigate"),
        "Hotkey hint should be visible inside the dialog popup area"
    );
    assert!(
        rect_contains_text(&buf, popup_rect, "close"),
        "Hotkey hint should mention 'close' keybinding"
    );
}

/// Hotkey hint should be pinned to the very bottom of the popup (last 2 lines).
#[test]
fn hotkeys_pinned_to_bottom_of_popup() {
    let _lock = crate::theme::test_lock();
    let mut state = AppState::default();
    state.config.scoped_models = make_overflowing_models();
    state.update(Event::ToggleScopedModelsDialog);

    let buf = render_dialog(&mut state);
    let bottom_rect = ratatui::layout::Rect {
        x: popup_inner_rect().x,
        y: popup_inner_rect().y + popup_inner_rect().height - 2,
        width: popup_inner_rect().width,
        height: 2,
    };
    assert!(
        rect_contains_text(&buf, bottom_rect, "navigate"),
        "Hotkey hint should be in the bottom 2 lines of the popup"
    );
}

/// Hotkey hint must be rendered somewhere in the popup using the shared
/// hint parser (so it contains the expected keybinding text).
#[test]
fn hotkeys_use_styled_key_indicator() {
    let _lock = crate::theme::test_lock();
    let mut state = AppState::default();
    state.config.scoped_models = vec![sm("mock", "echo", true)];
    state.update(Event::ToggleScopedModelsDialog);

    let buf = render_dialog(&mut state);
    let has_hint = (0..buf.area().height).any(|y| {
        let line: String = (0..buf.area().width)
            .map(|x| buf[(x, y)].symbol())
            .collect();
        line.contains("navigate") && line.contains("close")
    });
    assert!(
        has_hint,
        "Hotkey hint should be rendered in the dialog buffer"
    );
}
