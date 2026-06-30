//! Unified credential resolver for API keys and base URLs.

use std::collections::HashMap;

// ---------------------------------------------------------------------------
// Unified Credential Resolver
// ---------------------------------------------------------------------------

/// Unified credential resolver implementing consistent priority:
/// 1. Environment variables
/// 2. .env file (via dotenvy)
/// 3. OS keyring
/// 4. Config file
///
/// This resolver is used by both `runie-core` (for Config) and `runie-provider`
/// (for ProviderConfigResolver) to ensure consistent credential resolution.
#[derive(Debug, Clone)]
pub struct CredentialResolver {
    /// Environment variables captured at construction.
    env: HashMap<String, String>,
    /// Variables loaded from .env file via dotenvy.
    dotenv: HashMap<String, String>,
    /// Provider config entries (provider name -> (api_key, base_url)).
    entries: HashMap<String, (Option<String>, Option<String>)>,
}

impl Default for CredentialResolver {
    fn default() -> Self {
        Self::new()
    }
}

impl CredentialResolver {
    /// Create a new resolver that captures the current environment.
    pub fn new() -> Self {
        let env: HashMap<String, String> = std::env::vars().collect();
        let dotenv = Self::load_dotenv();
        Self {
            env,
            dotenv,
            entries: HashMap::new(),
        }
    }

    /// Create a resolver with an empty environment (for testing).
    pub fn empty() -> Self {
        Self {
            env: HashMap::new(),
            dotenv: HashMap::new(),
            entries: HashMap::new(),
        }
    }

    /// Load .env file using dotenvy, returning only API-related variables.
    ///
    /// Uses dotenvy to load the .env file into the environment, then captures
    /// relevant variables (API keys and base URLs) from the environment.
    fn load_dotenv() -> HashMap<String, String> {
        // dotenvy::dotenv() loads .env into the environment
        if dotenvy::dotenv().is_err() {
            return HashMap::new();
        }
        // Capture API-related variables from the environment
        std::env::vars()
            .filter(|(key, _)| key.ends_with("_API_KEY") || key.ends_with("_BASE_URL"))
            .collect()
    }

    /// Set a config entry for a provider.
    pub fn set_config(
        &mut self,
        provider: &str,
        api_key: Option<String>,
        base_url: Option<String>,
    ) {
        self.entries.insert(provider.to_lowercase(), (api_key, base_url));
    }

    /// Resolve the API key for a provider using the standard priority chain.
    pub fn resolve_api_key(&self, provider: &str) -> Option<String> {
        let env_key = format!("{}_API_KEY", provider.to_uppercase());

        // 1. Environment variable
        if let Some(val) = self.env.get(&env_key) {
            if !val.is_empty() {
                return Some(val.clone());
            }
        }

        // 2. .env file
        if let Some(val) = self.dotenv.get(&env_key) {
            if !val.is_empty() {
                return Some(val.clone());
            }
        }

        // 3. Keyring
        if let Some(token) = super::storage::AuthStorage::get_keyring_token(provider) {
            return Some(token);
        }

        // 4. Config file
        self.entries
            .get(&provider.to_lowercase())
            .and_then(|(api_key, _)| api_key.clone())
            .filter(|s| !s.is_empty())
    }

    /// Resolve the base URL for a provider using the standard priority chain.
    pub fn resolve_base_url(&self, provider: &str) -> Option<String> {
        let env_key = format!("{}_BASE_URL", provider.to_uppercase());

        // 1. Environment variable
        if let Some(val) = self.env.get(&env_key) {
            if !val.is_empty() {
                return Some(val.clone());
            }
        }

        // 2. .env file
        if let Some(val) = self.dotenv.get(&env_key) {
            if !val.is_empty() {
                return Some(val.clone());
            }
        }

        // 3. Config file (no keyring for base URLs)
        self.entries
            .get(&provider.to_lowercase())
            .and_then(|(_, base_url)| base_url.clone())
            .filter(|s| !s.is_empty())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn resolver_priority_env_over_keyring() {
        // Environment should win over keyring
        let mut resolver = CredentialResolver::empty();
        resolver.env.insert("TESTPROVIDER_API_KEY".to_owned(), "env-key".to_owned());

        // Note: This test assumes keyring doesn't have "testprovider"
        // In practice, we just verify env is checked first
        let result = resolver.resolve_api_key("testprovider");
        assert_eq!(result, Some("env-key".to_owned()));
    }

    #[test]
    fn resolver_fallback_to_config() {
        // Config should be used when env/keyring are absent
        let mut resolver = CredentialResolver::empty();
        resolver.set_config(
            "testprovider",
            Some("config-key".to_owned()),
            Some("http://config".to_owned()),
        );

        assert_eq!(
            resolver.resolve_api_key("testprovider"),
            Some("config-key".to_owned())
        );
        assert_eq!(
            resolver.resolve_base_url("testprovider"),
            Some("http://config".to_owned())
        );
    }

    #[test]
    fn resolver_prefers_env_over_dotenv() {
        let mut resolver = CredentialResolver::empty();
        resolver
            .env
            .insert("TEST_API_KEY".to_owned(), "env-key".to_owned());
        resolver.dotenv.insert("TEST_API_KEY".to_owned(), "dotenv-key".to_owned());

        assert_eq!(
            resolver.resolve_api_key("test"),
            Some("env-key".to_owned())
        );
    }

    #[test]
    fn resolver_prefers_dotenv_over_config() {
        let mut resolver = CredentialResolver::empty();
        resolver.dotenv.insert("TEST_API_KEY".to_owned(), "dotenv-key".to_owned());
        resolver.set_config("test", Some("config-key".to_owned()), None);

        assert_eq!(
            resolver.resolve_api_key("test"),
            Some("dotenv-key".to_owned())
        );
    }
}
