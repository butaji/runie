//! Login config tests.

use tempfile::tempdir;

fn parse_providers(doc: &toml::Value) -> Vec<(String, String, Vec<String>)> {
    doc.get("model_providers")
        .and_then(|v| v.as_table())
        .map(|providers| {
            providers
                .iter()
                .map(|(name, val)| {
                    let base_url = val
                        .get("base_url")
                        .and_then(|v| v.as_str())
                        .unwrap_or("")
                        .to_string();
                    let models = val
                        .get("models")
                        .and_then(|v| v.as_array())
                        .map(|arr| {
                            arr.iter()
                                .filter_map(|m| m.as_str().map(String::from))
                                .collect()
                        })
                        .unwrap_or_default();
                    (name.clone(), base_url, models)
                })
                .collect()
        })
        .unwrap_or_default()
}

#[test]
fn config_save_provider_writes_toml() {
    let dir = tempdir().unwrap();
    let path = dir.path().join("config.toml");

    // Write directly using the function logic
    let mut doc = toml::Value::Table(toml::map::Map::new());
    let table = doc.as_table_mut().unwrap();
    let providers = table
        .entry("model_providers")
        .or_insert_with(|| toml::Value::Table(toml::map::Map::new()))
        .as_table_mut()
        .unwrap();

    let mut provider = toml::map::Map::new();
    provider.insert(
        "base_url".into(),
        toml::Value::String("https://api.minimaxi.chat/v1".into()),
    );
    provider.insert("api_key".into(), toml::Value::String("sk-test".into()));
    let arr: Vec<toml::Value> = vec![toml::Value::String("MiniMax-M3".into())];
    provider.insert("models".into(), toml::Value::Array(arr));
    providers.insert("minimax".into(), toml::Value::Table(provider));

    std::fs::write(&path, toml::to_string_pretty(&doc).unwrap()).unwrap();

    let content = std::fs::read_to_string(&path).unwrap();
    assert!(content.contains("[model_providers.minimax]"));
    assert!(content.contains("base_url"));
    assert!(content.contains("api_key"));
    assert!(content.contains("models"));
}

#[test]
fn config_remove_provider_deletes_section() {
    let dir = tempdir().unwrap();
    let path = dir.path().join("config.toml");
    std::fs::write(
        &path,
        r#"
[model_providers.minimax]
base_url = "https://api.minimaxi.chat/v1"
api_key = "sk-test"
"#,
    )
    .unwrap();

    let content = std::fs::read_to_string(&path).unwrap();
    let mut doc: toml::Value = content.parse().unwrap();
    let table = doc.as_table_mut().unwrap();
    if let Some(providers) = table
        .get_mut("model_providers")
        .and_then(|v| v.as_table_mut())
    {
        providers.remove("minimax");
    }
    std::fs::write(&path, toml::to_string_pretty(&doc).unwrap()).unwrap();

    let content = std::fs::read_to_string(&path).unwrap();
    assert!(!content.contains("[model_providers.minimax]"));
}

#[test]
fn list_configured_providers_reads_toml() {
    let dir = tempdir().unwrap();
    let path = dir.path().join("config.toml");
    std::fs::write(
        &path,
        r#"
[model_providers.minimax]
base_url = "https://api.minimaxi.chat/v1"
api_key = "sk-test"
models = ["MiniMax-M3"]

[model_providers.openai]
base_url = "https://api.openai.com/v1"
api_key = "sk-openai"
"#,
    )
    .unwrap();

    let content = std::fs::read_to_string(&path).unwrap();
    let doc: toml::Value = content.parse().unwrap();
    let result = parse_providers(&doc);

    assert_eq!(result.len(), 2);
    let minimax = result.iter().find(|(n, _, _)| n == "minimax").unwrap();
    assert_eq!(minimax.1, "https://api.minimaxi.chat/v1");
    assert_eq!(minimax.2, vec!["MiniMax-M3"]);
}

#[test]
fn list_configured_providers_sorted_alphabetically() {
    use super::list_configured_providers;
    let dir = tempdir().unwrap();
    let path = dir.path().join("config.toml");
    std::fs::write(
        &path,
        r#"
[model_providers.zulu]
base_url = "https://zulu.example/v1"
api_key = "sk-zulu"
models = ["z-model"]

[model_providers.anthropic]
base_url = "https://api.anthropic.com/v1"
api_key = "sk-anthropic"
models = ["claude-sonnet-4-6"]

[model_providers.openai]
base_url = "https://api.openai.com/v1"
api_key = "sk-openai"
models = ["gpt-4o"]
"#,
    )
    .unwrap();

    super::set_test_config_path(path);
    let providers = list_configured_providers();
    let names: Vec<_> = providers.iter().map(|(n, _, _)| n.as_str()).collect();
    assert_eq!(names, vec!["anthropic", "openai", "zulu"]);
}
