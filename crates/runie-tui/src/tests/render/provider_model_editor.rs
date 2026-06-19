//! Render tests for the provider model editor dialog.

use crate::ui::view;
use ratatui::{backend::TestBackend, Terminal};
use runie_core::event::DialogEvent;
use runie_core::{AppState, ChatMessage, Role};

fn open_provider_model_editor(state: &mut AppState) {
    state.update(DialogEvent::ProviderEditModels {
        provider: "openai".into(),
    });
}

fn setup_provider_config(provider: &str, models: &[String]) {
    let path = std::path::PathBuf::from(format!(
        "/tmp/runie_tui_render_provider_models_{}_{}.toml",
        std::process::id(),
        provider
    ));
    let _ = std::fs::remove_file(&path);
    runie_core::login_config::set_test_config_path(path);
    let _ = runie_core::login_config::save_provider_config(provider, "http://test", "sk-test", models);
}

fn find_row_with_text(buf: &ratatui::buffer::Buffer, text: &str) -> Option<u16> {
    for y in 0..buf.area().height {
        let line: String = (0..buf.area().width).map(|x| buf[(x, y)].symbol()).collect();
        if line.contains(text) {
            return Some(y);
        }
    }
    None
}

#[test]
fn provider_model_editor_renders_provider_and_models() {
    let _lock = crate::theme::test_lock();
    setup_provider_config("openai", &["gpt-4o".into(), "gpt-4o-mini".into()]);
    let mut state = AppState::default();
    state.config.current_provider = "openai".into();
    state.config.current_model = "gpt-4o".into();

    open_provider_model_editor(&mut state);

    let backend = TestBackend::new(80, 24);
    let mut terminal = Terminal::new(backend).unwrap();
    terminal.draw(|f| view(f, &mut state)).unwrap();
    let buf = terminal.backend().buffer();

    assert!(
        find_row_with_text(&buf, "OpenAI Models").is_some(),
        "provider name should appear in dialog title"
    );
    assert!(
        find_row_with_text(&buf, "gpt-4o").is_some(),
        "model names should be rendered"
    );
    assert!(
        find_row_with_text(&buf, "gpt-4o-mini").is_some(),
        "all configured models should be rendered"
    );
}

#[test]
fn provider_model_editor_renders_checked_state() {
    let _lock = crate::theme::test_lock();
    setup_provider_config("openai", &["gpt-4o".into()]);
    let mut state = AppState::default();
    state.config.current_provider = "openai".into();
    state.config.current_model = "gpt-4o".into();

    open_provider_model_editor(&mut state);

    let backend = TestBackend::new(80, 24);
    let mut terminal = Terminal::new(backend).unwrap();
    terminal.draw(|f| view(f, &mut state)).unwrap();
    let buf = terminal.backend().buffer();

    // A checked toggle renders as "[x]".
    let has_check = (0..buf.area().height).any(|y| {
        let line: String = (0..buf.area().width).map(|x| buf[(x, y)].symbol()).collect();
        line.contains("[x]")
    });
    assert!(has_check, "checked model should render [x]");
}
