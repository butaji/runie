use super::find_input_box_bounds;
use crate::ui::view;
use ratatui::{backend::TestBackend, Terminal};
use runie_core::AppState;

fn clean_config() -> std::path::PathBuf {
    let dir =
        std::env::temp_dir().join(format!("runie_no_model_{:?}", std::thread::current().id()));
    let _ = std::fs::create_dir_all(&dir);
    let path = dir.join("config.toml");
    let _ = std::fs::remove_file(&path);
    runie_core::provider::config::set_test_config_path(path.clone());
    path
}

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

#[test]
fn apply_config_ignores_stale_top_level_provider() {
    let _path = clean_config();
    let config = r#"provider = "openai"
model = "gpt-4o"
"#;
    std::fs::write(&_path, config).unwrap();

    let mut state = AppState::default();
    state.config.current_provider.clear();
    state.config.current_model.clear();

    let config = runie_core::config::Config::load(Some(&_path));
    state.apply_config(&config);

    assert!(
        !state.has_models(),
        "stale top-level provider with no configured credentials must not restore a model"
    );
    assert!(state.config.current_provider.is_empty());
    assert!(state.config.current_model.is_empty());
}

#[test]
fn apply_config_ignores_stale_default_model_for_provider() {
    let _path = clean_config();
    let config = r#"provider = "openai"

[models]
default = "claude-sonnet-4-6"

[model_providers.openai]
base_url = "https://api.openai.com/v1"
api_key = "sk-test"
models = ["gpt-4o"]
"#;
    std::fs::write(&_path, config).unwrap();

    let mut state = AppState::default();
    state.config.current_provider.clear();
    state.config.current_model.clear();

    let config = runie_core::config::Config::load(Some(&_path));
    state.apply_config(&config);

    assert_eq!(state.config.current_provider, "openai");
    assert_eq!(
        state.config.current_model, "gpt-4o",
        "stale [models].default from another provider should be ignored"
    );
}
