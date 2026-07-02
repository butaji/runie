//! ProviderConfig implementation for `crate::proto::provider::ProviderConfig`.

impl crate::proto::provider::ProviderConfig for crate::config::Config {
    fn resolve_api_key(&self, provider: &str) -> Option<String> {
        // API keys are resolved from environment variables or OS keyring,
        // not from the config file.
        let mut resolver = crate::auth::CredentialResolver::new();
        for (name, p) in &self.model_providers {
            // Only pass base_url; api_key comes from env/keyring
            resolver.set_config(name, None, non_empty(&p.base_url));
        }
        resolver.resolve_api_key(provider)
    }

    fn resolve_base_url(&self, provider: &str) -> Option<String> {
        let mut resolver = crate::auth::CredentialResolver::new();
        for (name, p) in &self.model_providers {
            resolver.set_config(name, None, non_empty(&p.base_url));
        }
        resolver.resolve_base_url(provider)
    }
}

fn non_empty(s: &str) -> Option<String> {
    if s.is_empty() {
        None
    } else {
        Some(s.to_owned())
    }
}
