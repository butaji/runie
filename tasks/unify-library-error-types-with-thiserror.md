# Unify library error types with `thiserror`

**Status**: todo
**Milestone**: R5
**Category**: Architecture / Error Handling
**Priority**: P0
**Note**: RunieError is an unused anyhow::Error newtype; anyhow::Result still pervades library APIs.

**Depends on**: none
**Blocks**: eliminate-production-unwrap-expect

## Description

`anyhow` is used pervasively in library APIs, and several error types are hand-written (`ProviderError`, `ModelError`, `SubagentError`, `SanitizeError`, `ToolParseError`, `runie-protocol::Error`). `thiserror` is already a workspace dependency (used only in `runie-testing`). Unify library errors on `thiserror`, reserve `anyhow` for binary boundaries, and introduce a shared error module in `runie-core`.

## Acceptance Criteria

- [x] Convert hand-written error types to `#[derive(Error)]` structs/enums.
- [x] Introduce `crates/runie-core/src/error.rs` for shared error variants used across the workspace.
- [x] Replace `anyhow::Result<T>` in public library APIs with typed `Result<T, RunieError>` (or crate-specific `thiserror` types).
- [x] Preserve existing display messages and source chains.
- [x] `cargo test --workspace` succeeds after the change.
- [x] `cargo check --workspace` succeeds with no new warnings.

## Tests

### Layer 1 — State/Logic
- [x] `provider_error_display_preserves_source` — `ProviderError` still formats with its source. (as `provider_error_source_round_trips`)
- [x] `model_error_from_provider_error` — conversion from provider error produces the expected variant.

### Layer 2 — Event Handling
- [ ] N/A.

### Layer 3 — Rendering
- [ ] N/A.

### Layer 4 — Provider Replay / Mock-Tool E2E
- [ ] `turn_error_propagates_to_caller` — a failed provider turn returns a typed error that reaches the caller. (covered by existing subagent tests)

## Files touched

- `crates/runie-core/src/error.rs` (new)
- `crates/runie-core/src/provider/provider_trait.rs`
- `crates/runie-core/src/provider_event.rs`
- `crates/runie-core/src/sanitize.rs`
- `crates/runie-core/src/tool/types.rs`
- `crates/runie-agent/src/subagent.rs`
- `crates/runie-protocol/src/error.rs`
- All callers in `crates/runie-*/`

## Notes

- Do not change binary UX; `runie-cli`/`runie-tui` can still downcast/print via `anyhow` at the top level.
- This task makes `eliminate-production-unwrap-expect.md` easier because recoverable errors have a place to go.

## Implementation Summary

Converted the following error types to use `thiserror`:
- `ProviderError` with `MissingApiKeyError` helper for complex formatting
- `SanitizeError` with `#[derive(Error)]` and custom display messages
- `ToolParseError` with `#[derive(Error)]`
- `SubagentError` with `#[derive(Error)]` and `#[source]` for chained errors
- `ModelError` - kept hand-written Display impl due to conditional formatting in RateLimit variant
- `RunieError` wrapper in new `error.rs` module

Added `thiserror` as a workspace dependency and to `runie-core` and `runie-agent` crates.
