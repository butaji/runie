# Fix env lock isolation and remove duplicates

## Status

`done`

## Description

There were three `ENV_LOCK` definitions. Consolidated on `runie_testing::ENV_LOCK` and ensured every `set_var`/`remove_var` acquires it. Fixed `temp_home()` so `HOME` is isolated per test.

## Implementation

Updated `crates/runie-testing/src/env_lock.rs`:
- Added `EnvRestore` struct that automatically restores environment variables on drop
- Added `with_env()` function that acquires `ENV_LOCK`, creates an `EnvRestore` guard, and restores vars on return
- Made `env_lock()` available for simple lock-only cases

Updated `crates/runie-testing/src/fixtures.rs`:
- Replaced `Once`-based `temp_home()` with a version that returns `(TempDir, HomeRestore)`
- `HomeRestore` saves the original `HOME` and restores it on drop
- `load_default_config_for_test()` and `mock_provider()` now use `with_env()` for atomic env mutation

## Acceptance criteria

- [x] **Unit tests** — Concurrent env-mutating tests pass reliably; `temp_home()` returns isolated dirs.
- [x] **E2E tests** — Tests that rely on `HOME`/`RUNIE_*` env vars pass in any order.
- [x] **Live tmux tests** — Not applicable; test-only task.

## Tests

### Unit tests
- `ENV_LOCK` guards all env mutations via `with_env()`.
- `temp_home()` isolation across calls.

### E2E tests
- Full test suite passes with randomized order.

### Live tmux tests
- N/A.
