//! Login API-key validation effect handler.

use runie_core::actors::Reply;
use runie_core::actors::ProviderMsg;
use runie_core::Event as CoreEvent;
use tokio::sync::mpsc;

/// Validate the API key asynchronously and emit the result.
pub fn run(
    provider: String,
    key: String,
    tx: mpsc::Sender<CoreEvent>,
    provider_tx: mpsc::Sender<ProviderMsg>,
) {
    if provider.is_empty() || key.is_empty() {
        return;
    }

    tokio::spawn(async move {
        let result = validate_provider_key(provider_tx, &provider, &key).await;

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

async fn validate_provider_key(
    provider_tx: mpsc::Sender<ProviderMsg>,
    provider: &str,
    key: &str,
) -> anyhow::Result<Vec<String>> {
    let (reply_tx, reply_rx) = tokio::sync::oneshot::channel();
    provider_tx
        .send(ProviderMsg::ValidateKey {
            provider: provider.into(),
            api_key: key.into(),
            reply: Reply::new(reply_tx),
        })
        .await
        .map_err(|_| anyhow::anyhow!("provider actor unavailable"))?;
    reply_rx
        .await
        .map_err(|_| anyhow::anyhow!("provider actor dropped"))?
}

