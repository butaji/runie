//! ConfigAgent — Watches config files for changes and emits ConfigChanged events
//!
//! The ConfigAgent monitors the configuration file and emits domain events
//! when changes are detected, allowing actors to apply new settings
//! without requiring a restart.

use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::{Duration, SystemTime};

use crate::event_bus::{ActorChannel, BusEventEnvelope, ConfigValue, DomainEvent, EventBus};

/// Config file state for change detection
#[derive(Debug, Clone)]
struct ConfigFileState {
    path: PathBuf,
    last_mtime: Option<u64>,
    last_hash: Option<u64>,
}

impl ConfigFileState {
    fn new(path: PathBuf) -> Self {
        Self {
            path,
            last_mtime: None,
            last_hash: None,
        }
    }

    /// Check if file has changed and update state
    fn check_and_update(&mut self) -> Option<HashMap<String, ConfigValue>> {
        let metadata = match fs::metadata(&self.path) {
            Ok(m) => m,
            Err(_) => return None,
        };

        let mtime = metadata.modified().ok()?;
        let mtime_nanos = mtime
            .duration_since(SystemTime::UNIX_EPOCH)
            .ok()?
            .as_nanos() as u64;

        // Check if modified time changed
        let time_changed = self.last_mtime.map(|t| t != mtime_nanos).unwrap_or(true);

        // Calculate content hash
        let content = fs::read(&self.path).ok()?;
        let content_hash = simple_hash(&content);

        // Check if content changed
        let content_changed = self.last_hash.map(|h| h != content_hash).unwrap_or(true);

        if time_changed || content_changed {
            self.last_mtime = Some(mtime_nanos);
            self.last_hash = Some(content_hash);

            // Parse the config
            let content_str = String::from_utf8_lossy(&content);
            let value: toml::Value = toml::from_str(&content_str).ok()?;
            self.parse_config(&value)
        } else {
            None
        }
    }

    fn parse_config(&self, value: &toml::Value) -> Option<HashMap<String, ConfigValue>> {
        if let toml::Value::Table(table) = value {
            let mut changes = HashMap::new();
            for (key, val) in table {
                changes.insert(key.clone(), ConfigValue::from_toml(val));
            }
            Some(changes)
        } else {
            None
        }
    }
}

/// Simple hash function for content change detection
fn simple_hash(data: &[u8]) -> u64 {
    use std::collections::hash_map::DefaultHasher;
    use std::hash::{Hash, Hasher};
    let mut hasher = DefaultHasher::new();
    data.hash(&mut hasher);
    hasher.finish()
}

/// Run the ConfigAgent actor loop.
///
/// The actor:
/// - Monitors config files for changes
/// - Parses TOML configuration
/// - Emits ConfigChanged events on modification
pub fn run_config_agent(
    bus: EventBus,
    _channel: ActorChannel<BusEventEnvelope>,
    shutdown: Arc<AtomicBool>,
    config_paths: Vec<PathBuf>,
    poll_interval: Duration,
) {
    let mut states: Vec<ConfigFileState> = config_paths.into_iter().map(ConfigFileState::new).collect();

    while !shutdown.load(Ordering::SeqCst) {
        for state in &mut states {
            if let Some(changes) = state.check_and_update() {
                bus.publish_domain(DomainEvent::ConfigChanged {
                    path: state.path.clone(),
                    changes,
                });
            }
        }

        std::thread::sleep(poll_interval);
    }
}

/// Parse a TOML configuration file and return as HashMap
pub fn parse_config(path: &PathBuf) -> anyhow::Result<HashMap<String, ConfigValue>> {
    let content = fs::read_to_string(path)?;
    let value: toml::Value = toml::from_str(&content)?;
    let table = value.as_table().ok_or_else(|| {
        anyhow::anyhow!("Config file must contain a table at root")
    })?;

    let mut config = HashMap::new();
    for (key, val) in table {
        config.insert(key.clone(), ConfigValue::from_toml(val));
    }
    Ok(config)
}

/// Get a string config value
pub fn get_string(config: &HashMap<String, ConfigValue>, key: &str) -> Option<String> {
    config.get(key).and_then(|v| match v {
        ConfigValue::String(s) => Some(s.clone()),
        _ => None,
    })
}

/// Get a boolean config value
pub fn get_bool(config: &HashMap<String, ConfigValue>, key: &str) -> Option<bool> {
    config.get(key).and_then(|v| match v {
        ConfigValue::Boolean(b) => Some(*b),
        _ => None,
    })
}

/// Get an integer config value
pub fn get_integer(config: &HashMap<String, ConfigValue>, key: &str) -> Option<i64> {
    config.get(key).and_then(|v| match v {
        ConfigValue::Integer(i) => Some(*i),
        _ => None,
    })
}

/// Get a nested config value (dot-separated path)
pub fn get_nested(config: &HashMap<String, ConfigValue>, path: &str) -> Option<ConfigValue> {
    let parts: Vec<&str> = path.split('.').collect();
    let mut current = config.clone();

    for (i, part) in parts.iter().enumerate() {
        if i == parts.len() - 1 {
            return current.get(*part).cloned();
        }

        match current.get(*part)? {
            ConfigValue::Object(map) => current = map.clone(),
            _ => return None,
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn simple_hash_deterministic() {
        let data = b"hello world";
        let hash1 = simple_hash(data);
        let hash2 = simple_hash(data);
        assert_eq!(hash1, hash2);
    }

    #[test]
    fn simple_hash_different_content() {
        let hash1 = simple_hash(b"hello");
        let hash2 = simple_hash(b"world");
        assert_ne!(hash1, hash2);
    }

    #[test]
    fn get_string_helper() {
        let mut config = HashMap::new();
        config.insert("name".to_string(), ConfigValue::String("test".to_string()));
        config.insert("count".to_string(), ConfigValue::Integer(5));

        assert_eq!(get_string(&config, "name"), Some("test".to_string()));
        assert_eq!(get_string(&config, "count"), None);
        assert_eq!(get_string(&config, "missing"), None);
    }

    #[test]
    fn get_bool_helper() {
        let mut config = HashMap::new();
        config.insert("enabled".to_string(), ConfigValue::Boolean(true));
        config.insert("name".to_string(), ConfigValue::String("test".to_string()));

        assert_eq!(get_bool(&config, "enabled"), Some(true));
        assert_eq!(get_bool(&config, "name"), None);
    }

    #[test]
    fn get_integer_helper() {
        let mut config = HashMap::new();
        config.insert("count".to_string(), ConfigValue::Integer(42));

        assert_eq!(get_integer(&config, "count"), Some(42));
        assert_eq!(get_integer(&config, "missing"), None);
    }

    #[test]
    fn get_nested_helper() {
        let mut inner = HashMap::new();
        inner.insert("value".to_string(), ConfigValue::String("nested".to_string()));

        let mut config = HashMap::new();
        config.insert("outer".to_string(), ConfigValue::Object(inner));

        let result = get_nested(&config, "outer.value");
        assert!(matches!(result, Some(ConfigValue::String(s)) if s == "nested"));
    }

    #[test]
    fn get_nested_deep_path() {
        let mut level2 = HashMap::new();
        level2.insert("key".to_string(), ConfigValue::Integer(123));

        let mut level1 = HashMap::new();
        level1.insert("inner".to_string(), ConfigValue::Object(level2));

        let mut config = HashMap::new();
        config.insert("outer".to_string(), ConfigValue::Object(level1));

        let result = get_nested(&config, "outer.inner.key");
        assert!(matches!(result, Some(ConfigValue::Integer(123))));
    }

    #[test]
    fn config_file_state_detects_change() {
        let temp_dir = std::env::temp_dir().join("runie_config_test");
        std::fs::create_dir_all(&temp_dir).ok();
        let config_path = temp_dir.join("config.toml");

        // Write initial config
        std::fs::write(&config_path, "key = \"value\"\n").unwrap();
        std::thread::sleep(Duration::from_millis(10));

        let mut state = ConfigFileState::new(config_path.clone());

        // First check should return changes
        let changes = state.check_and_update();
        assert!(changes.is_some());

        // Second check should return None (no change)
        let changes = state.check_and_update();
        assert!(changes.is_none());

        // Modify the file
        std::fs::write(&config_path, "key = \"new_value\"\n").unwrap();
        std::thread::sleep(Duration::from_millis(10));

        // Should detect change
        let changes = state.check_and_update();
        assert!(changes.is_some());

        let _ = std::fs::remove_dir_all(temp_dir);
    }

    #[test]
    fn parse_config_file() {
        let temp_dir = std::env::temp_dir().join("runie_parse_test");
        std::fs::create_dir_all(&temp_dir).ok();
        let config_path = temp_dir.join("config.toml");

        std::fs::write(
            &config_path,
            r#"
name = "runie"
max_tokens = 4096
enabled = true
"#,
        )
        .unwrap();

        let config = parse_config(&config_path).unwrap();
        assert_eq!(get_string(&config, "name"), Some("runie".to_string()));
        assert_eq!(get_integer(&config, "max_tokens"), Some(4096));
        assert_eq!(get_bool(&config, "enabled"), Some(true));

        let _ = std::fs::remove_dir_all(temp_dir);
    }
}
