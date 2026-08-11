use super::*;

fn connected_provider<'a>(
    state: &'a runie_core::provider_registry::ProviderRegistryState,
    provider_id: &str,
) -> Option<&'a runie_core::provider_registry::ProviderConfig> {
    state
        .providers
        .iter()
        .find(|provider| provider.id == provider_id && provider.connected)
}

impl App {
    pub async fn select_provider_model(
        &self,
        provider_id: String,
        model_id: String,
    ) -> Result<(), String> {
        let providers = self.provider_registry.shared_snapshot();
        let provider = connected_provider(providers.get(), &provider_id)
            .ok_or_else(|| format!("provider is not connected: {provider_id}"))?;
        let catalog = self.model_catalog.shared_snapshot();
        let model = catalog
            .get()
            .catalog
            .available
            .iter()
            .find(|model| model.provider == provider_id && model.id == model_id)
            .cloned()
            .unwrap_or_else(|| runie_core::types::Model {
                id: model_id.clone(),
                name: model_id.clone(),
                provider: provider_id.clone(),
                base_url: provider.base_url.clone(),
                api: "openai-completions".into(),
                ..Default::default()
            });
        self.set_model_with_declared_effort(model.clone()).await;
        self.apply_provider_event(runie_core::provider_registry::ProviderEvent::Selected {
            provider_id,
            model: model_id,
        })
        .await?;
        self.prompt
            .set_model_caption(format!("{} · always-approve", model.id))
            .await;
        Ok(())
    }

    /// Project the catalog actor's immutable search universe into the UI actor.
    pub async fn toggle_model_selector(&self) {
        if self
            .model_catalog
            .shared_snapshot()
            .get()
            .catalog
            .available
            .is_empty()
        {
            self.refresh_models().await;
        }
        if self
            .model_catalog
            .shared_snapshot()
            .get()
            .catalog
            .available
            .is_empty()
        {
            self.model_catalog
                .load(vec![runie_core::types::Model {
                    id: std::env::var("MINIMAX_MODEL").unwrap_or_else(|_| "MiniMax-M2.1".into()),
                    name: "MiniMax M2.1".into(),
                    api: "openai-completions".into(),
                    provider: "minimax".into(),
                    base_url: std::env::var("MINIMAX_BASE_URL").unwrap_or_default(),
                    ..Default::default()
                }])
                .await;
        }
        let catalog = self.model_catalog.shared_snapshot();
        self.ui
            .send(UiMsg::SetModelSelectorRows(model_selector_rows(
                catalog.get(),
            )))
            .await;
        self.ui.send(UiMsg::ToggleModelSelector).await;
    }
}
