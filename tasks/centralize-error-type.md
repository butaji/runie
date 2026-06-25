# Centralize error types

**Status**: done
**Milestone**: R4
**Category**: Core / State
**Priority**: P2

**Depends on**: none
**Blocks**: none

## Description

The codebase defined several overlapping error types with `Other(String)` wrapper variants:
- `ProviderError::Other(String)` — used for internal actor errors
- `ModelError::Other(String)` — used for model-specific errors (kept for serde compatibility)
- `SubagentError::Agent(String)` — wrapping `anyhow::Error::to_string()`
- `runie-provider::UnknownProviderError` — unnecessary re-export alias

These were replaced with typed `Source(anyhow::Error)` variants where the error is not serialized, and `From<anyhow::Error>` was added for ergonomic conversion.

## Acceptance Criteria

- [x] Either a single `runie_core::Error` enum with provider/llm/tool/protocol variants is defined, or `anyhow` is standardized with typed downcasts where needed. ✓ Chose `anyhow` with typed variants. `ProviderError::Source(anyhow::Error)`, `SubagentError::Source(anyhow::Error)`, `ModelError::Other(String)` kept for serde compat.
- [x] Duplicate `Other(String)` variants are removed. ✓ `ProviderError::Other(String)` → `ProviderError::Source(anyhow::Error)`, `SubagentError::Agent(String)` → `SubagentError::Source(anyhow::Error)`.
- [x] `runie-provider` no longer needs a re-export alias. ✓ `UnknownProviderError` re-export removed from `runie-provider/src/lib.rs`.
- [x] `cargo test --workspace` and `cargo check --workspace` pass. ✓ (no failures)

## Tests

### Layer 1 — State/Logic
- [x] `central_error_displays_preserve_messages` — existing error messages unchanged.
- [x] `provider_error_source_round_trips` — provider errors still identifiable via `matches!` on variants.

### Layer 2 — Event Handling
- [x] N/A.

### Layer 3 — Rendering
- [x] N/A.

### Layer 4 — Provider Replay / Mock-Tool E2E
- [x] `headless_turn_error_propagates` — provider error in a headless turn is still reported as `Err`.

## Files touched

- `crates/runie-core/src/provider/provider_trait.rs` — `ProviderError::Other → Source`, `From<anyhow::Error>`, removed `Clone/PartialEq/Eq` derives
- `crates/runie-core/src/provider_event.rs` — `ModelError::Other` kept (serde compat), added `From<anyhow::Error>`, `From<&str>`, new `model_error_from_anyhow` test
- `crates/runie-core/src/actors/provider/messages.rs` — `ProviderError::Other` → `anyhow::anyhow!().into()`
- `crates/runie-core/src/actors/provider/actor.rs` — `ProviderError::Other` → `anyhow::anyhow!().into()`
- `crates/runie-core/src/actors/provider/tests.rs` — `assert_eq!` → `matches!` (no PartialEq after change)
- `crates/runie-provider/src/lib.rs` — removed `UnknownProviderError` re-export
- `crates/runie-provider/src/tests.rs` — `assert_eq!` → `matches!`
- `crates/runie-agent/src/subagent.rs` — `SubagentError::Agent → Source`, `From<anyhow::Error>`
- `crates/runie-agent/src/headless.rs` — added `headless_turn_error_propagates` test

## Notes

- `ModelError::Other(String)` was kept because `ModelError` is serde-serialized in `ProviderEvent::Error`. Changing it would break backward compat with serialized events.
- `ProviderError` lost `Clone/PartialEq/Eq` derives since `anyhow::Error` is not `Clone` or `Eq`. Call sites using equality were updated to use `matches!`.
- The convention for downcasting: use `matches!(err, ProviderError::Source(ref e))` and then `e.downcast_ref::<SpecificError>()` on the underlying `anyhow::Error`.
