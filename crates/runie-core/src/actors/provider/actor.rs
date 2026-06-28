//! `ProviderActor` — sole interactive path for building and validating providers.

use std::sync::Arc;

use tokio::sync::mpsc;

use crate::actors::config::{ConfigActorHandle, ConfigHandle, RactorConfigHandle};
use crate::actors::{Actor, ActorHandle};
use crate::bus::EventBus;
use crate::config::Config;
use crate::event::Event;
use crate::provider::ProviderError;

use super::factory::{BuiltProvider, ProviderFactory};
use super::messages::{take_reply, ProviderActorHandle, ProviderMsg};

/// Actor that owns provider construction and API-key validation.
pub struct ProviderActor<H: ConfigHandle> {
    config_handle: H,
    factory: Arc<dyn ProviderFactory>,
}

impl<H: ConfigHandle + 'static> ProviderActor<H> {
    /// Spawn a `ProviderActor` on the given event bus.
    pub fn spawn(
        bus: EventBus<Event>,
        config_handle: H,
        factory: Arc<dyn ProviderFactory>,
    ) -> (ProviderActorHandle, ActorHandle) {
        let (tx, rx) = mpsc::channel(32);
        let actor = Self {
            config_handle,
            factory,
        };
        let handle = ActorHandle::spawn(actor, rx, bus);
        (ProviderActorHandle::from_legacy_tx(tx), handle)
    }
}

/// Spawn with legacy ConfigActorHandle (for tests).
impl ProviderActor<ConfigActorHandle> {
    pub fn spawn_with_legacy_handle(
        bus: EventBus<Event>,
        config_handle: ConfigActorHandle,
        factory: Arc<dyn ProviderFactory>,
    ) -> (ProviderActorHandle, ActorHandle) {
        Self::spawn(bus, config_handle, factory)
    }
}

/// Spawn with RactorConfigHandle (for production).
impl ProviderActor<RactorConfigHandle> {
    pub fn spawn_with_ractor_handle(
        bus: EventBus<Event>,
        config_handle: RactorConfigHandle,
        factory: Arc<dyn ProviderFactory>,
    ) -> (ProviderActorHandle, ActorHandle) {
        Self::spawn(bus, config_handle, factory)
    }
}

impl<H: ConfigHandle + 'static> Actor for ProviderActor<H> {
    type Msg = ProviderMsg;
    type Event = Event;

    async fn run_body(mut self, mut rx: mpsc::Receiver<Self::Msg>, _bus: EventBus<Event>) {
        while let Some(msg) = rx.recv().await {
            self.handle_msg(msg).await;
        }
    }
}

impl<H: ConfigHandle + 'static> ProviderActor<H> {
    async fn handle_msg(&mut self, msg: ProviderMsg) {
        match msg {
            ProviderMsg::Build {
                provider,
                model,
                reply,
            } => {
                let result = self.build_provider(&provider, &model).await;
                if let Some(tx) = take_reply(&reply) {
                    let _ = tx.send(result);
                }
            }
            ProviderMsg::ValidateKey {
                provider,
                api_key,
                reply,
            } => {
                let result = self.validate_key(&provider, &api_key).await;
                if let Some(tx) = take_reply(&reply) {
                    let _ = tx.send(result);
                }
            }
            ProviderMsg::ListModels { provider, reply } => {
                let result = self.list_models(&provider).await;
                if let Some(tx) = take_reply(&reply) {
                    let _ = tx.send(result);
                }
            }
        }
    }

    async fn build_provider(
        &self,
        provider: &str,
        model: &str,
    ) -> Result<BuiltProvider, ProviderError> {
        let config = self.config().await?;
        self.factory.build(provider, model, &config)
    }

    async fn list_models(&self, provider: &str) -> anyhow::Result<Vec<String>> {
        self.validate_key(provider, "").await
    }

    async fn validate_key(&self, provider: &str, api_key: &str) -> anyhow::Result<Vec<String>> {
        let config = self.config().await.map_err(|e| anyhow::anyhow!("{e}"))?;
        let (base_url, resolved_key) = self.factory.resolve_credentials(provider, &config);
        let api_key = if api_key.is_empty() {
            &resolved_key
        } else {
            api_key
        };
        if api_key.is_empty() {
            anyhow::bail!("API key is required");
        }
        self.factory.validate_key(&base_url, api_key).await
    }

    async fn config(&self) -> Result<Config, ProviderError> {
        self.config_handle
            .get_config()
            .await
            .ok_or_else(|| anyhow::anyhow!("config actor unavailable").into())
    }
}
