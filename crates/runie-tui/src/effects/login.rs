//! Login API-key validation effect handler.

use runie_core::Event as CoreEvent;
use tokio::sync::mpsc;

/// Validate the API key asynchronously and emit the result.
pub fn run(provider: String, key: String, tx: mpsc::Sender<CoreEvent>) {
    if provider.is_empty() || key.is_empty() {
        return;
    }

    tokio::spawn(async move {
        let config = runie_core::config::Config::load(None);
        let result = validate_provider(&provider, &key, &config).await;

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
    });
}

async fn validate_provider(
    provider: &str,
    key: &str,
    config: &runie_core::config::Config,
) -> anyhow::Result<Vec<String>> {
    use runie_core::provider_registry::find_provider;
    use runie_provider::validate_api_key;

    let meta = find_provider(provider)
        .ok_or_else(|| anyhow::anyhow!("Unknown provider: {}", provider))?;
    let base_url = validation_base_url(provider, config)
        .unwrap_or_else(|| meta.base_url.to_string());
    validate_api_key(&base_url, key).await
}

fn validation_base_url(
    provider: &str,
    config: &runie_core::config::Config,
) -> Option<String> {
    let resolver = runie_provider::config::ProviderConfigResolver::from_config(config);
    resolver.resolve_base_url(provider)
}

#[cfg(test)]
mod tests;
