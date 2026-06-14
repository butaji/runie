//! Config file watcher for hot reload.

use std::path::PathBuf;
use std::time::Duration;
use tokio::sync::mpsc;
use tokio::time::interval;

use crate::event::Event;

use super::types::Config;

/// Start a config file watcher that monitors for changes and emits SwitchModel events.
///
/// Returns a tokio::JoinHandle that can be used to stop the watcher.
///
/// The watcher:
/// - Polls the config file every 2 seconds
/// - Classifies changes via `Config::classify_change`
/// - Emits SwitchModel / SwitchTheme events when relevant fields change
pub fn spawn_config_watcher(
    event_tx: mpsc::Sender<Event>,
    config_path: PathBuf,
) -> tokio::task::JoinHandle<()> {
    tokio::spawn(async move {
        let mut last_config = Config::default();
        let mut poll_interval = interval(Duration::from_secs(2));

        loop {
            poll_interval.tick().await;

            let config = Config::load_from(&config_path);

            if let Some(change) = config.classify_change(&last_config) {
                match change {
                    super::types::ConfigChange::Model { provider, model } => {
                        let _ = event_tx.send(Event::SwitchModel { provider, model }).await;
                    }
                    super::types::ConfigChange::Theme { name } => {
                        let _ = event_tx.send(Event::SwitchTheme { name }).await;
                    }
                }
            }

            last_config = config;
        }
    })
}

#[cfg(test)]
mod tests {
    use std::fs;
    use std::time::Duration;
    use tokio::sync::mpsc;

    use super::spawn_config_watcher;
    use crate::event::Event;

    /// Poll interval is 2s; wait 2.1s to guarantee at least one poll cycle.
    /// We initialise last_config from the file immediately on startup (before the
    /// first tick), so the first tick fires an event if the file differs from
    /// the default config.
    const POLL_WAIT: Duration = Duration::from_millis(2100);

    #[tokio::test]
    async fn config_watcher_detects_initial_change() {
        let dir = tempfile::tempdir().unwrap();
        let config_path = dir.path().join("config.toml");

        fs::write(
            &config_path,
            r#"
provider = "openai"
model = "gpt-4"

[model_providers.openai]
type = "openai"
base_url = "https://api.openai.com"
api_key = "test"
"#,
        )
        .unwrap();

        let (tx, mut rx) = mpsc::channel::<Event>(10);
        let handle = spawn_config_watcher(tx, config_path.clone());

        tokio::time::sleep(POLL_WAIT).await;

        let evt = tokio::time::timeout(Duration::from_secs(1), rx.recv()).await;
        assert!(evt.is_ok(), "Should receive SwitchModel event");
        assert!(matches!(evt.unwrap(), Some(Event::SwitchModel { .. })));

        handle.abort();
    }

    #[tokio::test]
    async fn config_watcher_parses_toml_changes() {
        let dir = tempfile::tempdir().unwrap();
        let config_path = dir.path().join("config.toml");

        write_test_config(&config_path, "openai", "gpt-4", "openai");

        let (tx, mut rx) = mpsc::channel::<Event>(10);
        let handle = spawn_config_watcher(tx, config_path.clone());

        // Drain the initial event
        let _ = tokio::time::timeout(Duration::from_secs(5), rx.recv()).await;

        write_test_config(&config_path, "anthropic", "claude-3", "anthropic");

        tokio::time::sleep(POLL_WAIT).await;

        let evt = tokio::time::timeout(Duration::from_secs(1), rx.recv()).await;
        assert!(evt.is_ok(), "Should receive SwitchModel event");

        if let Ok(Some(Event::SwitchModel { provider, model })) = evt {
            assert_eq!(provider, "anthropic");
            assert_eq!(model, "claude-3");
        } else {
            panic!("Expected SwitchModel event");
        }

        handle.abort();
    }

    fn write_test_config(path: &std::path::Path, provider: &str, model: &str, section: &str) {
        fs::write(
            path,
            format!(
                r#"
provider = "{provider}"
model = "{model}"

[model_providers.{section}]
type = "{section}"
base_url = "https://api.{provider}.com"
api_key = "test"
"#
            ),
        )
        .unwrap();
    }
}
