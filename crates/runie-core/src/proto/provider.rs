//! Minimal provider configuration trait.
//!
//! This trait provides credential resolution for LLM providers without requiring
//! either crate to depend on the other's configuration types.

use std::collections::HashMap;
use std::fmt;

use secrecy::SecretString;

/// Resolves provider credentials from configuration sources.
///
/// Priority order:
/// 1. Environment variables
/// 2. .env file in current working directory
/// 3. Config file entries
pub trait ProviderConfig: Send + Sync + fmt::Debug {
    /// Resolve the API key for a provider.
    ///
    /// Returns a `SecretString` to prevent accidental exposure. Use `.expose_secret()`
    /// only at the HTTP boundary where the key is actually needed.
    fn resolve_api_key(&self, provider: &str) -> Option<SecretString>;

    /// Resolve the base URL for a provider.
    fn resolve_base_url(&self, provider: &str) -> Option<String>;

    /// Resolve the retry policy for provider requests.
    ///
    /// Returns `None` when the implementation has no retry configuration;
    /// callers should use a default policy in that case.
    fn retry_config(&self) -> Option<crate::provider::RetryConfig> {
        None
    }

    /// Resolve custom HTTP headers for a provider from config.
    fn resolve_headers(&self, _provider: &str) -> Option<HashMap<String, String>> {
        None
    }
}
