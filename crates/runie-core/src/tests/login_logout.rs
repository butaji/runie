//! Login / logout command tests — config.toml is the single source of truth.

use crate::event::Event;
use crate::model::AppState;

static HOME_LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());

struct HomeGuard {
    original: Option<String>,
}

impl HomeGuard {
    fn new(home: &std::path::Path) -> Self {
        let original = std::env::var("HOME").ok();
        std::env::set_var("HOME", home);
        Self { original }
    }
}

impl Drop for HomeGuard {
    fn drop(&mut self) {
        match &self.original {
            Some(h) => std::env::set_var("HOME", h),
            None => std::env::remove_var("HOME"),
        }
    }
}

#[test]
fn login_command_saves_provider_to_config_toml() {
    let _guard = HOME_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    let tmp = tempfile::tempdir().unwrap();
    let _home = HomeGuard::new(tmp.path());

    let mut state = AppState::default();
    state.update(Event::RunLoginCommand {
        provider: "minimax".into(),
        token: "sk-test".into(),
    });

    let configured = crate::login_config::list_configured_providers();
    assert!(
        configured.iter().any(|(n, _, _)| n == "minimax"),
        "login must write provider to config.toml, got {:?}",
        configured
    );
    assert!(
        state.configured_providers.contains(&"minimax".to_string()),
        "login must refresh the configured_providers cache"
    );
    let last = state.session.messages.last().expect("system msg");
    assert!(
        last.content.contains("Logged in to 'minimax'"),
        "unexpected message: {}",
        last.content
    );
}

#[test]
fn logout_command_removes_provider_from_config_toml() {
    let _guard = HOME_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    let tmp = tempfile::tempdir().unwrap();
    let _home = HomeGuard::new(tmp.path());

    // Pre-populate config.toml with a provider entry.
    crate::login_config::save_provider_config(
        "minimax",
        "https://api.minimaxi.chat/v1",
        "sk-test",
        &["MiniMax-M3".into()],
    )
    .unwrap();

    let mut state = AppState::default();
    // Default initialization must pick up the pre-seeded provider.
    assert!(state.configured_providers.contains(&"minimax".to_string()));

    state.update(Event::RunLogoutCommand {
        provider: "minimax".into(),
    });

    let configured = crate::login_config::list_configured_providers();
    assert!(
        !configured.iter().any(|(n, _, _)| n == "minimax"),
        "logout must remove provider from config.toml, got {:?}",
        configured
    );
    assert!(
        !state.configured_providers.contains(&"minimax".to_string()),
        "logout must refresh the configured_providers cache"
    );
    let last = state.session.messages.last().expect("system msg");
    assert!(
        last.content.contains("Logged out from 'minimax'"),
        "unexpected message: {}",
        last.content
    );
}

#[test]
fn login_flow_save_refreshes_configured_providers() {
    let _guard = HOME_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    let tmp = tempfile::tempdir().unwrap();
    let _home = HomeGuard::new(tmp.path());

    let mut state = AppState::default();
    state.update(Event::LoginFlowStart);
    state.update(Event::LoginFlowSelectProvider {
        provider: "minimax".into(),
    });
    state.update(Event::LoginFlowSubmitKey {
        provider: "minimax".into(),
        key: "sk-test".into(),
    });
    state.update(Event::LoginFlowSave);

    assert!(state.open_dialog.is_none());
    let configured = crate::login_config::list_configured_providers();
    assert!(
        configured.iter().any(|(n, _, _)| n == "minimax"),
        "login flow save must write provider to config.toml"
    );
    assert!(
        state.configured_providers.contains(&"minimax".to_string()),
        "login flow save must refresh the configured_providers cache"
    );
}

#[test]
fn slash_login_with_args_saves_provider_to_config_toml() {
    let _guard = HOME_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    let tmp = tempfile::tempdir().unwrap();
    let _home = HomeGuard::new(tmp.path());

    let mut state = AppState::default();
    state.input.input = "/login minimax sk-test".into();
    state.input.cursor_pos = state.input.input.len();
    state.update(Event::Submit);

    let configured = crate::login_config::list_configured_providers();
    assert!(
        configured.iter().any(|(n, _, _)| n == "minimax"),
        "/login provider token must save to config.toml"
    );
    let last = state.session.messages.last().expect("system msg");
    assert!(
        last.content.contains("Logged in to 'minimax'"),
        "unexpected message: {}",
        last.content
    );
}

#[test]
fn slash_login_with_bad_args_shows_usage() {
    let mut state = AppState::default();
    state.input.input = "/login minimax".into();
    state.input.cursor_pos = state.input.input.len();
    state.update(Event::Submit);

    let last = state.session.messages.last().expect("system msg");
    assert!(
        last.content.contains("Usage: /login provider token"),
        "unexpected message: {}",
        last.content
    );
}

#[test]
fn slash_login_unknown_provider_shows_error() {
    let mut state = AppState::default();
    state.input.input = "/login ghost sk-test".into();
    state.input.cursor_pos = state.input.input.len();
    state.update(Event::Submit);

    let last = state.session.messages.last().expect("system msg");
    assert!(
        last.content.contains("Unknown provider"),
        "unexpected message: {}",
        last.content
    );
}

#[test]
fn slash_logout_with_args_removes_provider_from_config_toml() {
    let _guard = HOME_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    let tmp = tempfile::tempdir().unwrap();
    let _home = HomeGuard::new(tmp.path());

    crate::login_config::save_provider_config(
        "minimax",
        "https://api.minimaxi.chat/v1",
        "sk-test",
        &["MiniMax-M3".into()],
    )
    .unwrap();

    let mut state = AppState::default();
    state.input.input = "/logout minimax".into();
    state.input.cursor_pos = state.input.input.len();
    state.update(Event::Submit);

    let configured = crate::login_config::list_configured_providers();
    assert!(
        !configured.iter().any(|(n, _, _)| n == "minimax"),
        "/logout provider must remove from config.toml"
    );
    let last = state.session.messages.last().expect("system msg");
    assert!(
        last.content.contains("Logged out from 'minimax'"),
        "unexpected message: {}",
        last.content
    );
}

#[test]
fn slash_logout_no_args_opens_provider_picker() {
    let _guard = HOME_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    let tmp = tempfile::tempdir().unwrap();
    let _home = HomeGuard::new(tmp.path());

    crate::login_config::save_provider_config(
        "minimax",
        "https://api.minimaxi.chat/v1",
        "sk-test",
        &["MiniMax-M3".into()],
    )
    .unwrap();

    let mut state = AppState::default();
    state.input.input = "/logout".into();
    state.input.cursor_pos = state.input.input.len();
    state.update(Event::Submit);

    assert!(
        state.open_dialog.is_some(),
        "/logout with no args must open the provider picker"
    );
}

#[test]
fn slash_logout_no_args_no_providers_shows_message() {
    let _guard = HOME_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    let tmp = tempfile::tempdir().unwrap();
    let _home = HomeGuard::new(tmp.path());

    let mut state = AppState::default();
    state.input.input = "/logout".into();
    state.input.cursor_pos = state.input.input.len();
    state.update(Event::Submit);

    assert!(state.open_dialog.is_none());
    let last = state.session.messages.last().expect("system msg");
    assert!(
        last.content.contains("No providers configured"),
        "unexpected message: {}",
        last.content
    );
}
