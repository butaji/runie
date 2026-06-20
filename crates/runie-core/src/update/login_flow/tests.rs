use super::provider_base_url;
use crate::login_config::{save_provider_config, set_test_config_path};
use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, Ordering};

static COUNTER: AtomicU64 = AtomicU64::new(0);

fn temp_config_path() -> PathBuf {
    let n = COUNTER.fetch_add(1, Ordering::SeqCst);
    let path = PathBuf::from(format!(
        "/tmp/runie_login_flow_test_{}_{}.toml",
        std::process::id(),
        n
    ));
    set_test_config_path(path.clone());
    let _ = std::fs::remove_file(&path);
    path
}

fn app_state_with_config(path: &std::path::Path) -> crate::model::AppState {
    let mut state = crate::model::AppState::default();
    state.config_cache = Some(crate::config::Config::load(Some(path)));
    state
}

#[test]
fn provider_base_url_uses_registry_default_for_new_provider() {
    let path = temp_config_path();

    let default = crate::provider_registry::find_provider("openai")
        .map(|p| p.base_url.to_string())
        .expect("openai should be registered");

    let state = app_state_with_config(&path);
    assert_eq!(provider_base_url(&state, "openai"), default);
}

#[test]
fn provider_base_url_preserves_saved_custom_url() {
    let path = temp_config_path();
    save_provider_config("openai", "http://proxy.local/v1", "key", &["gpt-4o".into()]).unwrap();

    let state = app_state_with_config(&path);
    assert_eq!(provider_base_url(&state, "openai"), "http://proxy.local/v1");
}
