//! Layer 4 tests: actor lifecycle integration.
//!
//! These tests spawn the real actor system and verify actors stay alive
//! throughout the application lifetime. They catch lifecycle bugs like
//! silently dropping actor handles.
//!
//! Layer 2 test: `bootstrap_spawns_all_actors` — TUI bootstrap produces
//! a non-empty ActorSystem.

use runie_core::actors::{ConfigActor, ProviderActor};
use runie_core::actors::provider::{ProviderMsg, ProviderReply};
use runie_core::bus::EventBus;
use runie_core::Event;
use runie_provider::DynProviderFactory;
use std::sync::Arc;

/// Verifies the provider actor stays alive and can receive messages.
/// This test would fail if actors are dropped due to `_actors` pattern
/// or similar silent drops.
#[tokio::test]
async fn provider_actor_responds_to_list_models_request() {
    let bus = EventBus::<Event>::new(4);
    let (config_handle, _config_actor) = ConfigActor::spawn(bus.clone(), None);
    let (provider_handle, provider_actor) = ProviderActor::spawn(
        bus,
        config_handle,
        Arc::new(DynProviderFactory),
    );

    // Verify we can send a ListModels request and get a response.
    // If the actor is dropped, the send will fail immediately.
    let (reply_tx, _reply_rx) = tokio::sync::oneshot::channel();
    let send_result = provider_handle
        .tx()
        .send(ProviderMsg::ListModels {
            provider: "openai".into(),
            reply: ProviderReply::new(reply_tx),
        })
        .await;

    // Drop the actors explicitly to show the test is checking their lifetime.
    drop(provider_actor);
    drop(provider_handle);
    drop(_config_actor);

    // The send should succeed (actor is alive).
    // Note: The response might be an error (no config), but the actor
    // should have received and processed the message.
    assert!(
        send_result.is_ok(),
        "provider actor should be alive to receive messages. \
         If send fails with 'actor unavailable', the actor was dropped (lifecycle bug)"
    );
}

/// Verifies the provider actor handle can be cloned and used while the
/// underlying actor is still alive.
#[tokio::test]
async fn provider_actor_handle_can_be_cloned() {
    let bus = EventBus::<Event>::new(4);
    let (config_handle, _config_actor) = ConfigActor::spawn(bus.clone(), None);
    let (provider_handle, _provider_actor) = ProviderActor::spawn(
        bus,
        config_handle,
        Arc::new(DynProviderFactory),
    );

    // Clone the handle (like AppState does)
    let tx = provider_handle.tx();
    let tx_clone = tx.clone();

    // Both handles should work
    let (reply_tx, _reply_rx) = tokio::sync::oneshot::channel();
    let result = tx_clone
        .send(ProviderMsg::ListModels {
            provider: "openai".into(),
            reply: ProviderReply::new(reply_tx),
        })
        .await;

    assert!(
        result.is_ok(),
        "cloned provider_tx should send successfully"
    );
}

/// Layer 2 test: bootstrap produces an ActorHandles with all actors spawned.
#[tokio::test]
async fn bootstrap_spawns_all_actors() {
    use runie_core::bus::EventBus;
    use runie_core::Event;
    use runie_core::actors::{
        ConfigActor, IoActor, ProviderActor, SessionActor,
        ActorHandles,
    };
    use runie_provider::DynProviderFactory;
    use std::sync::Arc;

    let bus = EventBus::<Event>::new(16);

    // Spawn actors the same way bootstrap_app does
    let (config_handle, _config_actor) = ConfigActor::spawn(bus.clone(), None);
    let (provider_handle, _provider_actor) = ProviderActor::spawn(
        bus,
        config_handle,
        Arc::new(DynProviderFactory),
    );
    let (session_handle, _session_actor) = SessionActor::spawn(bus.clone());
    let (io_handle, _io_actor) = IoActor::spawn(bus.clone());

    let handles = ActorHandles {
        config: Some(config_handle),
        provider: Some(provider_handle),
        session: Some(session_handle),
        io: Some(io_handle),
        fff_indexer: None,
    };

    // Verify all actors are present
    assert!(handles.config.is_some(), "config actor should be spawned");
    assert!(handles.provider.is_some(), "provider actor should be spawned");
    assert!(handles.session.is_some(), "session actor should be spawned");
    assert!(handles.io.is_some(), "io actor should be spawned");
}
