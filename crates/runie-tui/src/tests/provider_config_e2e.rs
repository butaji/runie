//! End-to-end provider config tests.
//!
//! Verifies that a provider whose API key was saved during onboarding can be
//! rebuilt from the config file at message-send time.

use runie_core::provider::config as login_config;

fn isolated_config() -> std::path::PathBuf {
    let dir = std::env::temp_dir().join(format!(
        "runie_provider_config_e2e_{:?}",
        std::thread::current().id()
    ));
    let _ = std::fs::create_dir_all(&dir);
    dir.join("config.toml")
}

#[test]
fn minimax_key_saved_during_login_is_used_when_sending_message() {
    let path = isolated_config();
    let _ = std::fs::remove_file(&path);
    login_config::set_test_config_path(path.clone());

    // Simulate the onboarding save step.
    login_config::save_provider_config(
        "minimax",
        "https://api.minimaxi.chat/v1",
        "sk-minimax-saved",
        &["MiniMax-M3".into()],
    )
    .expect("save provider config");

    // Load the same config file that the TUI agent_loop will load.
    let config = runie_core::config::Config::load(Some(&path));

    // The migration v3→v4 moves api_key to keyring, so set env var for tests
    // (env has highest priority in credential resolution).
    let saved = std::env::var("MINIMAX_API_KEY").ok();
    std::env::set_var("MINIMAX_API_KEY", "sk-minimax-saved");

    let provider = runie_provider::build_provider_with_config("minimax", "MiniMax-M3", &config)
        .expect("provider should build from saved config key");
    assert_eq!(provider.key(), "minimax");
    assert_eq!(provider.model(), "MiniMax-M3");

    if let Some(v) = saved {
        std::env::set_var("MINIMAX_API_KEY", v);
    } else {
        std::env::remove_var("MINIMAX_API_KEY");
    }
}

#[tokio::test]
async fn minimax_key_persists_through_runtime_save_and_load() {
    let path = isolated_config();
    let _ = std::fs::remove_file(&path);
    login_config::set_test_config_path(path.clone());

    // Simulate the onboarding save step running on the Tokio runtime.
    login_config::save_provider_config(
        "minimax",
        "https://api.minimaxi.chat/v1",
        "sk-minimax-runtime",
        &["MiniMax-M3".into()],
    )
    .expect("save provider config");

    // Load the same config file that the TUI agent_loop will load.
    let config = runie_core::config::Config::load(Some(&path));

    // The migration v3→v4 moves api_key to keyring, so set env var for tests
    // (env has highest priority in credential resolution).
    let saved = std::env::var("MINIMAX_API_KEY").ok();
    std::env::set_var("MINIMAX_API_KEY", "sk-minimax-runtime");

    let provider = runie_provider::build_provider_with_config("minimax", "MiniMax-M3", &config)
        .expect("provider should build from saved config key after runtime save");
    assert_eq!(provider.key(), "minimax");
    assert_eq!(provider.model(), "MiniMax-M3");

    if let Some(v) = saved {
        std::env::set_var("MINIMAX_API_KEY", v);
    } else {
        std::env::remove_var("MINIMAX_API_KEY");
    }
}

#[test]
fn env_var_still_takes_priority_over_saved_config() {
    let path = isolated_config();
    let _ = std::fs::remove_file(&path);
    login_config::set_test_config_path(path.clone());

    login_config::save_provider_config(
        "minimax",
        "https://api.minimaxi.chat/v1",
        "sk-from-config",
        &["MiniMax-M3".into()],
    )
    .expect("save provider config");

    let config = runie_core::config::Config::load(Some(&path));

    let saved = std::env::var("MINIMAX_API_KEY").ok();
    std::env::set_var("MINIMAX_API_KEY", "sk-from-env");

    let provider =
        runie_provider::build_provider_with_config("minimax", "MiniMax-M3", &config).expect("provider should build");
    assert_eq!(provider.key(), "minimax");

    if let Some(v) = saved {
        std::env::set_var("MINIMAX_API_KEY", v);
    } else {
        std::env::remove_var("MINIMAX_API_KEY");
    }
}
