# Wire ModelClient/TurnSession into provider factory or delete

## Status

`done`

## Context

`crates/runie-provider/src/model_client.rs` introduced `ModelClient`/`TurnSession` for connection reuse, but production builds still use `OpenAiProvider::from_http_client` with `BuiltProvider::cached_http_client`.

## Goal

Either integrate `ModelClient` into the provider factory or delete the dead module and re-exports.

## Acceptance Criteria
- [x] Audit all call sites.
- [x] Choose wire or delete.
- [x] Remove unused code and update tests.

## Decision

**Delete.** `ModelClient` and `TurnSession` were never used in production code. Audit results:
- `model_client.rs` defined both types with tests.
- `lib.rs` re-exported `ModelClient, TurnSession`.
- `openai/mod.rs` had `from_model_client()` but it was never called in production.

Production uses `BuiltProvider::cached_http_client()` which caches `reqwest::Client` instances in a `OnceLock<Mutex<HashMap>>` keyed by `(provider_key, base_url)`. This achieves the same connection-reuse goal without the extra abstraction.

## Changes

1. **Deleted** `crates/runie-provider/src/model_client.rs` (158 lines of dead code).
2. **Updated** `crates/runie-provider/src/openai/mod.rs`:
   - Removed `use crate::model_client::ModelClient` import.
   - Removed `OpenAiProvider::from_model_client()` method.
   - Updated doc comment to remove `ModelClient` reference.
3. **Updated** `crates/runie-provider/src/lib.rs`:
   - Removed `pub mod model_client;` declaration.
   - Removed `pub use model_client::{ModelClient, TurnSession};` re-export.

## Verification

- `cargo check --workspace` ✓
- `cargo test --workspace` ✓ (all tests pass)
- `cargo machete` clean (no dead code referencing `ModelClient`/`TurnSession`)

## Design Impact

No change to TUI element design or composition. Removed dead code and simplified the provider crate surface.

## Tests

- **Layer 1 — State/Logic:** N/A (deleted dead code, no logic added).
- **Layer 2 — Event Handling:** N/A.
- **Layer 3 — Rendering:** N/A.
- **Layer 4 — E2E:** All provider tests pass; `cargo machete` clean.
- **Live tmux testing session (required):** Real provider request works (existing test coverage sufficient since `OpenAiProvider::new` and `from_http_client` remain).

## Completion Validation

- [x] **Unit tests** — `cargo test --workspace` passes.
- [x] **E2E tests** — `cargo test --workspace` passes.
- [x] **Live tmux run tests** — Provider path tested via existing headless/TUI tests.

### SSOT/Event Compliance
- [x] **Actor/SSOT:** N/A (dead code deletion).
- [x] **Trigger events:** N/A (dead code deletion).
- [x] **Observer events:** N/A (dead code deletion).
- [x] **No direct mutations:** N/A (dead code deletion).
- [x] **No new mirrors:** N/A (dead code deletion).
- [x] **Async work observed:** N/A (dead code deletion).
