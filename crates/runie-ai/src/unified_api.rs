use std::collections::HashMap;
use runie_core::{Message, ToolSchema, Event, ProviderError};
use futures::stream::BoxStream;
use crate::Provider;

pub struct UnifiedApi {
    providers: HashMap<String, Box<dyn Provider>>,
    default_provider: Option<String>,
}

impl UnifiedApi {

    #[must_use]
    #[must_use]
    pub fn new() -> Self {
        Self {
            providers: HashMap::new(),
            default_provider: None,
        }
    }

    pub fn register_provider(&mut self, name: String, provider: Box<dyn Provider>) {
        self.providers.insert(name, provider);
    }

    pub fn set_default(&mut self, name: String) {
        self.default_provider = Some(name);
    }

    pub async fn chat(
        &self,
        provider: Option<String>,
        messages: Vec<Message>,
        tools: Vec<ToolSchema>,
    ) -> Result<BoxStream<'static, Event>, ProviderError> {
        let name = provider.or_else(|| self.default_provider.clone())
            .ok_or_else(|| ProviderError::ApiError("No provider specified".to_string()))?;
        let provider = self.providers.get(&name)
            .ok_or_else(|| ProviderError::ApiError(format!("Provider '{}' not found", name)))?;
        provider.chat(messages, tools).await
    }

    pub fn get_provider(&self, name: &str) -> Option<&dyn Provider> {
        self.providers.get(name).map(|p| p.as_ref())
    }

    pub fn list_providers(&self) -> Vec<String> {
        self.providers.keys().cloned().collect()
    }

}

impl Default for UnifiedApi {
    fn default() -> Self {
        Self::new()
    }
}
