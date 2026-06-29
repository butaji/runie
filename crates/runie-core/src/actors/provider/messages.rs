//! Typed messages and handle for `ProviderActor`.

use std::fmt;
use std::sync::Arc;

use parking_lot::Mutex;

use tokio::sync::oneshot;

use crate::provider::ProviderError;

use super::factory::BuiltProvider;

/// Arc-wrapped reply sender for `Clone` compatibility.
type Reply<T> = Arc<Mutex<Option<oneshot::Sender<T>>>>;

pub(crate) fn make_reply<T>(tx: oneshot::Sender<T>) -> Reply<T> {
    Arc::new(Mutex::new(Some(tx)))
}

pub(crate) fn take_reply<T>(r: &Reply<T>) -> Option<oneshot::Sender<T>> {
    r.lock().take()
}

/// Messages accepted by `ProviderActor`.
#[derive(Clone)]
pub enum ProviderMsg {
    /// Build a provider for the given registry key and model.
    Build {
        provider: String,
        model: String,
        reply: Reply<Result<BuiltProvider, ProviderError>>,
    },
    /// Validate an API key for a provider, resolving the base URL from config.
    ValidateKey {
        provider: String,
        api_key: String,
        reply: Reply<anyhow::Result<Vec<String>>>,
    },
    /// List models for a configured provider, resolving credentials from config.
    ListModels {
        provider: String,
        reply: Reply<anyhow::Result<Vec<String>>>,
    },
}

impl fmt::Debug for ProviderMsg {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ProviderMsg::Build {
                provider, model, ..
            } => f
                .debug_struct("ProviderMsg::Build")
                .field("provider", provider)
                .field("model", model)
                .finish(),
            ProviderMsg::ValidateKey {
                provider,
                api_key: _,
                ..
            } => f
                .debug_struct("ProviderMsg::ValidateKey")
                .field("provider", provider)
                .field("api_key", &"***")
                .finish(),
            ProviderMsg::ListModels { provider, .. } => f
                .debug_struct("ProviderMsg::ListModels")
                .field("provider", provider)
                .finish(),
        }
    }
}

/// Ergonomic handle for sending messages to a `ProviderActor`.
#[derive(Clone, Debug)]
pub struct ProviderActorHandle {
    actor_ref: ractor::ActorRef<ProviderMsg>,
}

impl ProviderActorHandle {
    /// Construct from a ractor `ActorRef`.
    pub fn from_actor_ref(actor_ref: ractor::ActorRef<ProviderMsg>) -> Self {
        Self { actor_ref }
    }

    /// Access the underlying actor ref (low-level).
    pub fn actor_ref(&self) -> &ractor::ActorRef<ProviderMsg> {
        &self.actor_ref
    }

    /// Request a provider build.
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
        let _ = self.actor_ref.send_message(msg);
        reply_rx
            .await
            .unwrap_or_else(|_| Err(anyhow::anyhow!("provider actor dropped").into()))
    }

    /// Request API-key validation for a provider.
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
        let _ = self.actor_ref.send_message(msg);
        reply_rx
            .await
            .unwrap_or_else(|_| Err(anyhow::anyhow!("provider actor dropped")))
    }

    /// Request model listing for a configured provider.
    pub async fn list_models(&self, provider: String) -> anyhow::Result<Vec<String>> {
        let (reply_tx, reply_rx) = tokio::sync::oneshot::channel();
        let msg = ProviderMsg::ListModels {
            provider,
            reply: make_reply(reply_tx),
        };
        let _ = self.actor_ref.send_message(msg);
        reply_rx
            .await
            .unwrap_or_else(|_| Err(anyhow::anyhow!("provider actor dropped")))
    }
}
