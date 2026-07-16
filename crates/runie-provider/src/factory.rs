//! Concrete [`ProviderFactory`] implementation backed by `BuiltProvider`.

use async_trait::async_trait;
use secrecy::ExposeSecret;
use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, Instant};

use crate::config::ProviderConfigResolver;
use crate::{build_provider, find_provider, validate_api_key, ProviderError};
use runie_core::actors::provider::{BuiltProvider, ProviderFactory};
use runie_core::auth::KeyringStore;
use runie_core::config::Config;
use runie_core::proto::ProviderConfig;

#[cfg(feature = "replay")]
use crate::replay::{Protocol, ReplayProvider};

/// Identifier for a deployment (e.g., provider + model combination).
pub type DeploymentId = String;

/// Tracks failed attempts and cooldown state per deployment.
#[derive(Debug, Clone, Default)]
pub struct DeploymentCooldown {
    /// Number of consecutive failed attempts per deployment.
    failed_attempts: HashMap<DeploymentId, u32>,
    /// Timestamp when cooldown expires per deployment.
    cooldown_until: HashMap<DeploymentId, Instant>,
    /// Number of failures allowed before entering cooldown.
    allowed_fails: u32,
    /// Duration of the cooldown period.
    cooldown_duration: Duration,
}

impl DeploymentCooldown {
    /// Create a new cooldown tracker with default settings.
    ///
    /// - `allowed_fails`: 3 consecutive failures trigger cooldown
    /// - `cooldown_duration`: 60 seconds
    pub fn new() -> Self {
        Self::with_config(3, Duration::from_secs(60))
    }

    /// Create a cooldown tracker with custom settings.
    pub fn with_config(allowed_fails: u32, cooldown_duration: Duration) -> Self {
        Self {
            failed_attempts: HashMap::new(),
            cooldown_until: HashMap::new(),
            allowed_fails,
            cooldown_duration,
        }
    }

    /// Record a failed attempt for a deployment.
    ///
    /// If the number of consecutive failures reaches `allowed_fails`,
    /// the deployment enters a cooldown period.
    pub fn mark_failed(&mut self, deployment: &str) {
        let count = self.failed_attempts.entry(deployment.to_owned()).or_insert(0);
        *count += 1;

        if *count >= self.allowed_fails {
            let until = Instant::now() + self.cooldown_duration;
            self.cooldown_until.insert(deployment.to_owned(), until);
            tracing::debug!(
                deployment,
                failures = *count,
                cooldown_secs = self.cooldown_duration.as_secs(),
                "deployment entered cooldown"
            );
        }
    }

    /// Record a successful attempt for a deployment.
    ///
    /// This resets the failure counter.
    pub fn mark_success(&mut self, deployment: &str) {
        if self.failed_attempts.remove(deployment).is_some() {
            tracing::debug!(deployment, "deployment failure count reset");
        }
        self.cooldown_until.remove(deployment);
    }

    /// Check if a deployment is currently in cooldown.
    pub fn is_cooled(&self, deployment: &str) -> bool {
        if let Some(until) = self.cooldown_until.get(deployment) {
            if Instant::now() < *until {
                return true;
            }
        }
        false
    }

    /// Get remaining cooldown time for a deployment, if any.
    pub fn remaining_cooldown(&self, deployment: &str) -> Option<Duration> {
        self.cooldown_until
            .get(deployment)
            .map(|until| until.saturating_duration_since(Instant::now()))
            .filter(|d| !d.is_zero())
    }

    /// Get the number of consecutive failures for a deployment.
    pub fn failure_count(&self, deployment: &str) -> u32 {
        self.failed_attempts.get(deployment).copied().unwrap_or(0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_no_cooldown_on_success() {
        let cooldown = DeploymentCooldown::new();
        assert!(!cooldown.is_cooled("deploy-1"));
        assert_eq!(cooldown.failure_count("deploy-1"), 0);
    }

    #[test]
    fn test_no_cooldown_before_allowed_fails() {
        let mut cooldown = DeploymentCooldown::with_config(3, Duration::from_secs(60));

        cooldown.mark_failed("deploy-1");
        assert!(!cooldown.is_cooled("deploy-1"));
        assert_eq!(cooldown.failure_count("deploy-1"), 1);

        cooldown.mark_failed("deploy-1");
        assert!(!cooldown.is_cooled("deploy-1"));
        assert_eq!(cooldown.failure_count("deploy-1"), 2);
    }

    #[test]
    fn test_cooldown_after_allowed_fails() {
        let mut cooldown = DeploymentCooldown::with_config(2, Duration::from_secs(60));

        cooldown.mark_failed("deploy-1");
        assert!(!cooldown.is_cooled("deploy-1"));

        cooldown.mark_failed("deploy-1");
        assert!(cooldown.is_cooled("deploy-1"));
        assert!(cooldown.remaining_cooldown("deploy-1").is_some());
    }

    #[test]
    fn test_success_resets_failures() {
        let mut cooldown = DeploymentCooldown::new();

        cooldown.mark_failed("deploy-1");
        cooldown.mark_failed("deploy-1");
        assert_eq!(cooldown.failure_count("deploy-1"), 2);

        cooldown.mark_success("deploy-1");
        assert_eq!(cooldown.failure_count("deploy-1"), 0);
        assert!(!cooldown.is_cooled("deploy-1"));
    }

    #[test]
    fn test_success_after_cooldown() {
        let mut cooldown = DeploymentCooldown::with_config(2, Duration::from_secs(60));

        cooldown.mark_failed("deploy-1");
        cooldown.mark_failed("deploy-1");
        assert!(cooldown.is_cooled("deploy-1"));

        cooldown.mark_success("deploy-1");
        assert!(!cooldown.is_cooled("deploy-1"));
    }

    #[test]
    fn test_independent_deployments() {
        let mut cooldown = DeploymentCooldown::with_config(2, Duration::from_secs(60));

        cooldown.mark_failed("deploy-1");
        cooldown.mark_failed("deploy-1");
        assert!(cooldown.is_cooled("deploy-1"));

        // deploy-2 should not be in cooldown
        assert!(!cooldown.is_cooled("deploy-2"));
        assert_eq!(cooldown.failure_count("deploy-2"), 0);

        cooldown.mark_failed("deploy-2");
        assert!(!cooldown.is_cooled("deploy-2"));
    }

    #[test]
    fn test_cooldown_expiration() {
        let mut cooldown = DeploymentCooldown::with_config(1, Duration::from_millis(10));

        cooldown.mark_failed("deploy-1");
        assert!(cooldown.is_cooled("deploy-1"));

        // Wait for cooldown to expire
        std::thread::sleep(Duration::from_millis(20));
        assert!(!cooldown.is_cooled("deploy-1"));
    }

    #[test]
    fn test_zero_allowed_fails() {
        let mut cooldown = DeploymentCooldown::with_config(0, Duration::from_secs(60));

        cooldown.mark_failed("deploy-1");
        // With 0 allowed fails, any failure triggers cooldown immediately
        assert!(cooldown.is_cooled("deploy-1"));
    }
}

/// The production provider factory.
///
/// This is the only production implementation of [`ProviderFactory`] and the
/// only production code path that constructs providers.
#[derive(Clone)]
pub struct BuiltProviderFactory {
    /// Optional keyring store for credential resolution.
    /// When `None`, uses `OsKeyringStore` (production default).
    keyring_store: Option<Arc<dyn KeyringStore>>,
    /// Tracks deployment cooldown state for failed attempts.
    deployment_cooldown: DeploymentCooldown,
}

impl Default for BuiltProviderFactory {
    fn default() -> Self {
        Self::new()
    }
}

impl BuiltProviderFactory {
    /// Create a new factory using the OS keyring.
    pub fn new() -> Self {
        Self {
            keyring_store: None,
            deployment_cooldown: DeploymentCooldown::new(),
        }
    }

    /// Create a factory with an injectable keyring store.
    ///
    /// Use this in tests to avoid hitting the OS keyring.
    pub fn with_keyring_store(store: Arc<dyn KeyringStore>) -> Self {
        Self {
            keyring_store: Some(store),
            deployment_cooldown: DeploymentCooldown::new(),
        }
    }

    /// Create a factory with custom cooldown settings.
    pub fn with_cooldown_config(allowed_fails: u32, cooldown_duration: Duration) -> Self {
        Self {
            keyring_store: None,
            deployment_cooldown: DeploymentCooldown::with_config(allowed_fails, cooldown_duration),
        }
    }

    /// Record a failed attempt for a deployment.
    pub fn mark_deployment_failed(&mut self, deployment: &str) {
        self.deployment_cooldown.mark_failed(deployment);
    }

    /// Record a successful attempt for a deployment.
    pub fn mark_deployment_success(&mut self, deployment: &str) {
        self.deployment_cooldown.mark_success(deployment);
    }

    /// Check if a deployment is currently in cooldown.
    pub fn is_deployment_cooled(&self, deployment: &str) -> bool {
        self.deployment_cooldown.is_cooled(deployment)
    }

    /// Get remaining cooldown time for a deployment, if any.
    pub fn deployment_remaining_cooldown(&self, deployment: &str) -> Option<Duration> {
        self.deployment_cooldown.remaining_cooldown(deployment)
    }

    /// Get the number of consecutive failures for a deployment.
    pub fn deployment_failure_count(&self, deployment: &str) -> u32 {
        self.deployment_cooldown.failure_count(deployment)
    }

    /// Try to build a replay provider from `RUNIE_REPLAY_FIXTURES` env var.
    ///
    /// The env var should contain a comma-separated list of file paths to SSE
    /// fixtures. The protocol is inferred from the fixture contents, or can be
    /// explicitly set via `RUNIE_REPLAY_PROTOCOL` (values: `openai`, `anthropic`).
    #[cfg(feature = "replay")]
    fn try_build_replay_provider(provider: &str, model: &str) -> Option<BuiltProvider> {
        let fixture_list = std::env::var("RUNIE_REPLAY_FIXTURES").ok()?;
        if fixture_list.trim().is_empty() {
            return None;
        }

        let paths: Vec<&str> = fixture_list.split(',').map(str::trim).collect();
        let mut fixtures = Vec::new();

        for path in paths {
            if path.is_empty() {
                continue;
            }
            match std::fs::read_to_string(path) {
                Ok(contents) => fixtures.push(contents),
                Err(e) => {
                    tracing::warn!(path, error = %e, "failed to read replay fixture");
                    return None;
                }
            }
        }

        if fixtures.is_empty() {
            return None;
        }

        // Determine protocol from env var or fixture inference.
        let protocol = match std::env::var("RUNIE_REPLAY_PROTOCOL").ok().as_deref() {
            Some("anthropic") => Protocol::Anthropic,
            Some("openai") => Protocol::OpenAi,
            _ => ReplayProvider::infer_protocol(&fixtures),
        };

        let replay = ReplayProvider::new(fixtures, protocol);
        tracing::debug!(provider, model, protocol = ?protocol, "using replay provider");

        // Use provided provider/model, or defaults for replay context.
        let key = if provider.is_empty() || provider == "replay" {
            "openai"
        } else {
            provider
        };
        let model = if model.is_empty() { "replay" } else { model };

        Some(BuiltProvider::from_provider(Box::new(replay), key, model))
    }
}

#[async_trait]
impl ProviderFactory for BuiltProviderFactory {
    fn build(
        &self,
        provider: &str,
        model: &str,
        config: &Config,
    ) -> Result<BuiltProvider, ProviderError> {
        // Check for replay mode first.
        #[cfg(feature = "replay")]
        if let Some(replay_provider) = Self::try_build_replay_provider(provider, model) {
            return Ok(replay_provider);
        }

        build_provider(
            provider,
            model,
            Some(Arc::new(config.clone()) as Arc<dyn ProviderConfig>),
        )
    }

    async fn validate_key(&self, base_url: &str, api_key: &str) -> anyhow::Result<Vec<String>> {
        validate_api_key(base_url, api_key).await
    }

    fn resolve_credentials(&self, provider: &str, config: &Config) -> (String, String) {
        let config_arc = Arc::new(config.clone()) as Arc<dyn ProviderConfig>;
        let resolver = if let Some(store) = &self.keyring_store {
            ProviderConfigResolver::with_keyring_store(config_arc, store.clone())
        } else {
            ProviderConfigResolver::new(config_arc)
        };
        let base_url = resolver
            .resolve_base_url(provider)
            .or_else(|| default_base_url(provider))
            .unwrap_or_default();
        // Expose secret at the boundary where credentials are needed
        let api_key = resolver
            .resolve_api_key(provider)
            .map(|s| s.expose_secret().clone())
            .unwrap_or_default();
        (base_url, api_key)
    }
}

fn default_base_url(provider: &str) -> Option<String> {
    find_provider(provider).map(|m| m.base_url.to_owned())
}
