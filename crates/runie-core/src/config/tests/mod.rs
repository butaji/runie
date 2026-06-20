use super::*;
use std::fs;
use std::sync::Mutex;

mod layered_tests;

static HOME_LOCK: Mutex<()> = Mutex::new(());

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
    let mut prev = Config::default();
    prev.model_providers.insert(
        "openai".to_string(),
        ModelProvider {
            provider_type: Some("openai".to_string()),
            base_url: "https://api.openai.com".to_string(),
            api_key: "sk-old".to_string(),
            models: Vec::new(),
        },
    );
    let mut curr = prev.clone();
    curr.model_providers
        .get_mut("openai")
        .unwrap()
        .api_key = "sk-new".to_string();
    let changes = curr.classify_change(&prev);
    assert!(changes.contains(&ConfigChange::Credentials));
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
fn provider_chain_includes_fallbacks() {
    let mut config = Config::default();
    config.provider = Some("openai".to_string());
    config.fallback_providers = vec!["anthropic".to_string()];
    let chain = config.provider_chain();
    assert_eq!(chain, vec!["openai", "anthropic"]);
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
fn save_nonblocking_writes_file() {
    let _guard = HOME_LOCK.lock().unwrap();
    let dir = tempfile::tempdir().unwrap();
    let original = std::env::var("HOME").ok();
    std::env::set_var("HOME", dir.path());

    let mut config = Config::default();
    config.provider = Some("anthropic".to_string());
    config.save_nonblocking();

    let path = config_path();
    assert!(path.exists(), "config file should be written");
    let loaded = Config::load(Some(&path));
    assert_eq!(loaded.provider.as_deref(), Some("anthropic"));

    if let Some(home) = original {
        std::env::set_var("HOME", home);
    } else {
        std::env::remove_var("HOME");
    }
}
