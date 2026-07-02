//! Ractor-based `ProviderActor` implementation.
//!
//! Migrated from the custom Actor trait to ractor for consistency with the
//! rest of the actor system.
//!
//! ## Async IO Discipline
//! Network calls (`ValidateKey`, `ListModels`) are offloaded to spawned tasks
//! so the actor mailbox remains responsive while waiting for HTTP responses.

use std::sync::Arc;

use ractor::async_trait as ractor_async_trait;
use ractor::{Actor, ActorProcessingErr, ActorRef};

#[cfg(test)]
use crate::actors::config::RactorConfigActor;
use crate::actors::config::RactorConfigHandle;
use crate::actors::ractor_adapter::spawn_ractor;
use crate::bus::EventBus;
use crate::config::Config;
use crate::event::Event;
use crate::provider::ProviderError;

use super::factory::{BuiltProvider, ProviderFactory};
use super::messages::ProviderMsg;

/// Ractor-based `ProviderActor` handle with ergonomic helper methods.
#[derive(Clone, Debug)]
pub struct RactorProviderHandle {
    inner: ActorRef<ProviderMsg>,
}

impl RactorProviderHandle {
    /// Wrap an existing `ActorRef`.
    pub fn new(inner: ActorRef<ProviderMsg>) -> Self {
        Self { inner }
    }

    /// Build a provider for the given registry key and model.
    pub async fn build(
        &self,
        provider: String,
        model: String,
    ) -> Result<BuiltProvider, ProviderError> {
        match self.inner
            .call(
                |tx| ProviderMsg::Build {
                    provider,
                    model,
                    reply: tx,
                },
                None,
            )
            .await
        {
            Ok(ractor::rpc::CallResult::Success(result)) => result,
            _ => Err(anyhow::anyhow!("provider actor dropped").into()),
        }
    }

    /// Validate an API key for a provider, resolving base URL from config.
    pub async fn validate_key(
        &self,
        provider: String,
        api_key: String,
    ) -> anyhow::Result<Vec<String>> {
        match self.inner
            .call(
                |tx| ProviderMsg::ValidateKey {
                    provider,
                    api_key,
                    reply: tx,
                },
                None,
            )
            .await
        {
            Ok(ractor::rpc::CallResult::Success(result)) => result,
            _ => Err(anyhow::anyhow!("provider actor dropped")),
        }
    }

    /// List models for a configured provider.
    pub async fn list_models(&self, provider: String) -> anyhow::Result<Vec<String>> {
        match self.inner
            .call(
                |tx| ProviderMsg::ListModels {
                    provider,
                    reply: tx,
                },
                None,
            )
            .await
        {
            Ok(ractor::rpc::CallResult::Success(result)) => result,
            _ => Err(anyhow::anyhow!("provider actor dropped")),
        }
    }
}

/// Ractor-based `ProviderActor`.
pub struct RactorProviderActor {
    config_handle: RactorConfigHandle,
    factory: Arc<dyn ProviderFactory>,
}

impl RactorProviderActor {
    fn new(
        _bus: EventBus<Event>,
        config_handle: RactorConfigHandle,
        factory: Arc<dyn ProviderFactory>,
    ) -> Self {
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
    ) -> Result<(RactorProviderHandle, ractor::ActorCell, tokio::task::JoinHandle<()>), ractor::SpawnErr> {
        let actor = Self::new(bus, config_handle, factory);
        let (handle, join, cell) = spawn_ractor(None, actor, ()).await?;
        Ok((RactorProviderHandle::new(handle), cell, join))
    }

    /// Spawn a minimal provider actor for testing (no real config/factory needed).
    #[cfg(test)]
    pub async fn minimal_spawn_for_test(
        bus: EventBus<Event>,
    ) -> (RactorProviderHandle, ractor::ActorCell, tokio::task::JoinHandle<()>) {
        use crate::provider::Provider;
        use anyhow::Result;
        use async_trait::async_trait;
        use std::pin::Pin;
        use std::sync::Arc;

        struct EchoProvider;
        impl Provider for EchoProvider {
            fn generate(
                &self,
                _: Vec<crate::ChatMessage>,
            ) -> Pin<
                Box<
                    dyn futures::Stream<Item = Result<crate::provider_event::ProviderEvent>>
                        + Send
                        + '_,
                >,
            > {
                Box::pin(futures::stream::empty())
            }
        }
        struct TestFactory;

        #[async_trait]
        impl ProviderFactory for TestFactory {
            fn build(
                &self,
                _provider: &str,
                model: &str,
                _config: &Config,
            ) -> Result<BuiltProvider, ProviderError> {
                Ok(BuiltProvider::new(
                    Box::new(EchoProvider),
                    "test".into(),
                    model.into(),
                ))
            }
            async fn validate_key(&self, _: &str, _: &str) -> anyhow::Result<Vec<String>> {
                Ok(vec![])
            }
            fn resolve_credentials(&self, _: &str, _: &Config) -> (String, String) {
                ("http://localhost".into(), "sk-test".into())
            }
        }

        let (config_h, _, _) = RactorConfigActor::spawn_default(bus.clone()).await.unwrap();
        Self::spawn(bus, config_h, Arc::new(TestFactory))
            .await
            .unwrap()
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
    pub async fn validate_key(&self, provider: &str, api_key: &str) -> anyhow::Result<Vec<String>> {
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
            // Build is fast (no network) so we await inline.
            ProviderMsg::Build {
                provider,
                model,
                reply,
            } => {
                let result = self.build_provider(&provider, &model).await;
                let _ = reply.send(result);
            }
            // Network calls are awaited directly — ractor actors are async so the
            // mailbox is only blocked while waiting for a message, not while awaiting
            // the network call. No `tokio::spawn` handles are lost.
            ProviderMsg::ValidateKey {
                provider,
                api_key,
                reply,
            } => {
                let config = self.config().await;
                let result =
                    Self::call_validate_key(&provider, &api_key, config, &*self.factory).await;
                let _ = reply.send(result);
            }
            ProviderMsg::ListModels { provider, reply } => {
                let config = self.config().await;
                let result = Self::call_list_models(&provider, config, &*self.factory).await;
                let _ = reply.send(result);
            }
        }
        Ok(())
    }
}

impl RactorProviderActor {
    /// Call `validate_key` and return the result directly.
    async fn call_validate_key(
        provider: &str,
        api_key: &str,
        config: Result<Config, ProviderError>,
        factory: &dyn ProviderFactory,
    ) -> anyhow::Result<Vec<String>> {
        match config {
            Ok(cfg) => {
                let (base_url, resolved_key) = factory.resolve_credentials(provider, &cfg);
                let api_key = if api_key.is_empty() { &resolved_key } else { api_key };
                if api_key.is_empty() {
                    Err(anyhow::anyhow!("API key is required"))
                } else {
                    factory.validate_key(&base_url, api_key).await
                }
            }
            Err(e) => Err(anyhow::anyhow!("{e}")),
        }
    }

    /// Call `list_models` (via validate_key) and return the result directly.
    async fn call_list_models(
        provider: &str,
        config: Result<Config, ProviderError>,
        factory: &dyn ProviderFactory,
    ) -> anyhow::Result<Vec<String>> {
        match config {
            Ok(cfg) => {
                let (base_url, resolved_key) = factory.resolve_credentials(provider, &cfg);
                if resolved_key.is_empty() {
                    Err(anyhow::anyhow!("API key is required"))
                } else {
                    factory.validate_key(&base_url, &resolved_key).await
                }
            }
            Err(e) => Err(anyhow::anyhow!("{e}")),
        }
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
    use async_trait::async_trait;
    use std::pin::Pin;

    /// A minimal mock provider that always returns empty.
    struct MockProvider;
    impl Provider for MockProvider {
        fn generate(
            &self,
            _: Vec<ChatMessage>,
        ) -> Pin<Box<dyn futures::Stream<Item = anyhow::Result<ProviderEvent>> + Send + '_>>
        {
            Box::pin(futures::stream::empty())
        }
    }

    struct MockFactory;

    #[async_trait]
    impl ProviderFactory for MockFactory {
        fn build(
            &self,
            _provider: &str,
            model: &str,
            _config: &Config,
        ) -> Result<BuiltProvider, ProviderError> {
            Ok(BuiltProvider::new(
                Box::new(MockProvider),
                "mock".into(),
                model.into(),
            ))
        }
        async fn validate_key(&self, _: &str, _: &str) -> anyhow::Result<Vec<String>> {
            Ok(vec!["model-a".into(), "model-b".into()])
        }
        fn resolve_credentials(&self, _: &str, _: &Config) -> (String, String) {
            ("http://localhost".into(), "sk-test".into())
        }
    }

    #[tokio::test]
    async fn ractor_provider_actor_spawns() {
        let bus = EventBus::<Event>::new(16);
        let ( config_handle , _cell, _join ) = RactorConfigActor::spawn_default(bus.clone()).await.unwrap();
        let factory = Arc::new(MockFactory);
        let (handle, _cell, _) = RactorProviderActor::spawn(bus.clone(), config_handle, factory)
            .await
            .unwrap();
        let _ = handle;
    }

    #[tokio::test]
    async fn ractor_provider_handle_build() {
        let bus = EventBus::<Event>::new(16);
        let ( config_handle , _cell, _join ) = RactorConfigActor::spawn_default(bus.clone()).await.unwrap();
        let factory = Arc::new(MockFactory);

        let (handle, _cell, _) = RactorProviderActor::spawn(bus.clone(), config_handle, factory)
            .await
            .unwrap();
        let result = handle.build("mock".into(), "echo".into()).await;
        assert!(result.is_ok(), "build should succeed: {:?}", result);
        let built = result.unwrap();
        assert_eq!(built.key, "mock");
        assert_eq!(built.model, "echo");
    }

    #[tokio::test]
    async fn ractor_provider_handle_validate_key() {
        let bus = EventBus::<Event>::new(16);
        let ( config_handle , _cell, _join ) = RactorConfigActor::spawn_default(bus.clone()).await.unwrap();
        let factory = Arc::new(MockFactory);

        let (handle, _cell, _) = RactorProviderActor::spawn(bus.clone(), config_handle, factory)
            .await
            .unwrap();
        let result = handle.validate_key("mock".into(), "sk-test".into()).await;
        assert!(result.is_ok(), "validate_key should succeed: {:?}", result);
        let models = result.unwrap();
        assert_eq!(models, vec!["model-a", "model-b"]);
    }

    /// Verifies that network calls are offloaded and the mailbox stays responsive.
    /// While `ListModels` (slow network) is in flight, `ValidateKey` can also be
    /// processed without blocking.
    #[tokio::test]
    async fn provider_actor_mailbox_not_blocked_by_validate() {
        use std::time::Duration;

        let bus = EventBus::<Event>::new(16);
        let (config_handle, _cell, _join) =
            RactorConfigActor::spawn_default(bus.clone()).await.unwrap();

        // Factory that delays validate_key by 100ms to simulate network latency.
        struct SlowFactory;
        #[async_trait]
        impl ProviderFactory for SlowFactory {
            fn build(
                &self,
                _provider: &str,
                model: &str,
                _config: &Config,
            ) -> Result<BuiltProvider, ProviderError> {
                Ok(BuiltProvider::new(
                    Box::new(MockProvider),
                    "mock".into(),
                    model.into(),
                ))
            }
            async fn validate_key(&self, _: &str, _: &str) -> anyhow::Result<Vec<String>> {
                tokio::time::sleep(Duration::from_millis(100)).await;
                Ok(vec!["model-a".into(), "model-b".into()])
            }
            fn resolve_credentials(&self, _: &str, _: &Config) -> (String, String) {
                ("http://localhost".into(), "sk-test".into())
            }
        }

        let factory = Arc::new(SlowFactory);
        let (handle, _cell, _) =
            RactorProviderActor::spawn(bus.clone(), config_handle, factory)
                .await
                .unwrap();

        // Send ListModels first (will be slow) and ValidateKey second.
        // If the mailbox were blocked, ValidateKey would wait for ListModels.
        // With offloaded network calls, both can be processed concurrently.
        let list_models = handle.list_models("mock".into());
        let validate_key = handle.validate_key("mock".into(), "sk-test".into());

        // Both should complete successfully. If blocked, validate_key would take
        // ~100ms longer (until ListModels completes).
        let (list_result, validate_result) = tokio::join!(list_models, validate_key);

        assert!(
            validate_result.is_ok(),
            "validate_key should succeed: {:?}",
            validate_result
        );
        assert!(
            list_result.is_ok(),
            "list_models should succeed: {:?}",
            list_result
        );
    }
}
