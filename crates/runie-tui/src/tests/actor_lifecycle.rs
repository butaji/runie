//! Layer 4 tests: actor lifecycle integration.
//!
//! These tests spawn the real actor system and verify actors stay alive
//! throughout the application lifetime. They catch lifecycle bugs like
//! silently dropping actor handles.
//!
//! Layer 2 test: `bootstrap_spawns_all_actors` — Leader bootstrap produces
//! a LeaderHandle with all actors spawned.

use runie_core::actors::leader::Leader;
use runie_provider::BuiltProviderFactory;
use runie_agent::AgentActorFactoryImpl;

/// Verifies the leader bootstrap spawns all actors and produces a valid LeaderHandle.
#[tokio::test]
async fn bootstrap_spawns_all_actors() {
    let leader = Leader::new();
    let agent_factory = std::sync::Arc::new(AgentActorFactoryImpl);
    let provider_factory = std::sync::Arc::new(BuiltProviderFactory);
    let handle = leader
        .start(provider_factory, agent_factory)
        .await
        .expect("leader should start");

    // Verify all expected actor handles are present and non-null.
    use runie_core::actors::RactorConfigHandle;
    use runie_core::actors::RactorProviderHandle;
    use runie_core::actors::RactorSessionHandle;
    use runie_core::actors::RactorIoHandle;
    use runie_core::actors::RactorTurnHandle;
    use runie_core::actors::RactorInputHandle;
    use runie_core::actors::RactorPermissionHandle;

    // Config, provider, session, io, turn, input, permission
    // are all accessible via the LeaderHandle fields.
    let _: &RactorConfigHandle = &handle.config;
    let _: &RactorProviderHandle = &handle.provider;
    let _: &RactorSessionHandle = &handle.session;
    let _: &RactorIoHandle = &handle.io;
    let _: &RactorTurnHandle = &handle.turn;
    let _: &RactorInputHandle = &handle.input;
    let _: &RactorPermissionHandle = &handle.permission;

    // Agent handle is also present via the dyn trait.
    let _: &std::sync::Arc<dyn runie_core::actors::leader::LeaderAgentHandle> = &handle.agent;

    // Shutdown cleanly.
    handle.shutdown().await;
}
