//! Config file watcher using notify crate for hot reload.

use std::path::PathBuf;
use notify_debouncer_mini::{new_debouncer, DebouncedEventKind};
use tokio::sync::mpsc;
use crate::event::{Event, ModelConfigEvent};
use super::types::Config;

/// Start a config file watcher using notify crate.
pub fn spawn_config_watcher(event_tx: mpsc::Sender<Event>, config_path: PathBuf) -> tokio::task::JoinHandle<()> {
    tokio::spawn(async move {
        let mut last_config = Config::load_from(&config_path);
        let (tx, rx) = std::sync::mpsc::channel();
        let mut debouncer = match new_debouncer(std::time::Duration::from_millis(300), tx) {
            Ok(d) => d, Err(e) => { eprintln!("Failed to create watcher: {:?}", e); return; }
        };
        if let Some(parent) = config_path.parent() {
            if let Err(e) = debouncer.watcher().watch(parent, notify::RecursiveMode::NonRecursive) {
                eprintln!("Failed to watch config dir: {:?}", e);
            }
        }
        while let Ok(Ok(events)) = rx.recv() {
            if !events.iter().any(|e| &e.path == &config_path) { continue; }
            if !events.iter().any(|e| matches!(e.kind, DebouncedEventKind::Any | DebouncedEventKind::AnyContinuous)) { continue; }
            let config = Config::load_from(&config_path);
            for change in config.classify_change(&last_config) {
                match change {
                    super::types::ConfigChange::Model { provider, model } => { let _ = event_tx.send(Event::ModelConfig(ModelConfigEvent::SwitchModel { provider, model })).await; }
                    super::types::ConfigChange::Theme { name } => { let _ = event_tx.send(Event::ModelConfig(ModelConfigEvent::SwitchTheme { name })).await; }
                    super::types::ConfigChange::Keybindings => { let _ = event_tx.send(Event::ModelConfig(ModelConfigEvent::KeybindingsReloaded)).await; }
                }
            }
            last_config = config;
        }
    })
}

#[cfg(test)]
mod tests {
    use std::fs; use std::time::Duration; use tokio::sync::mpsc;
    use super::spawn_config_watcher;
    use crate::event::{Event, ModelConfigEvent};

    #[tokio::test] #[ignore] async fn config_watcher_detects_initial_change() {
        let dir = tempfile::tempdir().unwrap();
        let config_path = dir.path().join("config.toml");
        fs::write(&config_path, r#"provider = "openai" model = "gpt-4""#).unwrap();
        let (tx, mut rx) = mpsc::channel::<Event>(10);
        let handle = spawn_config_watcher(tx, config_path);
        let evt = tokio::time::timeout(Duration::from_secs(5), rx.recv()).await;
        assert!(evt.is_ok() && matches!(evt.unwrap(), Some(Event::ModelConfig(ModelConfigEvent::SwitchModel { .. }))));
        handle.abort();
    }

    fn write_test_config(path: &std::path::Path, provider: &str, model: &str) {
        fs::write(path, format!(r#"provider = "{provider}" model = "{model}""#)).unwrap();
    }
}
