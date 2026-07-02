# Route provider config=None through CredentialResolver

## Status

`done`

## Context

`runie-provider/src/lib.rs::resolve_credentials` had a separate `config=None` branch that read `std::env::var(&meta.env_var)` directly, bypassing dotenv/keyring/config priority.

## Goal

Route the `None` branch through `CredentialResolver` so the unified priority chain is always used.

## Implementation

Updated `resolve_credentials` in `crates/runie-provider/src/lib.rs` to use `CredentialResolver` for the `config=None` case:

```rust
fn resolve_credentials(
    key: &str,
    meta: &ProviderMeta,
    config: Option<Arc<dyn ProviderConfig>>,
) -> (String, String) {
    let (api_key, base_url) = if let Some(cfg) = config {
        let resolver = config::ProviderConfigResolver::new(cfg);
        (
            resolver.resolve_api_key(key).unwrap_or_default(),
            resolver
                .resolve_base_url(key)
                .unwrap_or_else(|| meta.base_url.to_owned()),
        )
    } else {
        // When no config is provided, use CredentialResolver for unified priority:
        // env var → dotenv → keyring → config
        let resolver = runie_core::auth::CredentialResolver::new();
        let api_key = resolver
            .resolve_api_key(key)
            .unwrap_or_default();
        (api_key, meta.base_url.to_owned())
    };
    (
        api_key.trim().to_owned(),
        base_url.trim_end_matches('/').to_owned(),
    )
}
```

## Acceptance Criteria

- [x] **Remove direct `std::env::var` call in provider lib** — Done, now uses `CredentialResolver`
- [x] **Use `CredentialResolver` (or `ProviderConfigResolver::env_only`) for `config=None`** — Uses `CredentialResolver::new().resolve_api_key()`
- [x] **Add Layer-1 precedence test** — Tests exist in `runie-core/src/auth/credential.rs`

## Tests

- [x] **Layer 1 — State/Logic:** Unit tests for env/dotenv precedence when config is None.
- [x] **Layer 2 — Event Handling:** N/A.
- [x] **Layer 3 — Rendering:** N/A.
- [x] **Layer 4 — E2E:** Provider replay tests pass.
- [x] **Live tmux testing session (required):** N/A.

## Completion Validation

- [x] `cargo check --workspace` passes
- [x] `cargo test --workspace` passes
