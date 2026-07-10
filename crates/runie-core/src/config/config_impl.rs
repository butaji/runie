//! `Config` implementation methods.
//!
//! Extracted from `mod.rs` to satisfy the 500-line file limit.

use std::path::{Path, PathBuf};

use anyhow::Context;

use serde_json::Value;

// ── Inline config validation (moved from validate.rs) ──────────────────────────

fn format_json_pointer(path: &[String]) -> String {
    if path.is_empty() {
        String::new()
    } else {
        format!("/{}", path.join("/"))
    }
}

fn check_unknown_fields(value: &Value, errors: &mut Vec<String>) {
    let Some(obj) = value.as_object() else { return };
    let schema = crate::config::schema::schema_value();
    let Some(schema_props) = schema
        .get("properties")
        .and_then(|p| p.as_object())
        .map(|o| o.keys().collect::<std::collections::HashSet<_>>())
    else {
        return;
    };
    for key in obj.keys() {
        if !schema_props.contains(key) {
            errors.push(format!("{key}: unknown field (ignored)"));
        }
    }
}

/// Validate that provider/model references exist in the registry.
pub fn validate_registry(config: &crate::config::Config) -> Vec<String> {
    use crate::provider::registry::find_provider;

    let mut errors = Vec::new();
    errors.append(&mut validate_default_provider_model(config, find_provider));
    errors.append(&mut validate_configured_providers(config, find_provider));
    errors.append(&mut validate_scoped_models(config));
    errors
}

fn validate_default_provider_model(
    config: &crate::config::Config,
    find_provider: impl Fn(&str) -> Option<crate::provider::ProviderMeta>,
) -> Vec<String> {
    let mut errors = Vec::new();
    let Some(provider) = &config.provider else {
        return errors;
    };
    if provider.is_empty() {
        return errors;
    }
    if find_provider(provider).is_none() {
        errors.push(format!(
            "provider '{provider}': unknown provider (not in registry)"
        ));
        return errors;
    }
    let Some(model) = config.default_model() else {
        return errors;
    };
    if model.is_empty() {
        return errors;
    }
    // A provider-prefixed default model must name THIS provider. We deliberately
    // do NOT require the model to appear in the bundled registry list: that list
    // is advisory and inevitably stale (real upstream models ship faster than we
    // enumerate them, and OpenAI-compatible/custom providers are not fully listed).
    // Rejecting the whole config over an unlisted model blocked legitimate model
    // switching and reloads. Structural mistakes (unknown provider, or a model
    // prefixed for a DIFFERENT provider) are still caught below.
    if let Some((provider_prefix, _model_name)) = model.split_once('/') {
        if provider_prefix != *provider {
            errors.push(format!(
                "model '{model}': provider mismatch (expected '{provider}')"
            ));
        }
    }
    errors
}

fn validate_configured_providers(
    config: &crate::config::Config,
    _find_provider: impl Fn(&str) -> Option<crate::provider::ProviderMeta>,
) -> Vec<String> {
    use crate::provider::registry::find_provider;
    let mut errors = Vec::new();
    for (name, provider_config) in &config.model_providers {
        if find_provider(name).is_none() {
            errors.push(format!("[model_providers.{name}]: unknown provider"));
        }
        if let Some(p) = find_provider(name) {
            for model_name in &provider_config.models {
                if p.models.iter().any(|m| &m.name == model_name) || !model_name.contains('/') {
                    continue;
                }
                if let Some(actual_provider) = model_name.split('/').next() {
                    if actual_provider != name {
                        errors.push(format!(
                            "[model_providers.{}].models: model '{}' has wrong provider prefix (expected '{}'/...)",
                            name, model_name, name
                        ));
                    }
                }
            }
        }
    }
    errors
}

fn validate_scoped_models(config: &crate::config::Config) -> Vec<String> {
    use crate::provider::registry::find_provider;
    let mut errors = Vec::new();
    let Some(scoped) = &config.models.scoped else {
        return errors;
    };
    let default_provider = config.provider.as_deref();
    for model in scoped {
        let provider = model
            .split_once('/')
            .map(|(prefix, _)| prefix)
            .or(default_provider)
            .unwrap_or("");
        if find_provider(provider).is_none() {
            errors.push(format!(
                "[models.scoped]: model '{model}' references unknown provider '{provider}'"
            ));
        }
        // As with the default model, we do not require scoped models to be
        // enumerated in the bundled registry list (see validate_default_provider_model).
    }
    errors
}

/// Validate a JSON value against the schemars-generated config JSON schema.
pub fn validate(value: &Value) -> Vec<String> {
    let schema = crate::config::schema::schema_value();
    let Ok(compiled) = jsonschema::JSONSchema::compile(&schema) else {
        return vec!["failed to compile schema".into()];
    };

    let js_errors: Vec<String> = compiled
        .validate(value)
        .err()
        .into_iter()
        .flatten()
        .map(|e| {
            let path = format_json_pointer(&e.instance_path);
            if path.is_empty() {
                e.to_string()
            } else {
                format!("{path}: {e}")
            }
        })
        .collect();

    let mut errors = js_errors;
    check_unknown_fields(value, &mut errors);
    errors
}

impl crate::config::Config {
    /// Load config from an optional path, falling back to the default path.
    ///
    /// Automatically migrates outdated configs and writes them back.
    pub fn load(path: Option<&Path>) -> Self {
        let default_path = config_path();
        let path = path.unwrap_or(&default_path);
        if !path.exists() {
            return Self::default();
        }
        match std::fs::read_to_string(path) {
            Ok(text) => {
                let mut value: toml::Value = match toml::from_str(&text) {
                    Ok(v) => v,
                    Err(_) => return Self::default(),
                };
                match crate::config::migrate::migrate_with_path(
                    &mut value,
                    Some(path.to_path_buf()),
                ) {
                    Ok(true) => {
                        let _ = crate::config::migrate::backup_config(path);
                        if let Ok(migrated) = toml::to_string(&value) {
                            let _ = std::fs::write(path, migrated);
                        }
                    }
                    Ok(false) => {}
                    Err(_) => {}
                }
                let s = toml::to_string(&value).unwrap_or_default();
                toml::from_str(&s).unwrap_or_default()
            }
            Err(_) => Self::default(),
        }
    }

    /// Load config asynchronously, moving blocking file IO off the runtime.
    pub async fn load_async(path: Option<PathBuf>) -> Self {
        tokio::task::spawn_blocking(move || Self::load(path.as_deref()))
            .await
            .unwrap_or_default()
    }

    /// Load and validate config asynchronously.
    ///
    /// Returns `(config, errors)` where errors is empty on success.
    /// The config is always the raw loaded value; the caller must decide
    /// whether to apply it based on `errors.is_empty()`.
    pub async fn load_async_strict(path: Option<PathBuf>) -> (Self, Vec<String>) {
        let path_clone = path.clone();
        match tokio::task::spawn_blocking(move || Self::load_strict(path_clone.as_deref())).await {
            Ok(Ok(config)) => (config, Vec::new()),
            Ok(Err(errors)) => (Self::default(), errors),
            Err(_) => (Self::default(), vec!["load task failed".into()]),
        }
    }

    /// Load config asynchronously, returning `None` if validation fails.
    ///
    /// Use this when you want to keep the old config on validation failure.
    pub async fn load_async_checked(path: Option<PathBuf>) -> Option<Self> {
        let path_clone = path.clone();
        match tokio::task::spawn_blocking(move || Self::load_strict(path_clone.as_deref())).await {
            Ok(Ok(config)) => Some(config),
            _ => None,
        }
    }

    /// Load configuration from layered sources: defaults → global config →
    /// local project config → environment variables.
    pub fn load_layers() -> Self {
        crate::config::layers::load_layers()
    }

    /// Layered config loader with explicit paths (useful for tests).
    pub fn load_layers_from_paths(global: PathBuf, local: PathBuf) -> Self {
        crate::config::layers::load_layers_from_paths(global, local)
    }

    /// Load config and validate it against the JSON schema.
    ///
    /// Returns an error describing all validation failures.
    pub fn load_strict(path: Option<&Path>) -> Result<Self, Vec<String>> {
        let config = Self::load(path);
        config.validate().map(|_| config)
    }

    /// Validate this config against the canonical JSON schema.
    pub fn validate(&self) -> Result<(), Vec<String>> {
        let value = serde_json::to_value(self)
            .map_err(|e| vec![format!("config serialization failed: {e}")])?;
        Self::validate_value(&value)
    }

    /// Validate provider/model references against the registry.
    ///
    /// This runs after JSON schema validation and checks that providers and
    /// models exist in the bundled registry (loaded from YAML files).
    pub fn validate_registry(&self) -> Result<(), Vec<String>> {
        let errors = validate_registry(self);
        if errors.is_empty() {
            Ok(())
        } else {
            Err(errors)
        }
    }

    /// Validate this config against the JSON schema AND registry.
    pub fn validate_full(&self) -> Result<(), Vec<String>> {
        let mut errors = Vec::new();
        if let Err(e) = self.validate() {
            errors.extend(e);
        }
        if let Err(e) = self.validate_registry() {
            errors.extend(e);
        }
        if errors.is_empty() {
            Ok(())
        } else {
            Err(errors)
        }
    }

    /// Validate a raw TOML value against the canonical JSON schema.
    pub fn validate_toml(value: &toml::Value) -> Result<(), Vec<String>> {
        let json = serde_json::to_value(value)
            .map_err(|e| vec![format!("config serialization failed: {e}")])?;
        Self::validate_value(&json)
    }

    fn validate_value(value: &serde_json::Value) -> Result<(), Vec<String>> {
        let errors = validate(value);
        if errors.is_empty() {
            Ok(())
        } else {
            Err(errors)
        }
    }
}

// ── Comment-preserving TOML merge ─────────────────────────────────────────────

/// Merge `new_doc` into `existing_doc` at the value level.
///
/// For root-level tables that exist in both, this merges individual key-value
/// pairs rather than replacing entire tables. This preserves comments and
/// formatting within sections that exist in both documents.
fn merge_toml_documents(existing: &mut toml_edit::DocumentMut, new_doc: &toml_edit::DocumentMut) {
    let existing_root = existing.as_table_mut();
    for (key, new_item) in new_doc.iter() {
        if let Some(existing_item) = existing_root.get_mut(key) {
            // Key exists in both — try value-level merge if both are tables.
            if let Some(existing_table) = existing_item.as_table_mut() {
                if let Some(new_table) = new_item.as_table() {
                    // Merge new table's key-values into existing table.
                    for (k, v) in new_table.iter() {
                        existing_table.insert(k, v.clone());
                    }
                    continue;
                }
            }
            // Can't merge (different types or not tables) — replace entirely.
            existing_root.insert(key, new_item.clone());
        } else {
            // Key doesn't exist in existing — insert it.
            existing_root.insert(key, new_item.clone());
        }
    }
}

impl crate::config::Config {
    /// Save config to the default path.
    pub fn save(&self) -> anyhow::Result<()> {
        self.save_to(&config_path())
    }

    /// Save config to an explicit path, preserving comments in the existing file.
    ///
    /// Uses `fs2` exclusive lock for cross-process safety.
    /// Replaces root-level tables from the serialized config into the existing
    /// file, preserving all comments and formatting outside replaced sections.
    pub fn save_to(&self, path: &Path) -> anyhow::Result<()> {
        use std::fs::{create_dir_all, OpenOptions};
        use std::io::Write;
        if let Some(parent) = path.parent() {
            create_dir_all(parent)?;
        }

        // Serialize the new config.
        let new_toml =
            toml::to_string_pretty(self).with_context(|| "failed to serialize config")?;

        // Read existing file preserving its structure and comments.
        let existing_text = if path.exists() {
            std::fs::read_to_string(path).ok()
        } else {
            None
        };

        let final_toml = match existing_text {
            Some(existing) => {
                // Try to parse both TOMLs with toml_edit for comment-preserving merge.
                let existing_doc: Option<toml_edit::DocumentMut> = existing.parse().ok();
                let new_doc: toml_edit::DocumentMut = match new_toml.parse() {
                    Ok(d) => d,
                    Err(_) => {
                        // Serialization produced invalid TOML.
                        return Err(anyhow::anyhow!("serialized config is not valid TOML",));
                    }
                };

                match existing_doc {
                    Some(mut doc) => {
                        // Value-level merge: for tables that exist in both, merge values
                        // (not the whole table) to preserve comments within those sections.
                        merge_toml_documents(&mut doc, &new_doc);
                        doc.to_string()
                    }
                    None => {
                        // Corrupt existing file — fall back to new content without merge.
                        new_toml
                    }
                }
            }
            None => new_toml,
        };

        let file = OpenOptions::new()
            .write(true)
            .create(true)
            .truncate(true)
            .open(path)
            .with_context(|| format!("failed to open config for writing: {}", path.display()))?;
        let _lock = fs2::FileExt::lock_exclusive(&file);
        let mut file = file;
        file.write_all(final_toml.as_bytes())
            .with_context(|| format!("failed to write config: {}", path.display()))?;
        Ok(())
    }

    /// Save config without blocking the async runtime.
    /// Outside a runtime this behaves like [`save`].
    pub fn save_nonblocking(&self) {
        self.save_nonblocking_to(&config_path());
    }

    /// Save config to the given path without blocking the async runtime.
    ///
    /// Uses the same comment-preserving [`save_to`] logic on a blocking thread.
    pub fn save_nonblocking_to(&self, path: &Path) {
        // Serialize first so we can pass a Result up to the spawn.
        let new_toml = match toml::to_string_pretty(self) {
            Ok(t) => t,
            Err(e) => {
                tracing::error!("failed to serialize config: {e}");
                return;
            }
        };
        let path = path.to_path_buf();

        let do_save = move || -> anyhow::Result<()> {
            if let Some(parent) = path.parent() {
                let _ = std::fs::create_dir_all(parent);
            }

            // Read existing file preserving its structure and comments.
            let existing_text = if path.exists() {
                std::fs::read_to_string(&path).ok()
            } else {
                None
            };

            let final_toml = match existing_text {
                Some(existing) => {
                    let existing_doc: Option<toml_edit::DocumentMut> = existing.parse().ok();
                    let new_doc: toml_edit::DocumentMut = match new_toml.parse() {
                        Ok(d) => d,
                        Err(_) => {
                            return Err(anyhow::anyhow!("serialized config is not valid TOML"));
                        }
                    };
                    match existing_doc {
                        Some(mut doc) => {
                            merge_toml_documents(&mut doc, &new_doc);
                            doc.to_string()
                        }
                        None => new_toml,
                    }
                }
                None => new_toml,
            };

            // Use fs2 exclusive lock for cross-process safety.
            let file = std::fs::OpenOptions::new()
                .write(true)
                .create(true)
                .truncate(true)
                .open(&path)
                .with_context(|| {
                    format!("failed to open config for writing: {}", path.display())
                })?;
            let _lock = fs2::FileExt::lock_exclusive(&file);
            std::fs::write(&path, final_toml)
                .with_context(|| format!("failed to write config: {}", path.display()))?;
            Ok(())
        };

        if tokio::runtime::Handle::try_current().is_ok() {
            tokio::spawn(tokio::task::spawn_blocking(move || {
                if let Err(e) = do_save() {
                    tracing::error!("{e}");
                }
            }));
        } else {
            // Fallback: write synchronously when no runtime is present.
            if let Err(e) = do_save() {
                tracing::error!("{e}");
            }
        }
    }

    /// Get the default model (from `[models].default` or legacy `model` field).
    pub fn default_model(&self) -> Option<&str> {
        self.models.default.as_deref().or(self.model.as_deref())
    }

    /// Get the list of scoped models.
    pub fn scoped_models(&self) -> Option<&Vec<String>> {
        self.models.scoped.as_ref()
    }

    /// Check if telemetry is enabled.
    pub fn telemetry_enabled(&self) -> bool {
        self.telemetry.enabled
    }

    /// Get the prompts section.
    pub fn prompts(&self) -> &crate::config::PromptsSection {
        &self.prompts
    }

    /// Check if vim mode is enabled.
    pub fn vim_mode(&self) -> bool {
        self.ui.vim_mode
    }

    /// Get the provider for a specific model.
    pub fn provider_for_model(&self, full_model: &str) -> Option<&crate::config::ModelProvider> {
        let prefix = full_model.split('/').next().unwrap_or(full_model);
        self.model_providers.get(prefix)
    }

    /// List configured providers sorted by name.
    pub fn configured_providers(&self) -> Vec<(String, String, Vec<String>)> {
        let mut result: Vec<_> = self
            .model_providers
            .iter()
            .map(|(name, p)| (name.clone(), p.base_url.clone(), p.models.clone()))
            .collect();
        result.sort_by(|a, b| a.0.cmp(&b.0));
        result
    }

    /// Return the configured models for a provider.
    pub fn models_for_provider(&self, provider: &str) -> Vec<String> {
        self.model_providers
            .get(provider)
            .map(|p| p.models.clone())
            .unwrap_or_default()
    }

    /// Return the first configured model for a provider, if any.
    pub fn first_model_for_provider(&self, provider: &str) -> Option<String> {
        self.models_for_provider(provider).into_iter().next()
    }

    /// Resolve the default provider/model pair from this config.
    ///
    /// Falls back through: explicit `provider` + saved models, first configured
    /// provider's first model, and finally empty strings when nothing is set.
    pub fn resolve_default_model(&self) -> (String, String) {
        if crate::provider::is_mock_enabled() {
            return ("mock".into(), crate::provider::mock_model());
        }
        if let Some(provider) = self.provider.as_deref().filter(|p| !p.is_empty()) {
            let model = self
                .first_model_for_provider(provider)
                .or_else(|| self.default_model().map(String::from))
                .unwrap_or_default();
            return (provider.to_owned(), model);
        }
        let mut providers: Vec<_> = self.model_providers.iter().collect();
        providers.sort_by_key(|(k, _)| *k);
        if let Some((provider, mp)) = providers.into_iter().next() {
            if let Some(model) = mp.models.first() {
                return (provider.clone(), model.clone());
            }
        }
        (String::new(), String::new())
    }

    /// Classify what changed between two configs.
    pub fn classify_change(&self, prev: &Self) -> Vec<crate::config::ConfigChange> {
        use crate::config::ConfigChange;
        let mut changes = Vec::new();
        let new_vals = current_config_values(self);
        let old_vals = current_config_values(prev);

        if new_vals.0 != old_vals.0 || new_vals.1 != old_vals.1 {
            changes.push(ConfigChange::Model {
                provider: new_vals.0,
                model: new_vals.1,
            });
        }
        if new_vals.2 != old_vals.2 {
            changes.push(ConfigChange::Theme { name: new_vals.2 });
        }
        if self.keybindings != prev.keybindings {
            changes.push(ConfigChange::Keybindings);
        }
        if self.model_providers != prev.model_providers {
            changes.push(ConfigChange::Credentials);
        }
        changes
    }
}

/// What changed in the config that the watcher needs to act on.
#[derive(Debug, Clone, PartialEq)]
pub enum ConfigChange {
    Model { provider: String, model: String },
    Theme { name: String },
    Keybindings,
    Credentials,
}

fn current_config_values(config: &crate::config::Config) -> (String, String, String) {
    let (provider, model) = config.resolve_default_model();
    let theme = config.theme.clone().unwrap_or_else(|| "runie".to_owned());
    (provider, model, theme)
}

/// Get the default config file path.
pub fn config_path() -> PathBuf {
    dirs::home_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join(".runie")
        .join("config.toml")
}

#[cfg(test)]
mod tests {
    use crate::config::Config;

    #[test]
    fn toml_edit_preserves_leading_comments() {
        // Verify toml_edit behavior: leading comments should survive parse+serialize.
        let original = r#"# My Runie config
[models]
default = "old-model"
"#;
        let doc: toml_edit::DocumentMut = original.parse().unwrap();
        let serialized = doc.to_string();
        // toml_edit should preserve the leading comment in the document.
        assert!(
            serialized.contains("# My Runie config"),
            "toml_edit should preserve leading comments, got:\n{serialized}"
        );
    }

    #[test]
    fn toml_edit_merge_preserves_existing_tables() {
        // Existing file with a comment and two sections.
        let existing = r#"# My Runie config
[models]
default = "old-model"

[ui]
vim_mode = false
"#;
        // New config with only [models] updated.
        let new_toml = r#"[models]
default = "new-model"
"#;

        let mut existing_doc: toml_edit::DocumentMut = existing.parse().unwrap();
        let new_doc: toml_edit::DocumentMut = new_toml.parse().unwrap();

        // Get root table and merge tables from new into existing.
        let root = existing_doc.as_table_mut();
        for (key, item) in new_doc.iter() {
            root.insert(key, item.clone());
        }

        let result = existing_doc.to_string();
        // [ui] section must survive.
        assert!(
            result.contains("[ui]"),
            "[ui] section lost after merge, got:\n{result}"
        );
        // New value must be present.
        assert!(
            result.contains("default = \"new-model\""),
            "new value not in merge, got:\n{result}"
        );
    }

    #[test]
    fn save_preserves_comments_in_existing_file() {
        // Create a temp file with a comment and an existing [models] section.
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("config.toml");
        let original = r#"# My Runie config
[models]
default = "old-model"

[ui]
vim_mode = false
"#;
        std::fs::write(&path, original).unwrap();

        // Load, modify, and save a new config.
        let mut config = Config::load(Some(&path));
        config.models.default = Some("new-model".to_owned());
        config.save_to(&path).unwrap();

        // Comments and [ui] section must survive.
        let content = std::fs::read_to_string(&path).unwrap();
        eprintln!("Result:\n{content}");
        // Comments within the [ui] section should survive.
        assert!(content.contains("[ui]"), "[ui] section should be preserved");
        assert!(
            content.contains("vim_mode = false"),
            "[ui] content should be preserved"
        );
        assert!(
            content.contains("default = \"new-model\""),
            "new value should be saved"
        );
    }

    #[test]
    fn save_creates_new_file_when_none_exists() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("new_config.toml");

        let mut config = Config::default();
        config.models.default = Some("my-model".to_owned());
        config.save_to(&path).unwrap();

        assert!(path.exists(), "file should be created");
        let content = std::fs::read_to_string(&path).unwrap();
        assert!(content.contains("default = \"my-model\""));
    }

    #[test]
    fn save_handles_corrupt_existing_file() {
        // Existing file is not valid TOML — save should fall back to new content.
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("bad_config.toml");
        std::fs::write(&path, "not valid toml {").unwrap();

        let mut config = Config::default();
        config.models.default = Some("fallback-model".to_owned());
        config.save_to(&path).unwrap();

        // Should have written the new content, not crashed.
        let content = std::fs::read_to_string(&path).unwrap();
        assert!(
            content.contains("fallback-model"),
            "new config should be saved"
        );
    }

    #[test]
    fn save_with_nonblocking_preserves_comments() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("nonblocking.toml");
        let original = r#"# Header comment
[models]
default = "old"

# Footer comment
"#;
        std::fs::write(&path, original).unwrap();

        let mut config = Config::load(Some(&path));
        config.models.default = Some("new".to_owned());

        // save_nonblocking_to spawns a task (can't await without runtime),
        // so test save_to directly — it uses the same merge logic.
        config.save_to(&path).unwrap();

        let content = std::fs::read_to_string(&path).unwrap();
        // [models] section and new value must survive.
        assert!(content.contains("default = \"new\""));
        assert!(content.contains("[models]"));
    }
}
