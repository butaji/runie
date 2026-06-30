//! Typed messages and handle for `ProviderActor`.

use std::fmt;

use crate::actors::ractor_adapter::Reply;
use crate::provider::ProviderError;

use super::factory::BuiltProvider;

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

