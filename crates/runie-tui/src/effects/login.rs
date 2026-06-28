//! Login API-key validation effect handler.

use runie_core::actors::provider::RactorProviderHandle;
use runie_core::Event as CoreEvent;
use tokio::sync::mpsc;

/// Validate the API key asynchronously and emit the result.
///
/// # Arguments
/// * `provider` - Provider name
/// * `key` - API key
/// * `tx` - Channel to send result events
/// * `provider_handle` - Handle to the provider actor
pub fn run(
    provider: String,
    key: String,
    tx: mpsc::Sender<CoreEvent>,
    provider_handle: RactorProviderHandle,
) -> impl std::future::Future<Output = ()> + Send + 'static {
    async move {
        if provider.is_empty() || key.is_empty() {
            return;
        }

        let result = provider_handle.validate_key(provider.clone(), key.clone()).await;

        match result {
            Ok(models) => {
                let _ = tx
                    .send(CoreEvent::ModelsFetched {
                        provider,
                        key,
                        models,
                    })
                    .await;
            }
            Err(e) => {
                let _ = tx
                    .send(CoreEvent::ValidationFailed {
                        provider,
                        key,
                        error: e.to_string(),
                    })
                    .await;
            }
        }
    }
}
