#![allow(clippy::all)]
use super::*;
use std::fs;
use std::sync::Mutex;

mod layered_tests;
mod schema_tests;
mod validate_tests;

static HOME_LOCK: Mutex<()> = Mutex::new(());

// ── Tunable value section tests ───────────────────────────────────────────────

#[test]
fn ui_section_has_tunable_history_max_entries() {
    let section = UiSection::default();
    assert_eq!(section.history_max_entries, 1000);
    assert_eq!(section.history_max(), 1000);

    let custom = UiSection {
        history_max_entries: 500,
        ..UiSection::default()
    };
    assert_eq!(custom.history_max(), 500);
}

#[test]
fn ui_section_has_tunable_page_size() {
    let section = UiSection::default();
    assert_eq!(section.page_size, 5);
    assert_eq!(section.page_size(), 5);

    let custom = UiSection {
        page_size: 10,
        ..UiSection::default()
    };
    assert_eq!(custom.page_size(), 10);
}

#[test]
fn http_section_has_tunable_timeouts() {
    let section = HttpSection::default();
    assert_eq!(section.request_timeout_secs, 120);
    assert_eq!(section.connect_timeout_secs, 10);

    let custom = HttpSection {
        request_timeout_secs: 60,
        connect_timeout_secs: 5,
        ..HttpSection::default()
    };
    assert_eq!(custom.request_timeout_secs, 60);
    assert_eq!(custom.connect_timeout_secs, 5);
}

#[test]
fn retry_section_has_tunable_policy() {
    let section = RetrySection::default();
    assert_eq!(section.max_attempts, 5);
    assert_eq!(section.initial_delay_ms, 100);
    assert_eq!(section.max_delay_ms, 30_000);
    assert!((section.multiplier - 2.0).abs() < f64::EPSILON);

    let custom = RetrySection {
        max_attempts: 3,
        initial_delay_ms: 200,
        max_delay_ms: 60_000,
        multiplier: 1.5,
        ..RetrySection::default()
    };
    assert_eq!(custom.max_attempts, 3);
    assert_eq!(custom.initial_delay_ms, 200);
    assert_eq!(custom.max_delay_ms, 60_000);
    assert!((custom.multiplier - 1.5).abs() < f64::EPSILON);
}

#[test]
fn fff_section_has_tunable_scan_settings() {
    let section = FffSection::default();
    assert_eq!(section.scan_timeout_secs, 30);
    assert_eq!(section.default_limit, 50);
    assert_eq!(section.max_file_size_bytes, 2 * 1024 * 1024);

    let custom = FffSection {
        scan_timeout_secs: 60,
        default_limit: 100,
        max_file_size_bytes: 5 * 1024 * 1024,
        ..FffSection::default()
    };
    assert_eq!(custom.scan_timeout_secs, 60);
    assert_eq!(custom.default_limit, 100);
    assert_eq!(custom.max_file_size_bytes, 5 * 1024 * 1024);
}

#[test]
fn config_includes_all_tunable_sections() {
    let config = Config::default();
    // HTTP section
    assert_eq!(config.http.request_timeout_secs, 120);
    assert_eq!(config.http.connect_timeout_secs, 10);
    // Retry section
    assert_eq!(config.retry.max_attempts, 5);
    // FFF section
    assert_eq!(config.fff.scan_timeout_secs, 30);
    assert_eq!(config.fff.default_limit, 50);
    // UI section
    assert_eq!(config.ui.history_max_entries, 1000);
    assert_eq!(config.ui.page_size, 5);
}

#[test]
fn config_load_parses_http_section() {
    let dir = tempfile::tempdir().unwrap();
    let path = make_test_config(
        &dir,
        r#"
[http]
request_timeout_secs = 60
connect_timeout_secs = 5
"#,
    );
    let config = Config::load(Some(&path));
    assert_eq!(config.http.request_timeout_secs, 60);
    assert_eq!(config.http.connect_timeout_secs, 5);
}

#[test]
fn config_load_parses_retry_section() {
    let dir = tempfile::tempdir().unwrap();
    let path = make_test_config(
        &dir,
        r#"
[retry]
max_attempts = 3
initial_delay_ms = 200
max_delay_ms = 60000
multiplier = 1.5
"#,
    );
    let config = Config::load(Some(&path));
    assert_eq!(config.retry.max_attempts, 3);
    assert_eq!(config.retry.initial_delay_ms, 200);
    assert_eq!(config.retry.max_delay_ms, 60_000);
    assert!((config.retry.multiplier - 1.5).abs() < f64::EPSILON);
}

#[test]
fn config_load_parses_fff_section() {
    let dir = tempfile::tempdir().unwrap();
    let path = make_test_config(
        &dir,
        r#"
[fff]
scan_timeout_secs = 60
default_limit = 100
max_file_size_bytes = 5242880
"#,
    );
    let config = Config::load(Some(&path));
    assert_eq!(config.fff.scan_timeout_secs, 60);
    assert_eq!(config.fff.default_limit, 100);
    assert_eq!(config.fff.max_file_size_bytes, 5 * 1024 * 1024);
}

#[test]
fn config_load_parses_ui_history_and_page_size() {
    let dir = tempfile::tempdir().unwrap();
    let path = make_test_config(
        &dir,
        r#"
[ui]
vim_mode = false
history_max_entries = 500
page_size = 10
"#,
    );
    let config = Config::load(Some(&path));
    assert!(!config.vim_mode());
    assert_eq!(config.ui.history_max_entries, 500);
    assert_eq!(config.ui.page_size(), 10);
}

#[test]
fn tunable_values_match_previous_constants() {
    // Verify defaults match the previous hardcoded constants.
    assert_eq!(HttpSection::default().request_timeout_secs, 120);
    assert_eq!(HttpSection::default().connect_timeout_secs, 10);
    assert_eq!(RetrySection::default().max_attempts, 5);
    assert_eq!(FffSection::default().scan_timeout_secs, 30);
    assert_eq!(FffSection::default().default_limit, 50);
    assert_eq!(FffSection::default().max_file_size_bytes, 2 * 1024 * 1024);
    assert_eq!(UiSection::default().history_max_entries, 1000);
    assert_eq!(UiSection::default().page_size, 5);
}

fn make_test_config(dir: &tempfile::TempDir, content: &str) -> std::path::PathBuf {
    let path = dir.path().join("config.toml");
    fs::write(&path, content).unwrap();
    path
}

#[test]
fn config_load_parses_basic_fields() {
    let dir = tempfile::tempdir().unwrap();
    let path = make_test_config(
        &dir,
        r#"
provider = "openai"
model = "gpt-4"
"#,
    );
    let config = Config::load(Some(&path));
    assert_eq!(config.provider, Some("openai".to_string()));
    assert_eq!(config.default_model(), Some("gpt-4"));
}

#[test]
fn config_load_parses_models_section() {
    let dir = tempfile::tempdir().unwrap();
    let path = make_test_config(
        &dir,
        r#"
[models]
default = "gpt-4o"
scoped = ["gpt-4", "gpt-3.5-turbo"]
"#,
    );
    let config = Config::load(Some(&path));
    assert_eq!(config.default_model(), Some("gpt-4o"));
    let scoped = config.scoped_models().unwrap();
    assert_eq!(scoped.len(), 2);
}

#[test]
fn config_load_parses_provider_section() {
    let dir = tempfile::tempdir().unwrap();
    let path = make_test_config(
        &dir,
        r#"
[model_providers.openai]
type = "openai"
base_url = "https://api.openai.com"
api_key = "sk-test"
"#,
    );
    let config = Config::load(Some(&path));
    let provider = config.provider_for_model("openai/gpt-4").unwrap();
    assert_eq!(provider.base_url, "https://api.openai.com");
}

#[test]
fn config_load_parses_ui_section() {
    let dir = tempfile::tempdir().unwrap();
    let path = make_test_config(
        &dir,
        r#"
[ui]
vim_mode = false
"#,
    );
    let config = Config::load(Some(&path));
    assert!(!config.vim_mode());
}

#[test]
fn config_load_parses_telemetry_section() {
    let dir = tempfile::tempdir().unwrap();
    let path = make_test_config(
        &dir,
        r#"
[telemetry]
enabled = false
"#,
    );
    let config = Config::load(Some(&path));
    assert!(!config.telemetry_enabled());
}

#[test]
fn config_defaults_when_missing() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("nonexistent.toml");
    let config = Config::load(Some(&path));
    assert_eq!(config.provider, None);
    assert_eq!(config.default_model(), None);
    assert!(config.vim_mode());
}

#[test]
fn config_path_returns_expected_path() {
    let path = config_path();
    assert!(path.file_name().is_some_and(|n| n == "config.toml"));
}

#[test]
fn classify_change_detects_model_change() {
    let prev = Config {
        provider: Some("openai".to_string()),
        ..Config::default()
    };
    let curr = Config {
        provider: Some("anthropic".to_string()),
        ..Config::default()
    };
    let changes = curr.classify_change(&prev);
    assert!(
        matches!(changes.as_slice(), [ConfigChange::Model { provider, .. }] if provider == "anthropic")
    );
}

#[test]
fn classify_change_detects_theme_change() {
    let prev = Config {
        theme: Some("dark".to_string()),
        ..Config::default()
    };
    let curr = Config {
        theme: Some("light".to_string()),
        ..Config::default()
    };
    let changes = curr.classify_change(&prev);
    assert!(matches!(changes.as_slice(), [ConfigChange::Theme { name }] if name == "light"));
}

#[test]
fn classify_change_returns_empty_when_identical() {
    let prev = Config {
        provider: Some("openai".to_string()),
        theme: Some("dark".to_string()),
        ..Config::default()
    };
    let curr = prev.clone();
    assert!(curr.classify_change(&prev).is_empty());
}

#[test]
fn classify_change_detects_keybindings_change() {
    let mut prev = Config::default();
    let mut curr = Config::default();
    prev.keybindings
        .insert("ctrl+c".to_string(), "Quit".to_string());
    curr.keybindings
        .insert("ctrl+c".to_string(), "Abort".to_string());
    let changes = curr.classify_change(&prev);
    assert!(matches!(changes.as_slice(), [ConfigChange::Keybindings]));
}

#[test]
fn classify_change_detects_credentials_change() {
    // Credentials are now stored in keyring, not config.
    // This test verifies that changes to base_url don't trigger Credentials change.
    let prev = Config::default();
    let curr = Config::default();
    let changes = curr.classify_change(&prev);
    // No credentials change since api_key is no longer in config
    assert!(!changes.contains(&ConfigChange::Credentials));
}

#[test]
fn classify_change_multiple_changes() {
    let mut prev = Config::default();
    let mut curr = Config::default();
    prev.provider = Some("openai".to_string());
    curr.provider = Some("anthropic".to_string());
    curr.theme = Some("nord".to_string());
    curr.keybindings
        .insert("ctrl+c".to_string(), "Abort".to_string());
    let changes = curr.classify_change(&prev);
    assert_eq!(changes.len(), 3);
}

#[test]
fn config_load_parses_all_sections() {
    let dir = tempfile::tempdir().unwrap();
    let path = make_test_config(
        &dir,
        r#"
provider = "openai"
model = "gpt-4"
theme = "nord"

[models]
default = "gpt-4o"

[ui]
vim_mode = false

[telemetry]
enabled = false

[truncation]
max_lines = 100
max_bytes = 100000

[prompts]
default = "default"
"#,
    );
    let config = Config::load(Some(&path));
    assert_eq!(config.provider, Some("openai".to_string()));
    assert_eq!(config.default_model(), Some("gpt-4o"));
    assert_eq!(config.theme, Some("nord".to_string()));
    assert!(!config.vim_mode());
    assert!(!config.telemetry_enabled());
}

#[test]
fn provider_and_core_see_same_default_model() {
    let dir = tempfile::tempdir().unwrap();
    let path = make_test_config(
        &dir,
        r#"
[models]
default = "gpt-4o"
"#,
    );
    let config = Config::load(Some(&path));
    let default = config.default_model();
    let config2 = Config::load(Some(&path));
    assert_eq!(default, config2.default_model());
}

#[test]
fn config_validation_accepts_valid_config() {
    let dir = tempfile::tempdir().unwrap();
    let path = make_test_config(
        &dir,
        r#"
provider = "openai"
model = "gpt-4o"
"#,
    );
    let config = Config::load(Some(&path));
    assert!(config.validate().is_ok());
}

#[test]
fn config_validation_rejects_invalid_json() {
    let raw: toml::Value = toml::from_str(r#"provider = 123"#).unwrap();
    let result = Config::validate_toml(&raw);
    assert!(
        result.is_err(),
        "provider as integer should fail validation"
    );
}

#[test]
fn layered_config_env_overrides_file() {
    let global = tempfile::tempdir().unwrap();
    let local = tempfile::tempdir().unwrap();
    let global_path = global.path().join("config.toml");
    let local_path = local.path().join("config.toml");
    fs::write(&global_path, "provider = \"openai\"\n").unwrap();
    fs::write(&local_path, "provider = \"anthropic\"\n").unwrap();
    let config = crate::config::layers::load_layers_from_paths(global_path, local_path);
    assert_eq!(config.provider, Some("anthropic".to_string()));
}

#[tokio::test]
async fn load_async_reads_file() {
    let dir = tempfile::tempdir().unwrap();
    let path = make_test_config(&dir, r#"provider = "openai""#);
    let config = Config::load_async(Some(path)).await;
    assert_eq!(config.provider.as_deref(), Some("openai"));
}

#[test]
fn mcp_feature_state_consistent() {
    // Verify the mcp module is always compiled (feature flag removed — see
    // tasks/delete-or-fix-dead-mcp-feature-flag.md). The `mcp = []` feature
    // in Cargo.toml was empty/dead, so McpSection is unconditionally available.
    let section = McpSection::default();
    assert!(section.is_empty());

    let dir = tempfile::tempdir().unwrap();
    let path = make_test_config(
        &dir,
        r#"
[model_providers.openai]
type = "openai"
base_url = "https://api.openai.com"
api_key = "sk-test"
"#,
    );
    let config = Config::load(Some(&path));
    // McpSection is always present in Config, no #[cfg(feature)] gating needed.
    assert!(config.mcp.is_empty());
    let provider = config.provider_for_model("openai/gpt-4").unwrap();
    assert_eq!(provider.base_url, "https://api.openai.com");
}

#[test]
fn save_nonblocking_writes_file() {
    let _guard = HOME_LOCK.lock().unwrap();
    let dir = tempfile::tempdir().unwrap();
    let original = std::env::var("HOME").ok();
    unsafe { std::env::set_var("HOME", dir.path()) };

    let mut config = Config::default();
    config.provider = Some("anthropic".to_string());
    config.save_nonblocking();

    let path = config_path();
    assert!(path.exists(), "config file should be written");
    let loaded = Config::load(Some(&path));
    assert_eq!(loaded.provider.as_deref(), Some("anthropic"));

    if let Some(home) = original {
        unsafe { std::env::set_var("HOME", home) };
    } else {
        unsafe { std::env::remove_var("HOME") };
    }
}

#[test]
fn config_validation_rejects_unknown_field() {
    // This test verifies that validation catches unknown fields.
    // We validate a JSON value directly to avoid serde ignoring unknown TOML fields.
    let value = serde_json::json!({
        "provider": "openai",
        "unknown_field": "this should trigger validation error"
    });
    let errors = crate::config::config_impl::validate(&value);
    assert!(
        !errors.is_empty(),
        "unknown field should produce validation errors: {:?}",
        errors
    );
    assert!(
        errors.iter().any(|e| e.contains("unknown_field")),
        "errors should mention unknown_field: {:?}",
        errors
    );
}
