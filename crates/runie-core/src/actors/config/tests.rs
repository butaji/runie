//! Unit tests for `RactorConfigActor`.

use tempfile::TempDir;

use crate::actors::RactorConfigActor;
use crate::bus::EventBus;
use crate::commands::dsl::handlers::model::handle_model;
use crate::commands::{CommandResult, DialogType};
use crate::config::ModelProvider;
use crate::event::Event;
use crate::model::{AppState, ModelSource};

fn temp_config_path() -> (TempDir, std::path::PathBuf) {
    let dir = TempDir::new().unwrap();
    let path = dir.path().join("config.toml");
    (dir, path)
}

#[tokio::test]
async fn config_actor_loads_and_emits_config_loaded() {
    // Use an isolated empty config so the test does not depend on the
    // developer's real `~/.runie/config.toml` validating cleanly.
    let (_dir, path) = temp_config_path();
    let bus = EventBus::<Event>::new(10);
    let mut sub = bus.subscribe();
    let (_handle, _actor, _join) = RactorConfigActor::spawn(bus, Some(path), None)
        .await
        .unwrap();

    let event = tokio::time::timeout(std::time::Duration::from_secs(2), sub.recv())
        .await
        .unwrap()
        .unwrap();
    assert!(matches!(event, Event::ConfigLoaded { .. }));
}

#[tokio::test]
async fn config_actor_save_provider_persists_and_reloads() {
    let (_dir, path) = temp_config_path();
    let bus = EventBus::<Event>::new(10);
    let mut sub = bus.subscribe();
    let (handle, _actor, _join) = RactorConfigActor::spawn(bus, Some(path), None)
        .await
        .unwrap();

    // Drain initial load.
    let _ = sub.recv().await;

    handle
        .save_provider(
            "openai".into(),
            "https://api.openai.com".into(),
            "sk-test".into(),
            vec!["gpt-4o".into()],
        )
        .await;

    let event = tokio::time::timeout(std::time::Duration::from_secs(2), sub.recv())
        .await
        .unwrap()
        .unwrap();
    let Event::ConfigLoaded { config } = event else {
        panic!("expected ConfigLoaded, got {event:?}");
    };
    assert!(config.model_providers.contains_key("openai"));

    let providers = handle.get_configured_providers().await.unwrap();
    assert_eq!(providers.len(), 1);
    assert_eq!(providers[0].0, "openai");
}

#[tokio::test]
async fn config_actor_set_default_model_updates_active_model() {
    let (_dir, path) = temp_config_path();
    let bus = EventBus::<Event>::new(10);
    let mut sub = bus.subscribe();
    let (handle, _actor, _join) = RactorConfigActor::spawn(bus, Some(path), None)
        .await
        .unwrap();

    // Drain initial load.
    let _ = sub.recv().await;

    handle
        .set_default_model("openai".into(), "gpt-4o".into())
        .await;

    let event = tokio::time::timeout(std::time::Duration::from_secs(2), sub.recv())
        .await
        .unwrap()
        .unwrap();
    let Event::ConfigLoaded { config } = event else {
        panic!("expected ConfigLoaded, got {event:?}");
    };
    assert_eq!(config.provider, Some("openai".into()));
    assert_eq!(config.models.default, Some("gpt-4o".into()));
}

#[tokio::test]
#[ignore]
async fn config_actor_watcher_reloads_on_external_change() {
    let (_dir, path) = temp_config_path();
    std::fs::write(&path, r#"provider = "openai""#).unwrap();

    let bus = EventBus::<Event>::new(10);
    let mut sub = bus.subscribe();
    let path_clone = path.clone();
    let (_handle, _actor, _join) = RactorConfigActor::spawn(bus, Some(path_clone), None)
        .await
        .unwrap();

    // Drain initial load.
    let _ = sub.recv().await;

    std::fs::write(&path, r#"provider = "anthropic""#).unwrap();

    let event = tokio::time::timeout(std::time::Duration::from_secs(5), sub.recv())
        .await
        .unwrap()
        .unwrap();
    assert!(matches!(event, Event::ConfigLoaded { .. }));
}

#[tokio::test]
async fn config_actor_emits_error_on_failed_save() {
    let tmp = tempfile::tempdir().unwrap();
    let path = tmp.path().join("config.toml");
    // Make the directory read-only after creating the actor so initial load succeeds.
    let readonly = tmp.path().to_path_buf();
    let perms = std::fs::metadata(&readonly).unwrap().permissions();
    let mut readonly_perms = perms.clone();
    readonly_perms.set_readonly(true);

    let bus = EventBus::<Event>::new(8);
    let mut sub = bus.subscribe();
    let path_clone = path.clone();
    let (handle, _actor, _join) = RactorConfigActor::spawn(bus.clone(), Some(path_clone), None)
        .await
        .unwrap();

    // Drain initial ConfigLoaded.
    let _ = tokio::time::timeout(std::time::Duration::from_secs(2), sub.recv()).await;

    std::fs::set_permissions(&readonly, readonly_perms).unwrap();
    handle
        .save_provider(
            "openai".into(),
            "https://api.openai.com".into(),
            "sk-test".into(),
            vec!["gpt-4o".into()],
        )
        .await;

    let mut saw_error = false;
    for _ in 0..20 {
        if let Ok(Ok(Event::Error { .. })) =
            tokio::time::timeout(std::time::Duration::from_millis(50), sub.recv()).await
        {
            saw_error = true;
            break;
        }
    }

    std::fs::set_permissions(&readonly, perms).unwrap();
    assert!(saw_error, "expected Event::Error after failed write");
}

// ─────────────────────────────────────────────────────────────────────────────
// Layer 1 & 2: mutate_config helper behavior (verified through public interface)
// ─────────────────────────────────────────────────────────────────────────────

/// Layer 1: Helper reloads config and emits event on success.
#[tokio::test]
async fn mutate_config_helper_emits_event_on_success() {
    let (_dir, path) = temp_config_path();
    let bus = EventBus::<Event>::new(10);
    let mut sub = bus.subscribe();
    let (handle, _actor, _join) = RactorConfigActor::spawn(bus, Some(path), None)
        .await
        .unwrap();

    // Drain initial load.
    let _ = sub.recv().await;

    // Use a known provider so validation passes.
    handle
        .set_default_model("openai".into(), "gpt-4o".into())
        .await;

    let event = tokio::time::timeout(std::time::Duration::from_secs(2), sub.recv())
        .await
        .unwrap()
        .unwrap();
    let Event::ConfigLoaded { config } = event else {
        panic!("expected ConfigLoaded after successful mutation");
    };
    assert_eq!(config.models.default, Some("gpt-4o".into()));
}

/// Layer 1: Helper does not swallow I/O errors.
#[tokio::test]
async fn mutate_config_helper_reports_error() {
    let tmp = tempfile::tempdir().unwrap();
    let path = tmp.path().join("config.toml");
    let readonly = tmp.path().to_path_buf();
    let perms = std::fs::metadata(&readonly).unwrap().permissions();
    let mut readonly_perms = perms.clone();
    readonly_perms.set_readonly(true);

    let bus = EventBus::<Event>::new(8);
    let mut sub = bus.subscribe();
    let path_clone = path.clone();
    let (handle, _actor, _join) = RactorConfigActor::spawn(bus.clone(), Some(path_clone), None)
        .await
        .unwrap();

    // Drain initial ConfigLoaded.
    let _ = tokio::time::timeout(std::time::Duration::from_secs(2), sub.recv()).await;

    std::fs::set_permissions(&readonly, readonly_perms).unwrap();
    handle.remove_provider("nonexistent".into()).await;

    let mut saw_error = false;
    for _ in 0..20 {
        if let Ok(Ok(Event::Error { id, .. })) =
            tokio::time::timeout(std::time::Duration::from_millis(50), sub.recv()).await
        {
            if id == "config" {
                saw_error = true;
                break;
            }
        }
    }

    std::fs::set_permissions(&readonly, perms).unwrap();
    assert!(
        saw_error,
        "expected Event::Error from config actor after failed write"
    );
}

/// Layer 2: SaveProvider event still flows through the refactored helper.
#[tokio::test]
async fn save_provider_event_still_flows() {
    let (_dir, path) = temp_config_path();
    let bus = EventBus::<Event>::new(10);
    let mut sub = bus.subscribe();
    let (handle, _actor, _join) = RactorConfigActor::spawn(bus, Some(path), None)
        .await
        .unwrap();

    // Drain initial load.
    let _ = sub.recv().await;

    // Use a known provider so validation passes.
    handle
        .save_provider(
            "anthropic".into(),
            "https://api.anthropic.com/v1".into(),
            "sk-test-key".into(),
            vec!["claude-sonnet-4-6".into()],
        )
        .await;

    let event = tokio::time::timeout(std::time::Duration::from_secs(2), sub.recv())
        .await
        .unwrap()
        .unwrap();
    let Event::ConfigLoaded { config } = event else {
        panic!("expected ConfigLoaded after save_provider, got {event:?}");
    };
    assert!(config.model_providers.contains_key("anthropic"));
    let provider = config.model_providers.get("anthropic").unwrap();
    assert_eq!(provider.base_url, "https://api.anthropic.com/v1");
    assert_eq!(provider.models, vec!["claude-sonnet-4-6"]);
}

#[tokio::test]
async fn tracing_event_emitted_on_config_load() {
    // Verify that a ConfigLoaded fact produces a matching tracing::info! event.
    use std::sync::mpsc;
    use tracing::Subscriber;
    use tracing_subscriber::{layer::SubscriberExt, Registry};

    struct CaptureLayer {
        sender: mpsc::Sender<tracing::Level>,
    }

    impl<S: Subscriber> tracing_subscriber::layer::Layer<S> for CaptureLayer {
        fn on_event(
            &self,
            event: &tracing::Event<'_>,
            _ctx: tracing_subscriber::layer::Context<'_, S>,
        ) {
            let _ = self.sender.send(*event.metadata().level());
        }
    }

    let (tx, rx) = mpsc::channel();
    let layer = CaptureLayer { sender: tx };
    let dispatcher = tracing::dispatcher::Dispatch::new(Registry::default().with(layer));
    let guard = tracing::dispatcher::set_global_default(dispatcher);

    // Isolated empty config: do not depend on the developer's real config validating.
    let (_dir, path) = temp_config_path();
    let bus = EventBus::<Event>::new(10);
    let mut sub = bus.subscribe();
    let (_handle, _actor, _join) = RactorConfigActor::spawn(bus, Some(path), None)
        .await
        .unwrap();

    // Verify ConfigLoaded fact is emitted.
    let event = tokio::time::timeout(std::time::Duration::from_secs(2), sub.recv())
        .await
        .unwrap()
        .unwrap();
    assert!(matches!(event, Event::ConfigLoaded { .. }));

    // Verify a matching tracing::info event was also emitted.
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(2);
    let mut found_info = false;
    while let Ok(level) = rx.recv_timeout(deadline - std::time::Instant::now()) {
        if level == tracing::Level::INFO {
            found_info = true;
            break;
        }
    }
    assert!(
        found_info,
        "tracing::info! should be emitted on ConfigLoaded"
    );

    drop(guard);
}

// CaptureLayer is defined above.

/// Regression: connecting a provider whose config fails `validate_full` (e.g. a
/// custom/OpenAI-compatible provider not in the bundled registry, or an
/// aggregator whose models carry an upstream prefix) must not break `/model`.
///
/// Root cause (fixed): `load_and_emit` used to re-emit the *previous* config as
/// `ConfigLoaded` when the reloaded config failed validation. `AppState::apply_config`
/// then overwrote the eagerly-synced `model_providers` with that stale/empty map,
/// so `/model` reported "No connected providers" even though the user had just
/// connected one (and the input box still showed the active model).
///
/// The fix makes `load_and_emit` decline to emit `ConfigLoaded` on validation
/// failure (matching `reload_and_emit`), so the in-memory projection is preserved.
#[tokio::test]
async fn save_provider_that_fails_validation_does_not_clobber_projection() {
    use tokio::time::{timeout, Duration, Instant};

    let (_dir, path) = temp_config_path();
    let bus = EventBus::<Event>::new(32);
    let mut sub = bus.subscribe();
    let (handle, _actor, _join) = RactorConfigActor::spawn(bus, Some(path.clone()), None)
        .await
        .unwrap();

    // Drain initial load.
    let _ = timeout(Duration::from_secs(2), sub.recv()).await;

    // A provider NOT in the bundled registry -> `validate_full` fails on reload.
    handle
        .save_provider(
            "my-custom-llm".into(),
            "http://localhost:11434/v1".into(),
            String::new(),
            vec!["foo-model".into(), "bar-model".into()],
        )
        .await;

    // Collect everything the actor emits in response to the save. Stop once the
    // bus goes idle (no events for 150ms) or the safety deadline elapses.
    let mut events = Vec::new();
    let deadline = Instant::now() + Duration::from_secs(3);
    while Instant::now() < deadline {
        match timeout(Duration::from_millis(150), sub.recv()).await {
            Ok(Ok(evt)) => events.push(evt),
            _ => break,
        }
    }

    let saw_config_error = events
        .iter()
        .any(|e| matches!(e, Event::Error { id, .. } if id == "config"));
    let saw_config_loaded = events
        .iter()
        .any(|e| matches!(e, Event::ConfigLoaded { .. }));

    assert!(
        saw_config_error,
        "validation failure should still surface an Error event, got: {events:?}"
    );
    assert!(
        !saw_config_loaded,
        "actor must NOT emit a stale ConfigLoaded on validation failure \
         (it would clobber the in-memory projection); got: {events:?}"
    );

    // The provider is still persisted to disk; only the stale broadcast is gone.
    let raw = std::fs::read_to_string(&path).unwrap();
    assert!(
        raw.contains("my-custom-llm"),
        "provider should be persisted to disk despite validation failure:\n{raw}"
    );

    // End-to-end: an AppState that the login flow just eagerly synced (mimicking
    // `sync_config_cache` + activating a model) must survive the actor's events.
    let mut state = AppState::default();
    state.config_mut().model_providers_mut().insert(
        "my-custom-llm".into(),
        ModelProvider {
            provider_type: None,
            base_url: "http://localhost:11434/v1".into(),
            models: vec!["foo-model".into(), "bar-model".into()],
            headers: std::collections::HashMap::new(),
        },
    );
    state.config.current_provider = "my-custom-llm".into();
    state.config.current_model = "foo-model".into();
    state.config.model_source = ModelSource::UserOverride;

    for evt in events {
        state.update(evt);
    }

    assert!(
        state
            .configured_providers()
            .iter()
            .any(|(p, _, _)| p == "my-custom-llm"),
        "eagerly-synced provider must survive the actor's validation-failure response"
    );

    let result = handle_model(&mut state, "");
    assert!(
        matches!(result, CommandResult::OpenDialog(DialogType::ModelSelector)),
        "/model must open the selector, not report 'No connected providers'; got {result:?}"
    );
}
