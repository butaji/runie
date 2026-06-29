//! Ractor-based `ProviderActor` implementation.
//!
//! Migrated from the custom Actor trait to ractor for consistency with the
//! rest of the actor system.

use std::sync::Arc;

use ractor::{Actor, ActorProcessingErr, ActorRef};
use ractor::async_trait as ractor_async_trait;

use crate::actors::config::RactorConfigHandle;
#[cfg(test)]
use crate::actors::config::RactorConfigActor;
use crate::actors::ractor_adapter::{spawn_ractor, RactorHandle};
use crate::bus::EventBus;
use crate::config::Config;
use crate::event::Event;
use crate::provider::ProviderError;

use super::factory::{BuiltProvider, ProviderFactory};
use super::messages::{make_reply, take_reply, ProviderMsg};

/// Ractor-based `ProviderActor` handle with ergonomic helper methods.
#[derive(Clone, Debug)]
pub struct RactorProviderHandle {
    inner: RactorHandle<ProviderMsg>,
}

impl RactorProviderHandle {
    /// Wrap an existing `RactorHandle`.
    pub fn new(inner: RactorHandle<ProviderMsg>) -> Self {
        Self { inner }
    }

    /// Access the underlying ractor handle.
    pub fn tx(&self) -> &RactorHandle<ProviderMsg> {
        &self.inner
    }

    /// Build a provider for the given registry key and model.
    pub async fn build(
        &self,
        provider: String,
        model: String,
    ) -> Result<BuiltProvider, ProviderError> {
        let (reply_tx, reply_rx) = tokio::sync::oneshot::channel();
        let msg = ProviderMsg::Build {
            provider,
            model,
            reply: make_reply(reply_tx),
        };
        let _ = self.inner.send(msg).await;
        reply_rx
            .await
            .unwrap_or_else(|_| Err(anyhow::anyhow!("provider actor dropped").into()))
    }

    /// Validate an API key for a provider, resolving base URL from config.
    pub async fn validate_key(
        &self,
        provider: String,
        api_key: String,
    ) -> anyhow::Result<Vec<String>> {
        let (reply_tx, reply_rx) = tokio::sync::oneshot::channel();
        let msg = ProviderMsg::ValidateKey {
            provider,
            api_key,
            reply: make_reply(reply_tx),
        };
        let _ = self.inner.send(msg).await;
        reply_rx
            .await
            .unwrap_or_else(|_| Err(anyhow::anyhow!("provider actor dropped")))
    }

    /// List models for a configured provider.
    pub async fn list_models(&self, provider: String) -> anyhow::Result<Vec<String>> {
        let (reply_tx, reply_rx) = tokio::sync::oneshot::channel();
        let msg = ProviderMsg::ListModels {
            provider,
            reply: make_reply(reply_tx),
        };
        let _ = self.inner.send(msg).await;
        reply_rx
            .await
            .unwrap_or_else(|_| Err(anyhow::anyhow!("provider actor dropped")))
    }
}

impl From<RactorProviderHandle> for crate::actors::ProviderActorHandle {
    fn from(h: RactorProviderHandle) -> Self {
        crate::actors::ProviderActorHandle::from_actor_ref(h.inner.actor_ref().clone())
    }
}

/// Ractor-based `ProviderActor`.
pub struct RactorProviderActor {
    config_handle: RactorConfigHandle,
    factory: Arc<dyn ProviderFactory>,
}

impl RactorProviderActor {
    fn new(_bus: EventBus<Event>, config_handle: RactorConfigHandle, factory: Arc<dyn ProviderFactory>) -> Self {
        Self {
            config_handle,
            factory,
        }
    }

    /// Spawn a `RactorProviderActor` on the given event bus.
    pub async fn spawn(
        bus: EventBus<Event>,
        config_handle: RactorConfigHandle,
        factory: Arc<dyn ProviderFactory>,
    ) -> Result<(RactorProviderHandle, ractor::ActorCell), ractor::SpawnErr> {
        let actor = Self::new(bus, config_handle, factory);
        let (handle, _join, cell) = spawn_ractor(None, actor, ()).await?;
        Ok((RactorProviderHandle::new(handle), cell))
    }

    /// Spawn a minimal provider actor for testing (no real config/factory needed).
    #[cfg(test)]
    pub async fn minimal_spawn_for_test(
        bus: EventBus<Event>,
    ) -> (RactorProviderHandle, ractor::ActorCell) {
        use crate::provider::Provider;
        use anyhow::Result;
        use std::pin::Pin;
        use std::sync::Arc;

        struct EchoProvider;
        impl Provider for EchoProvider {
            fn generate(&self, _: Vec<crate::ChatMessage>) -> Pin<Box<dyn futures::Stream<Item = Result<crate::provider_event::ProviderEvent>> + Send + '_>> {
                Box::pin(futures::stream::empty())
            }
        }
        struct TestFactory;
        impl ProviderFactory for TestFactory {
            fn build(&self, _provider: &str, model: &str, _config: &Config) -> Result<BuiltProvider, ProviderError> {
                Ok(BuiltProvider::new(Box::new(EchoProvider), "test".into(), model.into()))
            }
            fn validate_key(&self, _: &str, _: &str) -> Pin<Box<dyn std::future::Future<Output = anyhow::Result<Vec<String>>> + Send + '_>> {
                Box::pin(async { Ok(vec![]) })
            }
            fn resolve_credentials(&self, _: &str, _: &Config) -> (String, String) {
                ("http://localhost".into(), "sk-test".into())
            }
        }

        let (config_h, _) = RactorConfigActor::spawn_default(bus.clone()).await;
        Self::spawn(bus, config_h, Arc::new(TestFactory)).await.unwrap()
    }

    /// Build a provider for the given registry key and model.
    pub async fn build_provider(
        &self,
        provider: &str,
        model: &str,
    ) -> Result<BuiltProvider, ProviderError> {
        let config = self.config().await?;
        self.factory.build(provider, model, &config)
    }

    /// List models for a configured provider.
    pub async fn list_models(&self, provider: &str) -> anyhow::Result<Vec<String>> {
        self.validate_key(provider, "").await
    }

    /// Validate an API key for a provider.
    pub async fn validate_key(
        &self,
        provider: &str,
        api_key: &str,
    ) -> anyhow::Result<Vec<String>> {
        let config = self.config().await.map_err(|e| anyhow::anyhow!("{e}"))?;
        let (base_url, resolved_key) = self.factory.resolve_credentials(provider, &config);
        let api_key = if api_key.is_empty() { &resolved_key } else { api_key };
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

#[ractor_async_trait]
impl Actor for RactorProviderActor {
    type Msg = ProviderMsg;
    type State = ();
    type Arguments = ();

    async fn pre_start(
        &self,
        _myself: ActorRef<Self::Msg>,
        _args: Self::Arguments,
    ) -> Result<Self::State, ActorProcessingErr> {
        Ok(())
    }

    async fn handle(
        &self,
        _myself: ActorRef<Self::Msg>,
        msg: Self::Msg,
        _state: &mut Self::State,
    ) -> Result<(), ActorProcessingErr> {
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
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::actors::RactorConfigActor;
    use crate::bus::EventBus;
    use crate::event::Event;
    use crate::message::ChatMessage;
    use crate::provider::{Provider, ProviderError};
    use crate::provider_event::ProviderEvent;
    use std::pin::Pin;

    /// A minimal mock provider that echoes back the model name.
    struct MockProvider(String);
    impl Provider for MockProvider {
        fn generate(&self, _: Vec<ChatMessage>) -> Pin<Box<dyn futures::Stream<Item = anyhow::Result<ProviderEvent>> + Send + '_>> {
            Box::pin(futures::stream::empty())
        }
    }

    struct MockFactory;
    impl ProviderFactory for MockFactory {
        fn build(&self, _provider: &str, model: &str, _config: &Config) -> Result<BuiltProvider, ProviderError> {
            Ok(BuiltProvider::new(Box::new(MockProvider(model.into())), "mock".into(), model.into()))
        }
        fn validate_key(&self, _: &str, _: &str) -> Pin<Box<dyn std::future::Future<Output = anyhow::Result<Vec<String>>> + Send + '_>> {
            Box::pin(async { Ok(vec!["model-a".into(), "model-b".into()]) })
        }
        fn resolve_credentials(&self, _: &str, _: &Config) -> (String, String) {
            ("http://localhost".into(), "sk-test".into())
        }
    }

    #[tokio::test]
    async fn ractor_provider_actor_spawns() {
        let bus = EventBus::<Event>::new(16);
        let (config_handle, _cell) = RactorConfigActor::spawn_default(bus.clone()).await;
        let factory = Arc::new(MockFactory);
        let (handle, _cell) = RactorProviderActor::spawn(bus.clone(), config_handle, factory).await.unwrap();
        let _ = handle;
    }

    #[tokio::test]
    async fn ractor_provider_handle_build() {
        let bus = EventBus::<Event>::new(16);
        let (config_handle, _cell) = RactorConfigActor::spawn_default(bus.clone()).await;
        let factory = Arc::new(MockFactory);

        let (handle, _cell) = RactorProviderActor::spawn(bus.clone(), config_handle, factory).await.unwrap();
        let result = handle.build("mock".into(), "echo".into()).await;
        assert!(result.is_ok(), "build should succeed: {:?}", result);
        let built = result.unwrap();
        assert_eq!(built.key, "mock");
        assert_eq!(built.model, "echo");
    }

    #[tokio::test]
    async fn ractor_provider_handle_validate_key() {
        let bus = EventBus::<Event>::new(16);
        let (config_handle, _cell) = RactorConfigActor::spawn_default(bus.clone()).await;
        let factory = Arc::new(MockFactory);

        let (handle, _cell) = RactorProviderActor::spawn(bus.clone(), config_handle, factory).await.unwrap();
        let result = handle.validate_key("mock".into(), "sk-test".into()).await;
        assert!(result.is_ok(), "validate_key should succeed: {:?}", result);
        let models = result.unwrap();
        assert_eq!(models, vec!["model-a", "model-b"]);
    }
}
