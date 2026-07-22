use runie_core::plugins::{
    discover_plugins, LoadedPlugin, PluginDiscovery, PluginManager, PluginManifest, PluginRegistry,
    PluginScope,
};
use std::path::PathBuf;
use tempfile::TempDir;

fn temp_manifest(name: &str, version: &str) -> PluginManifest {
    PluginManifest {
        name: name.to_string(),
        version: Some(version.to_string()),
        description: Some("Test plugin".to_string()),
        author: Some("Test".to_string()),
        entrypoint: Some("mod.rs".to_string()),
        scopes: vec!["tool".to_string()],
    }
}

#[test]
fn test_manifest_validation_empty_name() {
    let manifest = PluginManifest {
        name: String::new(),
        version: None,
        description: None,
        author: None,
        entrypoint: None,
        scopes: vec![],
    };
    assert!(manifest.validate().is_err());
}

#[test]
fn test_manifest_validation_valid() {
    let manifest = temp_manifest("test-plugin", "1.0.0");
    assert!(manifest.validate().is_ok());
}

#[test]
fn test_manifest_validation_path_separator() {
    let manifest = PluginManifest {
        name: "bad/name".to_string(),
        version: None,
        description: None,
        author: None,
        entrypoint: None,
        scopes: vec![],
    };
    assert!(manifest.validate().is_err());
}

#[test]
fn test_plugin_scope_as_str() {
    assert_eq!(PluginScope::Local.as_str(), "local");
    assert_eq!(PluginScope::Repo.as_str(), "repo");
    assert_eq!(PluginScope::User.as_str(), "user");
    assert_eq!(PluginScope::Config.as_str(), "config");
    assert_eq!(PluginScope::Bundled.as_str(), "bundled");
}

#[test]
fn test_plugin_registry_register() {
    let mut registry = PluginRegistry::new();
    let plugin = LoadedPlugin {
        manifest: temp_manifest("test-plugin", "1.0.0"),
        root: PathBuf::from("/tmp/test"),
        enabled: true,
    };
    registry.register(plugin);
    assert_eq!(registry.list().len(), 1);
}

#[test]
fn test_plugin_registry_enable_disable() {
    let mut registry = PluginRegistry::new();
    let plugin = LoadedPlugin {
        manifest: temp_manifest("test-plugin", "1.0.0"),
        root: PathBuf::from("/tmp/test"),
        enabled: true,
    };
    registry.register(plugin);
    registry.disable("test-plugin");
    assert!(!registry.get("test-plugin").unwrap().enabled);
    registry.enable("test-plugin");
    assert!(registry.get("test-plugin").unwrap().enabled);
}

#[test]
fn test_plugin_registry_unregister() {
    let mut registry = PluginRegistry::new();
    let plugin = LoadedPlugin {
        manifest: temp_manifest("test-plugin", "1.0.0"),
        root: PathBuf::from("/tmp/test"),
        enabled: true,
    };
    registry.register(plugin);
    registry.unregister("test-plugin");
    assert!(registry.list().is_empty());
}

#[test]
fn test_plugin_registry_enabled_list() {
    let mut registry = PluginRegistry::new();
    let plugin1 = LoadedPlugin {
        manifest: temp_manifest("plugin1", "1.0.0"),
        root: PathBuf::from("/tmp/test"),
        enabled: true,
    };
    let plugin2 = LoadedPlugin {
        manifest: temp_manifest("plugin2", "1.0.0"),
        root: PathBuf::from("/tmp/test"),
        enabled: false,
    };
    registry.register(plugin1);
    registry.register(plugin2);
    assert_eq!(registry.enabled().len(), 1);
}

#[test]
fn test_plugin_discovery_new() {
    let discovery = PluginDiscovery::new();
    let discovered = discovery.discover();
    assert!(discovered.is_empty());
}

#[test]
fn test_plugin_discovery_with_config_paths() {
    let discovery = PluginDiscovery::new().with_config_paths(vec![]);
    assert!(discovery.discover().is_empty());
}

#[test]
fn test_plugin_manager_new() {
    let manager = PluginManager::new();
    assert!(manager.list_plugins().is_empty());
}

#[test]
fn test_plugin_manager_install() {
    let temp_dir = TempDir::new().unwrap();
    let plugin_dir = temp_dir.path().join("test-plugin");
    std::fs::create_dir(&plugin_dir).unwrap();
    let manifest_path = plugin_dir.join("manifest.json");
    std::fs::write(
        &manifest_path,
        r#"{"name":"install-test","version":"1.0.0"}"#,
    )
    .unwrap();
    let mut manager = PluginManager::new();
    manager.install(plugin_dir.to_str().unwrap()).unwrap();
    assert_eq!(manager.list_plugins().len(), 1);
}

#[test]
fn test_plugin_manager_install_missing_manifest() {
    let temp_dir = TempDir::new().unwrap();
    let plugin_dir = temp_dir.path().join("test-plugin");
    std::fs::create_dir(&plugin_dir).unwrap();
    let mut manager = PluginManager::new();
    assert!(manager.install(plugin_dir.to_str().unwrap()).is_err());
}

#[test]
fn test_plugin_manager_uninstall() {
    let temp_dir = TempDir::new().unwrap();
    let plugin_dir = temp_dir.path().join("test-plugin");
    std::fs::create_dir(&plugin_dir).unwrap();
    let manifest_path = plugin_dir.join("manifest.json");
    std::fs::write(
        &manifest_path,
        r#"{"name":"uninstall-test","version":"1.0.0"}"#,
    )
    .unwrap();
    let mut manager = PluginManager::new();
    manager.install(plugin_dir.to_str().unwrap()).unwrap();
    manager.uninstall("uninstall-test").unwrap();
    assert!(manager.list_plugins().is_empty());
}

#[test]
fn test_plugin_manager_uninstall_not_found() {
    let mut manager = PluginManager::new();
    assert!(manager.uninstall("nonexistent").is_err());
}

#[test]
fn test_plugin_manager_enable_disable() {
    let temp_dir = TempDir::new().unwrap();
    let plugin_dir = temp_dir.path().join("test-plugin");
    std::fs::create_dir(&plugin_dir).unwrap();
    let manifest_path = plugin_dir.join("manifest.json");
    std::fs::write(
        &manifest_path,
        r#"{"name":"toggle-test","version":"1.0.0"}"#,
    )
    .unwrap();
    let mut manager = PluginManager::new();
    manager.install(plugin_dir.to_str().unwrap()).unwrap();
    manager.disable("toggle-test");
    assert!(!manager.list_plugins()[0].enabled);
    manager.enable("toggle-test");
    assert!(manager.list_plugins()[0].enabled);
}

#[test]
fn test_discover_plugins_empty() {
    let plugins = discover_plugins();
    assert!(plugins.is_empty());
}
