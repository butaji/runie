//! Typed messages and handle for `ConfigActor`.

use tokio::sync::{mpsc, oneshot};

use crate::config::Config;

/// Messages accepted by `ConfigActor`.
#[derive(Debug, Clone)]
pub enum ConfigMsg {
    /// Load config from disk and publish `Event::ConfigLoaded`.
    Load,
    /// Reload from disk, detect changes, and publish `Event::ConfigLoaded` if changed.
    Reload,
    /// Save or update a provider entry.
    SaveProvider {
        name: String,
        base_url: String,
        api_key: String,
        models: Vec<String>,
    },
    /// Remove a provider entry.
    RemoveProvider { name: String },
    /// Persist the active provider/model as the default.
    SetDefaultModel { provider: String, model: String },
    /// Update the saved model list for a provider.
    SetProviderModels { name: String, models: Vec<String> },
    /// Request the current in-memory config.
    GetConfig(ConfigReply<Config>),
    /// Request the list of configured providers.
    GetConfiguredProviders(ConfigReply<Vec<(String, String, Vec<String>)>>),
}

/// Cloneable reply wrapper around a `oneshot::Sender`.
#[derive(Debug, Clone)]
pub struct ConfigReply<T>(std::sync::Arc<std::sync::Mutex<Option<oneshot::Sender<T>>>>);

impl<T> ConfigReply<T> {
    /// Build a reply handle from a fresh oneshot sender.
    pub fn new(sender: oneshot::Sender<T>) -> Self {
        Self(std::sync::Arc::new(std::sync::Mutex::new(Some(sender))))
    }

    /// Send the reply, consuming the underlying sender.
    pub fn send(self, value: T) {
        if let Some(sender) = self.0.lock().unwrap_or_else(|e| e.into_inner()).take() {
            let _ = sender.send(value);
        }
    }
}

/// Ergonomic handle for sending messages to a `ConfigActor`.
#[derive(Clone, Debug)]
pub struct ConfigActorHandle {
    tx: mpsc::Sender<ConfigMsg>,
}

impl ConfigActorHandle {
    /// Wrap an existing sender.
    pub fn new(tx: mpsc::Sender<ConfigMsg>) -> Self {
        Self { tx }
    }

    /// Access the underlying sender.
    pub fn tx(&self) -> &mpsc::Sender<ConfigMsg> {
        &self.tx
    }

    /// Ask the actor to load config from disk.
    pub async fn load(&self) {
        let _ = self.tx.send(ConfigMsg::Load).await;
    }

    /// Ask the actor to reload config from disk.
    pub async fn reload(&self) {
        let _ = self.tx.send(ConfigMsg::Reload).await;
    }

    /// Save a provider configuration.
    pub async fn save_provider(
        &self,
        name: String,
        base_url: String,
        api_key: String,
        models: Vec<String>,
    ) {
        let _ = self
            .tx
            .send(ConfigMsg::SaveProvider {
                name,
                base_url,
                api_key,
                models,
            })
            .await;
    }

    /// Remove a provider configuration.
    pub async fn remove_provider(&self, name: String) {
        let _ = self.tx.send(ConfigMsg::RemoveProvider { name }).await;
    }

    /// Persist the active provider/model as the default.
    pub async fn set_default_model(&self, provider: String, model: String) {
        let _ = self
            .tx
            .send(ConfigMsg::SetDefaultModel { provider, model })
            .await;
    }

    /// Update the saved model list for a provider.
    pub async fn set_provider_models(&self, name: String, models: Vec<String>) {
        let _ = self
            .tx
            .send(ConfigMsg::SetProviderModels { name, models })
            .await;
    }

    /// Request the current in-memory config.
    pub async fn get_config(&self) -> Option<Config> {
        let (tx, rx) = oneshot::channel();
        self.tx
            .send(ConfigMsg::GetConfig(ConfigReply::new(tx)))
            .await
            .ok()?;
        rx.await.ok()
    }

    /// Request the list of configured providers.
    pub async fn get_configured_providers(&self) -> Option<Vec<(String, String, Vec<String>)>> {
        let (tx, rx) = oneshot::channel();
        self.tx
            .send(ConfigMsg::GetConfiguredProviders(ConfigReply::new(tx)))
            .await
            .ok()?;
        rx.await.ok()
    }
}
