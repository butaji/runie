# Unify library error types with `thiserror`

**Status**: todo
**Milestone**: R5
**Category**: Architecture / Error Handling
**Priority**: P0

**Depends on**: none
**Blocks**: eliminate-production-unwrap-expect

## Description

`anyhow` is used pervasively in library APIs, and several error types are hand-written (`ProviderError`, `ModelError`, `SubagentError`, `SanitizeError`, `ToolParseError`, `runie-protocol::Error`). `thiserror` is already a workspace dependency (used only in `runie-testing`). Unify library errors on `thiserror`, reserve `anyhow` for binary boundaries, and introduce a shared error module in `runie-core`.

## Acceptance Criteria

- [ ] Convert hand-written error types to `#[derive(Error)]` structs/enums.
- [ ] Introduce `crates/runie-core/src/error.rs` for shared error variants used across the workspace.
- [ ] Replace `anyhow::Result<T>` in public library APIs with typed `Result<T, RunieError>` (or crate-specific `thiserror` types).
- [ ] Preserve existing display messages and source chains.
- [ ] `cargo test --workspace` succeeds after the change.
- [ ] `cargo check --workspace` succeeds with no new warnings.

## Tests

### Layer 1 — State/Logic
- [ ] `provider_error_display_preserves_source` — `ProviderError` still formats with its source.
- [ ] `model_error_from_provider_error` — conversion from provider error produces the expected variant.

### Layer 2 — Event Handling
- [ ] N/A.

### Layer 3 — Rendering
- [ ] N/A.

### Layer 4 — Provider Replay / Mock-Tool E2E
- [ ] `turn_error_propagates_to_caller` — a failed provider turn returns a typed error that reaches the caller.

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
