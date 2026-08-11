use super::*;

impl App {
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
