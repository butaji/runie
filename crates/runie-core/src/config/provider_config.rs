//! ProviderConfig implementation for `crate::proto::provider::ProviderConfig`.

use secrecy::SecretString;
use std::time::Duration;

impl crate::proto::provider::ProviderConfig for crate::config::Config {
    fn resolve_api_key(&self, provider: &str) -> Option<SecretString> {
        // API keys are resolved from environment variables or OS keyring,
        // not from the config file.
        let mut resolver = crate::auth::CredentialResolver::new();
        for (name, p) in &self.model_providers {
            // Only pass base_url; api_key comes from env/keyring
            resolver.set_config(name, None, non_empty(&p.base_url));
        }
        // Honor the provider's declared env_var (e.g. kimi-code → KIMI_API_KEY)
        // while still accepting the derived KIMI-CODE_API_KEY form.
        let preferred = crate::provider::find_provider(provider)
            .map(|p| vec![p.env_var])
            .unwrap_or_default();
        resolver.resolve_api_key_with_env_vars(provider, &preferred)
    }

    fn resolve_base_url(&self, provider: &str) -> Option<String> {
        let mut resolver = crate::auth::CredentialResolver::new();
        for (name, p) in &self.model_providers {
            resolver.set_config(name, None, non_empty(&p.base_url));
        }
        resolver.resolve_base_url(provider)
    }

    fn retry_config(&self) -> Option<crate::provider::RetryConfig> {
        Some(crate::provider::RetryConfig::new(
            self.retry.max_attempts,
            Duration::from_millis(self.retry.initial_delay_ms),
            Duration::from_millis(self.retry.max_delay_ms),
            self.retry.multiplier,
        ))
    }

    fn resolve_headers(
        &self,
        provider: &str,
    ) -> Option<std::collections::HashMap<String, String>> {
        self.model_providers
            .get(provider)
            .map(|p| p.headers.clone())
            .filter(|h| !h.is_empty())
    }
}

fn non_empty(s: &str) -> Option<String> {
    if s.is_empty() {
        None
    } else {
        Some(s.to_owned())
    }
}
