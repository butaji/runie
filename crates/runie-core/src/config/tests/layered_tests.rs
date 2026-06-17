use super::*;
use super::layers::load_layers_from_paths;
use std::fs;

#[test]
fn layered_config_local_overrides_global() {
    let global = tempfile::tempdir().unwrap();
    let local = tempfile::tempdir().unwrap();
    let global_path = global.path().join("config.toml");
    let local_path = local.path().join("config.toml");
    fs::write(&global_path, "provider = \"openai\"\nmodel = \"gpt-4o\"\n").unwrap();
    fs::write(&local_path, "provider = \"anthropic\"\n").unwrap();
    let config = load_layers_from_paths(global_path, local_path);
    assert_eq!(config.provider, Some("anthropic".to_string()));
    assert_eq!(config.default_model(), Some("gpt-4o"));
}

#[test]
fn layered_config_env_overrides_file() {
    let global = tempfile::tempdir().unwrap();
    let local = tempfile::tempdir().unwrap();
    let global_path = global.path().join("config.toml");
    fs::write(&global_path, "provider = \"openai\"\nmodel = \"gpt-4o\"\n").unwrap();
    std::env::set_var("RUNIE_PROVIDER", "anthropic");
    let config = Config::load_layers_from_paths(global_path, local.path().join("config.toml"));
    assert_eq!(config.provider, Some("anthropic".to_string()));
}

#[test]
fn layered_config_merges_nested_sections() {
    let global = tempfile::tempdir().unwrap();
    let local = tempfile::tempdir().unwrap();
    let global_path = global.path().join("config.toml");
    let local_path = local.path().join("config.toml");
    fs::write(
        &global_path,
        "[ui]\nvim_mode = true\n[telemetry]\nenabled = false\n",
    )
    .unwrap();
    fs::write(&local_path, "[ui]\nvim_mode = false\n").unwrap();
    let config = load_layers_from_paths(global_path, local_path);
    assert!(!config.vim_mode());
    assert!(!config.telemetry_enabled());
}
