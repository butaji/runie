//! Typed messages and handle for `ProviderActor`.

use tokio::sync::{mpsc, oneshot};

use crate::provider::ProviderError;

use super::factory::BuiltProvider;

/// Cloneable reply wrapper around a `oneshot::Sender`.
///
/// Unlike [`crate::actors::config::ConfigReply`], this does not require the
/// reply payload to be `Clone` so it can carry trait objects and `anyhow::Error`.
#[derive(Debug)]
pub struct ProviderReply<T>(std::sync::Arc<std::sync::Mutex<Option<oneshot::Sender<T>>>>);

impl<T> ProviderReply<T> {
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

impl<T> Clone for ProviderReply<T> {
    fn clone(&self) -> Self {
        Self(self.0.clone())
    }
}

/// Messages accepted by `ProviderActor`.
#[derive(Clone, Debug)]
pub enum ProviderMsg {
    /// Build a provider for the given registry key and model.
    Build {
        provider: String,
        model: String,
        reply: ProviderReply<Result<BuiltProvider, ProviderError>>,
    },
    /// Validate an API key for a provider, resolving the base URL from config.
    ValidateKey {
        provider: String,
        api_key: String,
        reply: ProviderReply<anyhow::Result<Vec<String>>>,
    },
    /// List models for a configured provider, resolving credentials from config.
    ListModels {
        provider: String,
        reply: ProviderReply<anyhow::Result<Vec<String>>>,
    },
}

/// Ergonomic handle for sending messages to a `ProviderActor`.
#[derive(Clone, Debug)]
pub struct ProviderActorHandle {
    tx: mpsc::Sender<ProviderMsg>,
}

impl ProviderActorHandle {
    /// Wrap an existing sender.
    pub fn new(tx: mpsc::Sender<ProviderMsg>) -> Self {
        Self { tx }
    }

    /// Access the underlying sender.
    pub fn tx(&self) -> &mpsc::Sender<ProviderMsg> {
        &self.tx
    }

    /// Request a provider build.
    pub async fn build(
        &self,
        provider: String,
        model: String,
    ) -> Result<BuiltProvider, ProviderError> {
        let (reply_tx, reply_rx) = oneshot::channel();
        let _ = self
            .tx
            .send(ProviderMsg::Build {
                provider,
                model,
                reply: ProviderReply::new(reply_tx),
            })
            .await;
        reply_rx
            .await
            .unwrap_or_else(|_| Err(ProviderError::Other("provider actor dropped".into())))
    }

    /// Request API-key validation for a provider.
    pub async fn validate_key(
        &self,
        provider: String,
        api_key: String,
    ) -> anyhow::Result<Vec<String>> {
        let (reply_tx, reply_rx) = oneshot::channel();
        let _ = self
            .tx
            .send(ProviderMsg::ValidateKey {
                provider,
                api_key,
                reply: ProviderReply::new(reply_tx),
            })
            .await;
        reply_rx
            .await
            .unwrap_or_else(|_| Err(anyhow::anyhow!("provider actor dropped")))
    }

    /// Request model listing for a configured provider.
    pub async fn list_models(&self, provider: String) -> anyhow::Result<Vec<String>> {
        let (reply_tx, reply_rx) = oneshot::channel();
        let _ = self
            .tx
            .send(ProviderMsg::ListModels {
                provider,
                reply: ProviderReply::new(reply_tx),
            })
            .await;
        reply_rx
            .await
            .unwrap_or_else(|_| Err(anyhow::anyhow!("provider actor dropped")))
    }
}
