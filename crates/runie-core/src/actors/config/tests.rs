//! Unit tests for `RactorConfigActor`.

use tempfile::TempDir;

use crate::actors::RactorConfigActor;
use crate::bus::EventBus;
use crate::event::Event;

fn temp_config_path() -> (TempDir, std::path::PathBuf) {
    let dir = TempDir::new().unwrap();
    let path = dir.path().join("config.toml");
    (dir, path)
}

#[tokio::test]
async fn config_actor_loads_and_emits_config_loaded() {
    let bus = EventBus::<Event>::new(10);
    let mut sub = bus.subscribe();
    let (_handle, _actor) = RactorConfigActor::spawn(bus, None).await;

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
    let (handle, _actor) = RactorConfigActor::spawn(bus, Some(path)).await;

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
    let (handle, _actor) = RactorConfigActor::spawn(bus, Some(path)).await;

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
    let (_handle, _actor) = RactorConfigActor::spawn(bus, Some(path_clone)).await;

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
    let (handle, _actor) = RactorConfigActor::spawn(bus.clone(), Some(path_clone)).await;

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
    let (handle, _actor) = RactorConfigActor::spawn(bus, Some(path)).await;

    // Drain initial load.
    let _ = sub.recv().await;

    handle
        .set_default_model("test".into(), "model-x".into())
        .await;

    let event = tokio::time::timeout(std::time::Duration::from_secs(2), sub.recv())
        .await
        .unwrap()
        .unwrap();
    let Event::ConfigLoaded { config } = event else {
        panic!("expected ConfigLoaded after successful mutation");
    };
    assert_eq!(config.models.default, Some("model-x".into()));
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
    let (handle, _actor) = RactorConfigActor::spawn(bus.clone(), Some(path_clone)).await;

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
    let (handle, _actor) = RactorConfigActor::spawn(bus, Some(path)).await;

    // Drain initial load.
    let _ = sub.recv().await;

    handle
        .save_provider(
            "claude".into(),
            "https://api.anthropic.com".into(),
            "sk-test-key".into(),
            vec!["claude-3-5-sonnet".into()],
        )
        .await;

    let event = tokio::time::timeout(std::time::Duration::from_secs(2), sub.recv())
        .await
        .unwrap()
        .unwrap();
    let Event::ConfigLoaded { config } = event else {
        panic!("expected ConfigLoaded after save_provider, got {event:?}");
    };
    assert!(config.model_providers.contains_key("claude"));
    let provider = config.model_providers.get("claude").unwrap();
    assert_eq!(provider.base_url, "https://api.anthropic.com");
    assert_eq!(provider.models, vec!["claude-3-5-sonnet"]);
}
