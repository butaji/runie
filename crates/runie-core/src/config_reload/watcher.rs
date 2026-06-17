//! Config file watcher using notify crate for hot reload.

use super::types::Config;
use crate::event::{Event, ModelConfigEvent};
use notify_debouncer_mini::{new_debouncer, DebouncedEvent, DebouncedEventKind};
use std::path::PathBuf;
use tokio::sync::mpsc;

/// Start a config file watcher using notify crate.
pub fn spawn_config_watcher(
    event_tx: mpsc::Sender<Event>,
    config_path: PathBuf,
) -> tokio::task::JoinHandle<()> {
    tokio::spawn(async move {
        if let Err(e) = run_watcher_loop(event_tx, config_path).await {
            eprintln!("Config watcher failed: {:?}", e);
        }
    })
}

async fn run_watcher_loop(
    event_tx: mpsc::Sender<Event>,
    config_path: PathBuf,
) -> anyhow::Result<()> {
    let mut last_config = Config::load(Some(&config_path));
    let (tx, rx) = std::sync::mpsc::channel();
    let mut debouncer = new_debouncer(std::time::Duration::from_millis(300), tx)
        .map_err(|e| anyhow::anyhow!("Failed to create watcher: {e:?}"))?;
    if let Some(parent) = config_path.parent() {
        debouncer
            .watcher()
            .watch(parent, notify::RecursiveMode::NonRecursive)
            .map_err(|e| anyhow::anyhow!("Failed to watch config dir: {e:?}"))?;
    }
    while let Ok(Ok(events)) = rx.recv() {
        if !should_handle_config_event(&events, &config_path) {
            continue;
        }
        let config = Config::load(Some(&config_path));
        apply_config_changes(&event_tx, &config, &last_config).await;
        last_config = config;
    }
    Ok(())
}

fn should_handle_config_event(events: &[DebouncedEvent], config_path: &PathBuf) -> bool {
    let touches_config = events.iter().any(|e| e.path == *config_path);
    let has_relevant_kind = events.iter().any(|e| {
        matches!(
            e.kind,
            DebouncedEventKind::Any | DebouncedEventKind::AnyContinuous
        )
    });
    touches_config && has_relevant_kind
}

async fn apply_config_changes(
    event_tx: &mpsc::Sender<Event>,
    config: &Config,
    last_config: &Config,
) {
    for change in config.classify_change(last_config) {
        match change {
            super::types::ConfigChange::Model { provider, model } => {
                let _ = event_tx
                    .send(ModelConfigEvent::SwitchModel { provider, model })
                    .await;
            }
            super::types::ConfigChange::Theme { name } => {
                let _ = event_tx.send(ModelConfigEvent::SwitchTheme { name }).await;
            }
            super::types::ConfigChange::Keybindings => {
                let _ = event_tx.send(ModelConfigEvent::KeybindingsReloaded).await;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::spawn_config_watcher;
    use crate::event::{Event, ModelConfigEvent};
    use std::fs;
    use std::time::Duration;
    use tokio::sync::mpsc;

    #[tokio::test]
    #[ignore]
    async fn config_watcher_detects_initial_change() {
        let dir = tempfile::tempdir().unwrap();
        let config_path = dir.path().join("config.toml");
        fs::write(&config_path, r#"provider = "openai" model = "gpt-4""#).unwrap();
        let (tx, mut rx) = mpsc::channel::<Event>(10);
        let handle = spawn_config_watcher(tx, config_path);
        let evt = tokio::time::timeout(Duration::from_secs(5), rx.recv()).await;
        assert!(evt.is_ok() && matches!(evt.unwrap(), Some(ModelConfigEvent::SwitchModel { .. })));
        handle.abort();
    }

    fn write_test_config(path: &std::path::Path, provider: &str, model: &str) {
        fs::write(
            path,
            format!(r#"provider = "{provider}" model = "{model}""#),
        )
        .unwrap();
    }
}
