//! Shared headless runtime for non-interactive binaries.
//!
//! `HeadlessRuntime` owns a small actor system (ConfigActor + ProviderActor) so
//! CLI/server processes never call `Config::load` or build providers directly.
//!
//! ```ignore
//! let rt = HeadlessRuntime::spawn(EventBus::new(10), Arc::new(factory)).await?;
//! let built = rt.provider(Some("openai"), Some("gpt-4o")).await?;
//! run_headless_turn(messages, built.provider.as_ref(), options).await;
//! ```

use std::sync::Arc;
use std::time::Duration;

use tokio::time::timeout;

use crate::actors::{ConfigActorHandle, ProviderActorHandle, ProviderActor, ConfigActor};
use crate::actor::ActorHandle;
use crate::bus::EventBus;
use crate::config::Config;
use crate::actors::provider::{BuiltProvider, ProviderFactory};
use crate::provider::ProviderError;
use crate::event::Event;

/// Non-interactive runtime backed by the same actors as the TUI.
pub struct HeadlessRuntime {
    config_handle: ConfigActorHandle,
    provider_handle: ProviderActorHandle,
    _config_actor: ActorHandle,
    _provider_actor: ActorHandle,
}

impl HeadlessRuntime {
    /// Spawn the runtime and wait for the initial config load.
    pub async fn spawn(
        bus: EventBus<Event>,
        factory: Arc<dyn ProviderFactory>,
    ) -> anyhow::Result<Self> {
        let mut sub = bus.subscribe();
        let (config_handle, config_actor) = ConfigActor::spawn(bus.clone(), None);
        let (provider_handle, provider_actor) =
            ProviderActor::spawn(bus, config_handle.clone(), factory);

        // Wait until the config actor has loaded (or failed to load) so callers
        // can resolve provider/model defaults immediately.
        timeout(Duration::from_secs(2), async {
            loop {
                match sub.recv().await {
                    Ok(Event::ConfigLoaded { .. }) | Ok(Event::Error { .. }) => return Ok::<(), anyhow::Error>(()),
                    Err(_) => return Ok::<(), anyhow::Error>(()),
                    // intentionally ignored: other events loop back
                    _ => {}
                }
            }
        })
        .await
        .map_err(|_| anyhow::anyhow!("timed out waiting for config to load"))??;

        Ok(Self {
            config_handle,
            provider_handle,
            _config_actor: config_actor,
            _provider_actor: provider_actor,
        })
    }

    /// Current config, if loaded.
    pub async fn config(&self) -> Option<Config> {
        self.config_handle.get_config().await
    }

    /// Resolve provider/model from explicit args or config defaults and build it.
    pub async fn provider(
        &self,
        provider: Option<&str>,
        model: Option<&str>,
    ) -> Result<BuiltProvider, ProviderError> {
        let config = self.config().await.ok_or(ProviderError::ConfigNotLoaded)?;
        let provider_name = provider
            .map(String::from)
            .or(config.provider.clone())
            .unwrap_or_else(|| "mock".to_string());
        let model_name = model
            .map(String::from)
            .or_else(|| config.default_model().map(String::from))
            .unwrap_or_else(|| "echo".to_string());
        self.provider_handle.build(provider_name, model_name).await
    }

    /// Validate an API key for a provider.
    pub async fn validate_key(
        &self,
        provider: &str,
        api_key: &str,
    ) -> anyhow::Result<Vec<String>> {
        self.provider_handle.validate_key(provider.into(), api_key.into()).await
    }
}
