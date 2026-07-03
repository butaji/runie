# Centralize `reqwest` client and URL normalization

## Status

`done` (2026-07-02)

## Changes

Created `runie-provider/src/http.rs` with:
- `build_client()` — central `reqwest::Client` constructor with standardized timeouts
- `normalize_base_url()` — strip trailing slashes
- `normalize_api_key()` — trim whitespace
- `bearer_header()` — build `Bearer <key>` auth header
- `request_url()` — format full URL from base + path

Updated all call sites:
- `model_client.rs::new()` → `crate::http::build_client()` + `normalize_base_url`
- `openai/mod.rs::new()` and `from_http_client()` → `build_client()` + `normalize_api_key`
- `lib.rs::resolve_credentials()` → `normalize_api_key` + `normalize_base_url`
- `lib.rs::fetch_models()` → `request_url()` + `bearer_header()`

The factory in `runie-core` uses `REQUEST_TIMEOUT`/`CONNECT_TIMEOUT` constants from `runie_core::provider`, which are the canonical source.

## Acceptance criteria

- [x] All provider creation paths use the centralized helper.
- [x] Trailing slashes and key trimming are consistent.
- [x] Unit tests for normalization helpers pass.
- [x] All tests pass.

## Tests

- [x] `cargo test --workspace` passes.
- [x] New unit tests for `http` module (normalization, client building).
- [x] Pre-existing clippy warnings are unrelated to this change.
