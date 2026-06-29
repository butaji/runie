//! Minimal provider configuration trait.
//!
//! This trait provides credential resolution for LLM providers without requiring
//! either crate to depend on the other's configuration types.

use std::fmt;
use std::sync::Arc;

/// Resolves provider credentials from configuration sources.
///
/// Priority order:
/// 1. Environment variables
/// 2. .env file in current working directory
/// 3. Config file entries
pub trait ProviderConfig: Send + Sync + fmt::Debug {
    /// Resolve the API key for a provider.
    fn resolve_api_key(&self, provider: &str) -> Option<String>;

    /// Resolve the base URL for a provider.
    fn resolve_base_url(&self, provider: &str) -> Option<String>;
}

/// Wrapper for type-erased ProviderConfig that can be cloned.
///
/// This is used when we need to pass a ProviderConfig through multiple
/// API calls that require Clone.
pub struct ProviderConfigBox {
    inner: Arc<dyn ProviderConfig>,
}

impl ProviderConfigBox {
    /// Create a new box from any ProviderConfig implementation.
    pub fn new(config: impl ProviderConfig + 'static) -> Self {
        Self {
            inner: Arc::new(config),
        }
    }
}

impl std::ops::Deref for ProviderConfigBox {
    type Target = dyn ProviderConfig;
    fn deref(&self) -> &Self::Target {
        &*self.inner
    }
}

impl Clone for ProviderConfigBox {
    fn clone(&self) -> Self {
        Self {
            inner: self.inner.clone(),
        }
    }
}

impl fmt::Debug for ProviderConfigBox {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("ProviderConfigBox").finish()
    }
}
