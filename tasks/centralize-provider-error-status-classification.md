# Centralize provider error status classification

## Status

`done`

## Description

HTTP status classification existed in both `ProviderError::from_reqwest` and `from_sse_error`. Now `ProviderError::from_reqwest` is the single classifier and SSE errors convert through it via a shared `classify_http_status` helper.

## Changes

1. **Added `ProviderError::classify_http_status(code: u16) -> Option<Self>`** — A public helper that maps HTTP status codes to typed `ProviderError` variants:
   - 401/403 → `Auth`
   - 429 → `RateLimit`
   - 5xx → `Server`
   - Other → `None`

2. **Updated `from_reqwest`** — Now uses `classify_http_status` for status code classification.

3. **Updated `from_sse_error`** — Now uses `classify_http_status` for `InvalidStatusCode` errors, eliminating duplicate classification logic.

## Acceptance criteria

- [x] **Unit tests** — Each status code (401, 403, 429, 5xx, etc.) maps to the expected error type from both HTTP and SSE paths. (9 new tests in `retry.rs`)
- [x] **E2E tests** — Provider replay tests pass (no regressions).
- [x] **Live tmux tests** — N/A (no behavior change, just refactoring).

## Tests

### Unit tests (added to `crates/runie-provider/src/retry.rs`)
- `classify_http_status_401_auth` — 401 → Auth
- `classify_http_status_403_auth` — 403 → Auth
- `classify_http_status_429_rate_limit` — 429 → RateLimit
- `classify_http_status_500_server` — 500 → Server
- `classify_http_status_502_server` — 502 → Server
- `classify_http_status_503_server` — 503 → Server
- `classify_http_status_400_none` — 400 → None
- `classify_http_status_404_none` — 404 → None
- `from_sse_error_uses_shared_classifier` — Verifies SSE path uses same classifier

### E2E tests
- All existing tests pass.
