//! Login API-key validation effect handler.

use runie_core::event::LoginFlowEvent;
use runie_core::Event as CoreEvent;
use tokio::sync::mpsc;

/// Validate the API key asynchronously and emit the result.
pub fn run(provider: String, key: String, tx: mpsc::Sender<CoreEvent>) {
    if provider.is_empty() || key.is_empty() {
        return;
    }

    tokio::spawn(async move {
        use runie_core::provider_registry::find_provider;
        use runie_provider::validate_api_key;

        let result = if let Some(meta) = find_provider(&provider) {
            validate_api_key(meta.base_url, &key).await
        } else {
            Err(anyhow::anyhow!("Unknown provider: {}", provider))
        };

        match result {
            Ok(models) => {
                let _ = tx
                    .send(CoreEvent::LoginFlow(LoginFlowEvent::ModelsFetched {
                        provider,
                        key,
                        models,
                    }))
                    .await;
            }
            Err(e) => {
                let _ = tx
                    .send(CoreEvent::LoginFlow(LoginFlowEvent::ValidationFailed {
                        provider,
                        key,
                        error: e.to_string(),
                    }))
                    .await;
            }
        }
    });
}
