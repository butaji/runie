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
/// - Compares the provider/model with the last known values
/// - Emits SwitchModel events when they change
pub fn spawn_config_watcher(
    event_tx: mpsc::Sender<Event>,
    config_path: PathBuf,
) -> tokio::task::JoinHandle<()> {
    tokio::spawn(async move {
        let mut state = WatcherState::default();
        let mut poll_interval = interval(Duration::from_secs(2));

        loop {
            poll_interval.tick().await;
            if tick(&config_path, &event_tx, &mut state).await.is_err() {
                break;
            }
        }
    })
}

#[derive(Default)]
struct WatcherState {
    last_provider: Option<String>,
    last_model: Option<String>,
    last_theme: Option<String>,
}

async fn tick(
    config_path: &PathBuf,
    event_tx: &mpsc::Sender<Event>,
    state: &mut WatcherState,
) -> Result<(), mpsc::error::SendError<Event>> {
    let config = Config::load_from(config_path);
    let (current_provider, current_model, current_theme) = current_config_values(&config);

    let provider_changed = state.last_provider.as_ref() != Some(&current_provider);
    let model_changed = state.last_model.as_ref() != Some(&current_model);
    let theme_changed = state.last_theme.as_ref() != Some(&current_theme);

    if provider_changed || model_changed {
        send_event(
            event_tx,
            Event::SwitchModel {
                provider: current_provider.clone(),
                model: current_model.clone(),
            },
        )
        .await?;
    }

    if theme_changed {
        send_event(
            event_tx,
            Event::SwitchTheme {
                name: current_theme.clone(),
            },
        )
        .await?;
    }

    state.last_provider = Some(current_provider);
    state.last_model = Some(current_model);
    state.last_theme = Some(current_theme);
    Ok(())
}

fn current_config_values(config: &Config) -> (String, String, String) {
    let (default_provider, default_model) = default_provider_model();
    let provider = config
        .provider
        .clone()
        .unwrap_or_else(|| default_provider.to_string());
    let model = config.default_model().unwrap_or(default_model).to_string();
    let theme = config.theme.clone().unwrap_or_else(|| "runie".to_string());
    (provider, model, theme)
}

async fn send_event(
    tx: &mpsc::Sender<Event>,
    evt: Event,
) -> Result<(), mpsc::error::SendError<Event>> {
    tx.send(evt).await
}

fn default_provider_model() -> (&'static str, &'static str) {
    if crate::provider_registry::is_mock_enabled() {
        ("mock", "echo")
    } else {
        ("", "")
    }
}

#[cfg(test)]
mod tests {
    use std::fs;
    use std::time::Duration;

    use tokio::sync::mpsc;

    use super::spawn_config_watcher;
    use crate::event::Event;

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

        tokio::time::sleep(Duration::from_secs(4)).await;

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

        tokio::time::sleep(Duration::from_secs(4)).await;
        while rx.try_recv().is_ok() {}

        write_test_config(&config_path, "anthropic", "claude-3", "anthropic");

        tokio::time::sleep(Duration::from_secs(3)).await;

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
