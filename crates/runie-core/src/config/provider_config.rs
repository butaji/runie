//! ProviderConfig implementation for `crate::proto::provider::ProviderConfig`.

impl crate::proto::provider::ProviderConfig for crate::config::Config {
    fn resolve_api_key(&self, provider: &str) -> Option<String> {
        let env_key = format!("{}_API_KEY", provider.to_uppercase());
        if let Ok(val) = std::env::var(&env_key) {
            if !val.is_empty() {
                return Some(val);
            }
        }
        self.model_providers
            .get(provider)
            .and_then(|p| non_empty(&p.api_key))
    }

    fn resolve_base_url(&self, provider: &str) -> Option<String> {
        let env_key = format!("{}_BASE_URL", provider.to_uppercase());
        if let Ok(val) = std::env::var(&env_key) {
            if !val.is_empty() {
                return Some(val);
            }
        }
        self.model_providers
            .get(provider)
            .and_then(|p| non_empty(&p.base_url))
    }
}

fn non_empty(s: &str) -> Option<String> {
    if s.is_empty() { None } else { Some(s.to_owned()) }
}
