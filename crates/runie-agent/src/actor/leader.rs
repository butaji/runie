//! Leader integration for AgentActor.

use std::pin::Pin;

use runie_core::actors::permission::RactorPermissionHandle;
use runie_core::actors::provider::RactorProviderHandle;

use super::{spawn_ractor_agent, AgentMsg, RactorAgentHandle};

/// Handle that implements `LeaderAgentHandle` for use by the leader.
pub struct LeaderAgentHandleImpl {
    inner: RactorAgentHandle,
}

impl LeaderAgentHandleImpl {
    pub fn new(inner: RactorAgentHandle) -> Self {
        Self { inner }
    }
}

impl runie_core::actors::leader::LeaderAgentHandle for LeaderAgentHandleImpl {
    fn run(
        &self,
        cmd: runie_core::actors::leader::LeaderAgentCmd,
    ) -> Pin<Box<dyn std::future::Future<Output = ()> + Send>> {
        let inner = self.inner.clone();
        let msg = AgentMsg::RunLeader { command: cmd };
        Box::pin(async move {
            let _ = inner.send_message(msg);
        })
    }

    fn abort(&self) -> Pin<Box<dyn std::future::Future<Output = ()> + Send>> {
        let inner = self.inner.clone();
        Box::pin(async move {
            let _ = inner.send_message(AgentMsg::Abort);
        })
    }
}

/// Factory for spawning agent actors (implements `AgentActorFactory`).
#[derive(Default)]
pub struct AgentActorFactoryImpl;

impl AgentActorFactoryImpl {
    pub fn new() -> Self {
        Self
    }
}

impl runie_core::actors::leader::AgentActorFactory for AgentActorFactoryImpl {
    type SpawnFuture = std::pin::Pin<
        Box<
            dyn std::future::Future<
                    Output = Result<
                        std::sync::Arc<dyn runie_core::actors::leader::LeaderAgentHandle>,
                        ractor::SpawnErr,
                    >,
                > + Send,
        >,
    >;

    fn spawn(
        &self,
        bus: runie_core::bus::EventBus<runie_core::event::Event>,
        provider_handle: RactorProviderHandle,
        permission_handle: RactorPermissionHandle,
    ) -> Self::SpawnFuture {
        Box::pin(async move {
            let (handle, _, _cell) = spawn_ractor_agent(bus, provider_handle, permission_handle).await?;
            Ok(std::sync::Arc::new(LeaderAgentHandleImpl::new(handle))
                as std::sync::Arc<
                    dyn runie_core::actors::leader::LeaderAgentHandle,
                >)
        })
    }

    fn spawn_with_join(
        &self,
        bus: runie_core::bus::EventBus<runie_core::event::Event>,
        provider_handle: RactorProviderHandle,
        permission_handle: RactorPermissionHandle,
    ) -> std::pin::Pin<
        Box<
            dyn std::future::Future<Output = Result<runie_core::actors::leader::SpawnedAgent, ractor::SpawnErr>> + Send,
        >,
    > {
        Box::pin(async move {
            let (handle, join, cell) = spawn_ractor_agent(bus, provider_handle, permission_handle).await?;
            Ok(runie_core::actors::leader::SpawnedAgent {
                handle: std::sync::Arc::new(LeaderAgentHandleImpl::new(handle))
                    as std::sync::Arc<dyn runie_core::actors::leader::LeaderAgentHandle>,
                join,
                cell,
            })
        })
    }
}
