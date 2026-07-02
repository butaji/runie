# Add parameterized tests with `test-case`

## Status

`done`

## Description

Many parser/provider tests are repeated for similar inputs. Use `test-case` to reduce boilerplate and improve coverage.

## Acceptance criteria

1. **Unit tests** — Representative parser/provider tests use `test-case` and cover more inputs.
2. **E2E tests** — Existing replay tests still pass.
3. **Live tmux tests** — Not applicable; test-only task.

## Changes

### Cargo.toml (workspace)
Added `test-case = "3.1"` to `[workspace.dependencies]`.

### crates/runie-provider/Cargo.toml
Added `test-case.workspace = true` to `[dev-dependencies]`.

### crates/runie-provider/src/retry.rs
Refactored 8 separate status classification tests into a single parameterized `classify_http_status` test:
- Added `#[test_case]` for codes: 401, 403, 429, 500, 502, 503, 400, 404, 418
- Reduced 8 test functions (64+ lines) to 1 parameterized test (35 lines)
- Added coverage for 418 (additional 4xx case)

Refactored 6 typed error retryable tests into a single parameterized `is_retryable_for_typed_errors` test:
- Added `#[test_case]` for: RateLimit, Timeout, Network, Server, Auth, ContextLength

Refactored 6 string error retryable tests into a single parameterized `is_retryable_for_string_errors` test:
- Added `#[test_case]` for: "server overloaded", "rate limit exceeded", "timeout error", "connection refused", "try again later", "401 Unauthorized", "400 Bad Request", "invalid request"

## Tests

### Unit tests
- ✅ `classify_http_status` — 9 parameterized cases covering auth, rate-limit, server errors, and None returns
- ✅ `is_retryable_for_typed_errors` — 6 parameterized cases for typed ProviderError variants
- ✅ `is_retryable_for_string_errors` — 8 parameterized cases for string-based heuristics

### E2E tests
- ✅ All existing replay tests pass (32 tests in retry module)

### Live tmux tests
- N/A — test-only task
