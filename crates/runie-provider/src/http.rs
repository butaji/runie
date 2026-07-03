//! Centralized HTTP client construction for all providers.
//!
//! All `reqwest::Client` instances are built through this module, ensuring:
//! - Consistent timeout configuration (`REQUEST_TIMEOUT`, `CONNECT_TIMEOUT`)
//! - Consistent URL normalization (trailing slash stripping)
//! - Consistent API key normalization (whitespace trimming)
//! - Connection pooling via shared clients per provider+URL pair

use std::sync::Arc;

use secrecy::ExposeSecret;

use reqwest::Client;

/// Build a `reqwest::Client` with standard timeouts.
///
/// Uses [`REQUEST_TIMEOUT`](crate::REQUEST_TIMEOUT) and [`CONNECT_TIMEOUT`](crate::CONNECT_TIMEOUT).
/// Falls back to a default client if construction fails.
pub fn build_client() -> Arc<Client> {
    let client = Client::builder()
        .timeout(crate::REQUEST_TIMEOUT)
        .connect_timeout(crate::CONNECT_TIMEOUT)
        .build()
        .unwrap_or_else(|_| Client::new());
    Arc::new(client)
}

/// Normalize a base URL: strip trailing slashes.
pub fn normalize_base_url(url: &str) -> String {
    url.trim_end_matches('/').to_owned()
}

/// Normalize an API key: trim leading/trailing whitespace.
pub fn normalize_api_key(key: &str) -> String {
    key.trim().to_owned()
}

/// Build an Authorization header value for a Bearer token.
pub fn bearer_header(api_key: &str) -> String {
    format!("Bearer {}", normalize_api_key(api_key))
}

/// Build an Authorization header value for a Bearer token from a SecretString.
pub fn bearer_header_secret(api_key: &secrecy::SecretString) -> String {
    format!("Bearer {}", normalize_api_key(api_key.expose_secret()))
}

/// Format a full request URL from a base URL and a path.
pub fn request_url(base_url: &str, path: &str) -> String {
    let base = normalize_base_url(base_url);
    let path = path.trim_start_matches('/');
    format!("{}/{}", base, path)
}

#[cfg(test)]
mod tests {
    use secrecy::SecretString;

    use super::*;

    fn ss(s: &str) -> SecretString {
        SecretString::from(s.to_owned())
    }

    #[test]
    fn normalize_base_url_strips_trailing_slash() {
        assert_eq!(
            normalize_base_url("https://api.openai.com/v1/"),
            "https://api.openai.com/v1"
        );
        assert_eq!(
            normalize_base_url("https://api.openai.com/v1"),
            "https://api.openai.com/v1"
        );
    }

    #[test]
    fn normalize_base_url_handles_multiple_trailing_slashes() {
        assert_eq!(
            normalize_base_url("https://api.openai.com/v1///"),
            "https://api.openai.com/v1"
        );
    }

    #[test]
    fn normalize_api_key_trims_whitespace() {
        assert_eq!(normalize_api_key("  key123  "), "key123");
        assert_eq!(normalize_api_key("key123"), "key123");
        assert_eq!(normalize_api_key(""), "");
    }

    #[test]
    fn bearer_header_includes_normalized_key() {
        assert_eq!(bearer_header("  key123  "), "Bearer key123");
        assert_eq!(bearer_header("key123"), "Bearer key123");
    }

    #[test]
    fn bearer_header_secret_includes_normalized_key() {
        assert_eq!(bearer_header_secret(&ss("  key123  ")), "Bearer key123");
        assert_eq!(bearer_header_secret(&ss("key123")), "Bearer key123");
    }

    #[test]
    fn request_url_normalizes_base() {
        assert_eq!(
            request_url("https://api.openai.com/v1/", "models"),
            "https://api.openai.com/v1/models"
        );
        assert_eq!(
            request_url("https://api.openai.com/v1", "/chat/completions"),
            "https://api.openai.com/v1/chat/completions"
        );
    }

    #[test]
    fn build_client_returns_arc_client() {
        let client = build_client();
        assert!(Arc::strong_count(&client) >= 1);
    }
}
