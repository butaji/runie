//! Test helpers for constructing a `LeaderHandle` with all actors spawned.
#![allow(clippy::too_many_lines)]

use crate::actors::provider::{BuiltProvider, ProviderFactory};
use crate::bus::EventBus;
use crate::provider::{Provider, ProviderError};
use crate::provider_event::ProviderEvent;
use crate::Event as CoreEvent;
use async_trait::async_trait;
use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;
use tokio::sync::mpsc;

use super::{LeaderAgentHandle, LeaderHandle, SpawnedHandles};

/// Construct a minimal `LeaderHandle` for unit tests.
///
/// Spawns all production actors with a shared event bus and returns
/// a `LeaderHandle` with default bus/command channels.
/// The caller takes ownership and must eventually call `shutdown()`.
pub async fn test_leader_handle() -> LeaderHandle {
    use super::LeaderAgentCmd;
    use crate::actors::turn::RactorTurnActor;
    use crate::actors::{
        RactorConfigActor, RactorPermissionActor, RactorProviderActor,
        spawn_input_actor, spawn_io_actor, spawn_session_actor,
    };

    struct NoOpAgentHandle;
    impl LeaderAgentHandle for NoOpAgentHandle {
        fn run(&self, _cmd: LeaderAgentCmd) -> Pin<Box<dyn Future<Output = ()> + Send>> {
            Box::pin(std::future::pending())
        }
        fn abort(&self) -> Pin<Box<dyn Future<Output = ()> + Send>> {
            Box::pin(std::future::pending())
        }
    }

    struct NoOpProvider;
    impl Provider for NoOpProvider {
        fn generate(
            &self,
            _: Vec<crate::message::ChatMessage>,
        ) -> Pin<Box<dyn futures::Stream<Item = anyhow::Result<ProviderEvent>> + Send + '_>> {
            Box::pin(futures::stream::empty())
        }
    }

    struct TestProviderFactory;

    #[async_trait]
    impl ProviderFactory for TestProviderFactory {
        fn build(&self, provider: &str, model: &str, _config: &crate::Config) -> Result<BuiltProvider, ProviderError> {
            Ok(BuiltProvider::new(
                Box::new(NoOpProvider),
                provider.into(),
                model.into(),
            ))
        }
        async fn validate_key(&self, _: &str, _: &str) -> anyhow::Result<Vec<String>> {
            Ok(vec![])
        }
        fn resolve_credentials(&self, _: &str, _: &crate::Config) -> (String, String) {
            ("http://localhost".into(), "sk-test".into())
        }
    }

    let bus = EventBus::<CoreEvent>::new(16);
    let (config_h, config_cell, config_join) = RactorConfigActor::spawn_default(bus.clone()).await.unwrap();
    let factory: Arc<dyn ProviderFactory> = Arc::new(TestProviderFactory);
    let (provider_h, provider_cell, provider_join) = RactorProviderActor::spawn(bus.clone(), config_h.clone(), factory)
        .await
        .expect("provider spawn");
    let (io_h, io_cell, io_join) = spawn_io_actor(bus.clone());
    let (session_h, session_cell, session_join) = spawn_session_actor(bus.clone());
    let (permission_h, permission_cell, permission_join) = RactorPermissionActor::spawn(bus.clone(), config_h.clone())
        .await
        .unwrap();
    let (turn_h, turn_cell, turn_join) = RactorTurnActor::spawn(bus.clone()).await.unwrap();
    let (input_h, input_cell, input_join) = spawn_input_actor(bus.clone());

    let (cmd_tx, _cmd_rx) = mpsc::channel(4);
    let agent: Arc<dyn LeaderAgentHandle> = Arc::new(NoOpAgentHandle);
    // Spawn a dummy task as the agent join handle (agent is not a real ractor actor
    // in tests; the NoOpAgentHandle::run returns pending which never completes).
    let agent_join: tokio::task::JoinHandle<()> = tokio::spawn(std::future::pending::<()>());
    let all_joins = vec![
        config_join,
        provider_join,
        io_join,
        session_join,
        permission_join,
        turn_join,
        input_join,
        agent_join,
    ];

    // Dummy coordinator join for tests.
    let coordinator_join = tokio::spawn(std::future::pending::<()>());

    LeaderHandle::new(
        cmd_tx,
        bus,
        SpawnedHandles {
            config: config_h,
            provider: provider_h,
            io: io_h,
            session: session_h,
            permission: permission_h,
            turn: turn_h,
            input: input_h,
            agent,
            agent_cell: None,
            config_cell: config_cell.into(),
            provider_cell: provider_cell.into(),
            io_cell: io_cell.into(),
            session_cell: session_cell.into(),
            permission_cell: permission_cell.into(),
            turn_cell: turn_cell.into(),
            input_cell: input_cell.into(),
            all_joins,
        },
        Some(coordinator_join),
        None,
    )
}
