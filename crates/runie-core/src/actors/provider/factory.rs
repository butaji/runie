//! Abstract factory used by `ProviderActor` to build and validate providers.

use std::future::Future;
use std::pin::Pin;
use std::sync::{Arc, OnceLock};
use std::collections::HashMap;
use parking_lot::Mutex;

use crate::config::Config;
use crate::message::ChatMessage;
use crate::model_catalog::ModelInfo;
use crate::provider_event::ProviderEvent;

// `Provider` and `ProviderError` are defined in `runie-provider` and re-exported here.
use crate::provider::{Provider, ProviderError, ProviderMetadata, RetryConfig};

/// Process-global cache of HTTP clients keyed by `(provider_key, base_url)`.
///
/// This enables connection reuse across turns: each unique provider+URL pair shares
/// one `reqwest::Client`, so TCP connections and HTTP/2 streams are pooled.
static HTTP_CLIENT_CACHE: OnceLock<Mutex<HashMap<(String, String), Arc<reqwest::Client>>>> =
    OnceLock::new();

fn get_cached_http_client(provider_key: &str, base_url: &str) -> Arc<reqwest::Client> {
    let cache = HTTP_CLIENT_CACHE.get_or_init(|| Mutex::new(HashMap::new()));
    let key = (provider_key.to_owned(), base_url.to_owned());
    let mut guard = cache.lock();
    guard
        .entry(key)
        .or_insert_with(|| {
            let client = reqwest::Client::builder()
                .timeout(std::time::Duration::from_secs(120))
                .connect_timeout(std::time::Duration::from_secs(10))
                .build()
                .unwrap_or_else(|_| reqwest::Client::new());
            Arc::new(client)
        })
        .clone()
}

/// Built provider that wraps a concrete provider implementation.
///
/// This type implements `Provider` directly, so it can be used anywhere a
/// `Box<dyn Provider>` is expected without an extra layer of indirection.
///
/// Internally, HTTP clients are cached per provider+URL pair so that TCP
/// connections are reused across turns.
#[derive(Clone)]
pub struct BuiltProvider {
    /// The constructed provider implementation.
    provider: Arc<dyn Provider>,
    /// Registry key used to build the provider (e.g. "openai", "mock").
    pub key: String,
    /// Model name (e.g. "gpt-4o", "echo").
    pub model: String,
    /// Metadata about this provider's capabilities.
    metadata: ProviderMetadata,
}

impl BuiltProvider {
    /// Create a new `BuiltProvider` from a boxed provider.
    pub fn new(provider: Box<dyn Provider>, key: String, model: String) -> Self {
        Self {
            provider: Arc::from(provider),
            key,
            model,
            metadata: ProviderMetadata::default(),
        }
    }

    /// Create a new `BuiltProvider` with metadata.
    pub fn with_metadata(
        provider: Box<dyn Provider>,
        key: String,
        model: String,
        metadata: ProviderMetadata,
    ) -> Self {
        Self {
            provider: Arc::from(provider),
            key,
            model,
            metadata,
        }
    }

    /// Wrap a provider implementation.
    #[doc(hidden)]
    pub fn from_provider(provider: Box<dyn Provider>, key: &str, model: &str) -> Self {
        Self {
            provider: Arc::from(provider),
            key: key.to_owned(),
            model: model.to_owned(),
            metadata: ProviderMetadata::default(),
        }
    }

    /// Get a cached HTTP client for a provider+URL pair.
    ///
    /// This is the primary mechanism for HTTP connection reuse across turns.
    /// Call this instead of constructing `reqwest::Client` directly.
    pub fn cached_http_client(provider_key: &str, base_url: &str) -> Arc<reqwest::Client> {
        get_cached_http_client(provider_key, base_url)
    }

    /// Returns the registry key used to build this provider.
    pub fn key(&self) -> &str {
        &self.key
    }

    /// Returns the model name.
    pub fn model(&self) -> &str {
        &self.model
    }

    /// Returns the metadata for this provider.
    pub fn metadata(&self) -> &ProviderMetadata {
        &self.metadata
    }

    /// Set the model info in metadata.
    pub fn with_model_info(mut self, info: ModelInfo) -> Self {
        self.metadata = self.metadata.with_model_info(info);
        self
    }

    /// Set a custom retry config.
    pub fn with_retry_config(mut self, config: RetryConfig) -> Self {
        self.metadata.retry_config = config;
        self
    }
}

impl std::fmt::Debug for BuiltProvider {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("BuiltProvider")
            .field("key", &self.key)
            .field("model", &self.model)
            .finish_non_exhaustive()
    }
}

impl Provider for BuiltProvider {
    fn generate(
        &self,
        messages: Vec<ChatMessage>,
    ) -> std::pin::Pin<Box<dyn futures::Stream<Item = anyhow::Result<ProviderEvent>> + Send + '_>>
    {
        self.provider.generate(messages)
    }

    fn generate_with_tools(
        &self,
        messages: Vec<ChatMessage>,
        tools: Vec<serde_json::Value>,
    ) -> std::pin::Pin<Box<dyn futures::Stream<Item = anyhow::Result<ProviderEvent>> + Send + '_>>
    {
        self.provider.generate_with_tools(messages, tools)
    }

    fn metadata(&self) -> ProviderMetadata {
        self.metadata.clone()
    }

    fn complete_fast(
        &self,
        messages: Vec<ChatMessage>,
    ) -> std::pin::Pin<Box<dyn futures::Stream<Item = anyhow::Result<ProviderEvent>> + Send + '_>>
    {
        self.provider.complete_fast(messages)
    }
}

/// Abstract factory for constructing and validating providers.
///
/// Implemented in `runie-provider` so that `runie-core` can avoid a circular
/// dependency on the concrete provider crate. The actor is the sole
/// interactive path that invokes this factory in production.
pub trait ProviderFactory: Send + Sync + 'static {
    /// Build a provider for `provider`/`model` using credentials in `config`.
    fn build(
        &self,
        provider: &str,
        model: &str,
        config: &Config,
    ) -> Result<BuiltProvider, ProviderError>;

    /// Validate `api_key` against `base_url` and return available model IDs.
    fn validate_key(
        &self,
        base_url: &str,
        api_key: &str,
    ) -> Pin<Box<dyn Future<Output = anyhow::Result<Vec<String>>> + Send + '_>>;

    /// Resolve the `(base_url, api_key)` pair for `provider` from `config`.
    fn resolve_credentials(&self, provider: &str, config: &Config) -> (String, String);
}
