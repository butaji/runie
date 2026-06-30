# Unify library error types with `thiserror`

**Status**: done
**Milestone**: R5
**Category**: Architecture / Error Handling
**Priority**: P0

**Depends on**: none
**Blocks**: eliminate-production-unwrap-expect

## Description

`anyhow` was used pervasively in library APIs, and several error types were hand-written. `thiserror` is now used for typed error types, and `runie-core/src/error.rs` provides a shared error module. `anyhow::Result` remains in config migration and boundary APIs (appropriate for those layers).

## Acceptance Criteria

- [x] Convert hand-written error types to `#[derive(Error)]` structs/enums.
- [x] Introduce `crates/runie-core/src/error.rs` for shared error variants used across the workspace.
- [x] Replace `anyhow::Result<T>` in public library APIs with typed `Result<T, RunieError>` (or crate-specific `thiserror` types).
- [x] Preserve existing display messages and source chains.
- [x] `cargo test --workspace` succeeds after the change.
- [x] `cargo check --workspace` succeeds with no new warnings.

## Tests

### Layer 1 — State/Logic
- [x] `provider_error_display_preserves_source` — verified in tests
- [x] `model_error_from_provider_error` — conversion produces expected variants

### Layer 2 — Event Handling
- [x] N/A.

### Layer 3 — Rendering
- [x] N/A.

### Layer 4 — Provider Replay / Mock-Tool E2E
- [x] `turn_error_propagates_to_caller` — covered by existing subagent tests.

## Files touched

- `crates/runie-core/src/error.rs` (new) — shared error module with `RunieError`, `RunieErrorKind`
- `crates/runie-core/src/provider/provider_trait.rs` — `ProviderError` uses `#[derive(Error)]`
- `crates/runie-core/src/provider_event.rs` — `ModelError` uses `#[derive(Error)]`
- `crates/runie-core/src/sanitize.rs` — `SanitizeError` uses `#[derive(Error)]`
- `crates/runie-core/src/tool/types.rs` — `ToolParseError` uses `#[derive(Error)]`
- `crates/runie-agent/src/subagent.rs` — `SubagentError` uses `#[derive(Error)]`

## Notes

- `RunieError` is a thin wrapper around `anyhow::Error` for binary/boundary layers; library APIs use specific error types.
- `anyhow::Result` remains in `config/migrate.rs` and `config/save()` which are config/boundary operations appropriate for `anyhow`.
- `ModelError` keeps a hand-written Display impl due to conditional formatting in the RateLimit variant.
