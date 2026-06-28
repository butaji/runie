//! Typed messages and handle for `ProviderActor`.

use std::fmt;
use std::sync::Arc;

use tokio::sync::mpsc;
use tokio::sync::oneshot;

use crate::provider::ProviderError;

use super::factory::BuiltProvider;

/// Arc-wrapped reply sender for `Clone` compatibility.
type Reply<T> = Arc<std::sync::Mutex<Option<oneshot::Sender<T>>>>;

pub(crate) fn make_reply<T>(tx: oneshot::Sender<T>) -> Reply<T> {
    Arc::new(std::sync::Mutex::new(Some(tx)))
}

pub(crate) fn take_reply<T>(r: &Reply<T>) -> Option<oneshot::Sender<T>> {
    r.lock().unwrap_or_else(|e| e.into_inner()).take()
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
///
/// Supports two backends:
/// - `ractor::ActorRef` (ractor-based actors — the production path)
/// - `mpsc::Sender` (legacy custom-trait actors — kept for test compatibility)
#[derive(Clone, Debug)]
pub struct ProviderActorHandle {
    /// Ractor-based backend (preferred).
    actor_ref: Option<ractor::ActorRef<ProviderMsg>>,
    /// Legacy mpsc sender (for custom-trait actors in tests).
    legacy_tx: Option<mpsc::Sender<ProviderMsg>>,
}

impl ProviderActorHandle {
    /// Construct from a ractor `ActorRef` (ractor-based production actors).
    pub fn from_actor_ref(actor_ref: ractor::ActorRef<ProviderMsg>) -> Self {
        Self { actor_ref: Some(actor_ref), legacy_tx: None }
    }

    /// Construct from an mpsc sender (legacy custom-trait actors).
    pub fn from_legacy_tx(tx: mpsc::Sender<ProviderMsg>) -> Self {
        Self { actor_ref: None, legacy_tx: Some(tx) }
    }

    /// Access the underlying actor ref (low-level, ractor path).
    pub fn actor_ref(&self) -> Option<&ractor::ActorRef<ProviderMsg>> {
        self.actor_ref.as_ref()
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
        if let Some(ref ar) = self.actor_ref {
            let _ = ar.send_message(msg);
        } else if let Some(ref tx) = self.legacy_tx {
            let _ = tx.send(msg).await;
        }
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
        if let Some(ref ar) = self.actor_ref {
            let _ = ar.send_message(msg);
        } else if let Some(ref tx) = self.legacy_tx {
            let _ = tx.send(msg).await;
        }
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
        if let Some(ref ar) = self.actor_ref {
            let _ = ar.send_message(msg);
        } else if let Some(ref tx) = self.legacy_tx {
            let _ = tx.send(msg).await;
        }
        reply_rx
            .await
            .unwrap_or_else(|_| Err(anyhow::anyhow!("provider actor dropped")))
    }
}
