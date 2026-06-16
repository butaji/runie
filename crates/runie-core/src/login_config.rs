//! Login config persistence — read/write provider credentials in config.toml.

use std::path::PathBuf;

thread_local! {
    static TEST_CONFIG_PATH: std::cell::RefCell<Option<PathBuf>> = const { std::cell::RefCell::new(None) };
}

/// Override the config file path for the current thread (tests only).
#[cfg(test)]
pub fn set_test_config_path(path: PathBuf) {
    TEST_CONFIG_PATH.with(|p| *p.borrow_mut() = Some(path));
}

/// Get the default config file path (from canonical config module).
pub fn config_path() -> PathBuf {
    TEST_CONFIG_PATH.with(|p| {
        if let Some(ref path) = *p.borrow() {
            return path.clone();
        }
        crate::config::config_path()
    })
}

/// Save a provider configuration to `~/.runie/config.toml`.
/// Creates the file and parent directories if needed.
pub fn save_provider_config(
    name: &str,
    base_url: &str,
    api_key: &str,
    models: &[String],
) -> anyhow::Result<()> {
    let path = config_path();
    let content = std::fs::read_to_string(&path).unwrap_or_default();
    let mut doc = parse_config_doc(&content)?;
    let providers = get_or_create_providers_table(&mut doc)?;
    providers.insert(name.into(), build_provider_value(base_url, api_key, models));
    write_config_doc(&path, &doc)
}

fn parse_config_doc(content: &str) -> anyhow::Result<toml::Value> {
    if content.trim().is_empty() {
        Ok(toml::Value::Table(toml::map::Map::new()))
    } else {
        content.parse().map_err(|e| anyhow::anyhow!("{}", e))
    }
}

fn get_or_create_providers_table(
    doc: &mut toml::Value,
) -> anyhow::Result<&mut toml::map::Map<String, toml::Value>> {
    let table = doc
        .as_table_mut()
        .ok_or_else(|| anyhow::anyhow!("Invalid config structure"))?;
    table
        .entry("model_providers")
        .or_insert_with(|| toml::Value::Table(toml::map::Map::new()))
        .as_table_mut()
        .ok_or_else(|| anyhow::anyhow!("Invalid model_providers structure"))
}

fn build_provider_value(base_url: &str, api_key: &str, models: &[String]) -> toml::Value {
    let mut provider = toml::map::Map::new();
    provider.insert("base_url".into(), toml::Value::String(base_url.into()));
    provider.insert("api_key".into(), toml::Value::String(api_key.into()));
    if !models.is_empty() {
        let arr: Vec<toml::Value> = models
            .iter()
            .map(|m| toml::Value::String(m.clone()))
            .collect();
        provider.insert("models".into(), toml::Value::Array(arr));
    }
    toml::Value::Table(provider)
}

fn write_config_doc(path: &std::path::Path, doc: &toml::Value) -> anyhow::Result<()> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    std::fs::write(path, toml::to_string_pretty(doc)?)?;
    Ok(())
}

/// Remove a provider configuration from `~/.runie/config.toml`.
pub fn remove_provider_config(name: &str) -> anyhow::Result<()> {
    let path = config_path();
    let content = match std::fs::read_to_string(&path) {
        Ok(c) => c,
        Err(_) => return Ok(()),
    };
    let mut doc: toml::Value = content.parse()?;

    let table = doc
        .as_table_mut()
        .ok_or_else(|| anyhow::anyhow!("Invalid config structure"))?;
    if let Some(providers) = table
        .get_mut("model_providers")
        .and_then(|v| v.as_table_mut())
    {
        providers.remove(name);
    }

    std::fs::write(&path, toml::to_string_pretty(&doc)?)?;
    Ok(())
}

/// List providers that have configurations in `~/.runie/config.toml`.
pub fn list_configured_providers() -> Vec<(String, String, Vec<String>)> {
    let path = config_path();
    let content = match std::fs::read_to_string(&path) {
        Ok(c) => c,
        Err(_) => return Vec::new(),
    };
    let doc: toml::Value = match content.parse() {
        Ok(v) => v,
        Err(_) => return Vec::new(),
    };

    let mut result = Vec::new();
    if let Some(providers) = doc.get("model_providers").and_then(|v| v.as_table()) {
        for (name, val) in providers {
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
            result.push((name.clone(), base_url, models));
        }
    }
    result
}

// ============================================================================

#[cfg(test)]
mod tests;
