//! Typed messages and handle for `ProviderActor`.

use std::fmt;

use crate::provider::ProviderError;
use ractor::RpcReplyPort;

use super::factory::BuiltProvider;

/// Messages accepted by `ProviderActor`.
pub enum ProviderMsg {
    /// Build a provider for the given registry key and model.
    Build {
        provider: String,
        model: String,
        /// Optional reply channel. `Some(port)` for RPC callers; `None` for fire-and-forget.
        reply: Option<RpcReplyPort<Result<BuiltProvider, ProviderError>>>,
    },
    /// Validate an API key for a provider, resolving the base URL from config.
    ValidateKey {
        provider: String,
        api_key: String,
        /// Optional reply channel. `Some(port)` for RPC callers; `None` for fire-and-forget.
        reply: Option<RpcReplyPort<anyhow::Result<Vec<String>>>>,
    },
    /// List models for a configured provider, resolving credentials from config.
    ListModels {
        provider: String,
        /// Optional reply channel. `Some(port)` for RPC callers; `None` for fire-and-forget.
        reply: Option<RpcReplyPort<anyhow::Result<Vec<String>>>>,
    },
}

impl Clone for ProviderMsg {
    fn clone(&self) -> Self {
        match self {
            ProviderMsg::Build { provider, model, reply: _ } => ProviderMsg::Build {
                provider: provider.clone(),
                model: model.clone(),
                reply: None, // Fire-and-forget; original reply not usable after move.
            },
            ProviderMsg::ValidateKey { provider, api_key, reply: _ } => ProviderMsg::ValidateKey {
                provider: provider.clone(),
                api_key: api_key.clone(),
                reply: None, // Fire-and-forget.
            },
            ProviderMsg::ListModels { provider, reply: _ } => ProviderMsg::ListModels {
                provider: provider.clone(),
                reply: None, // Fire-and-forget.
            },
        }
    }
}

impl fmt::Debug for ProviderMsg {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ProviderMsg::Build { provider, model, .. } => f
                .debug_struct("ProviderMsg::Build")
                .field("provider", provider)
                .field("model", model)
                .finish(),
            ProviderMsg::ValidateKey { provider, .. } => f
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
