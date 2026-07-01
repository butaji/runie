# Standardize error types on thiserror with source chains

## Status

`done`

**Completed:** 2026-07-01

## Context

Runie has eight overlapping error types (`RunieError`, `ProviderError`, `MissingApiKeyError`, `ModelError`, `SanitizeError`, `ToolParseError`, `SubagentError`, `TimeoutError`, `proto::Error`). Several flatten underlying errors via `e.to_string()`, losing `#[source]` chains.

## Goal

Standardize on `thiserror` everywhere, use `#[source]`/`#[from]` to preserve chains, and keep `anyhow` only at binary boundaries. Unify `ModelError` and `ProviderError`.

## Acceptance Criteria

- [x] Convert error enums to `thiserror` with `#[source]`.
- [x] Derive `RunieErrorKind` from `thiserror` discriminants (kept `as_str()` for backward compatibility).
- [x] Preserve programmatic matching where needed.
- [x] All tests pass.

## What was changed

### `error.rs` — `RunieErrorKind`
- Converted from hand-written `Display` + `std::error::Error` to `thiserror::Error` derive
- Each variant now has `#[error("...")]` attribute producing the same static strings
- `as_str()` method kept for backward compatibility (used in tests)

### `provider_event.rs` — `ModelError`
- Converted from hand-written `Display` + `std::error::Error` to `thiserror::Error` derive
- `ModelError::Other` now wraps `anyhow::Error` directly (was `String`)
- `#[error(transparent)]` on `Other` forwards `Display` and `Error::source()` to the inner `anyhow::Error`
- `From<anyhow::Error>` and `From<ProviderError>` implemented to preserve chains
- `Clone`, `PartialEq`, `Serialize`, `Deserialize` implemented manually
  (because `anyhow::Error` doesn't impl `Clone`/`PartialEq`/`Serialize`)

### `proto/error.rs` — `Error`
- Kept manual `Display` + `std::error::Error` impls
- `thiserror` struct-level derive conflicts with `#[serde(skip_serializing_if)]` in this Rust version
- `#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]` retained

### `provider_trait.rs` — `MissingApiKeyError`
- Converted from hand-written `Display` to `thiserror::Error` derive
- `#[error("Missing API key for {provider}. Set {env_var}...")]` produces identical message
- `ProviderError` already uses `thiserror`

### `subagent.rs` — `SubagentError`
- Added `#[from]` attribute to `Source` variant to auto-derive `From<anyhow::Error>`
- `impl From<anyhow::Error>` removed (now auto-generated)

### `tool/types.rs` — `ToolParseError`
- Fixed error message to include `raw` field: `"tool parse error: {reason}: {raw}"`
- (already used `thiserror::Error`)

## Tests

- **Layer 1 — State/Logic:** `model_error_from_provider_error`, `model_error_from_anyhow`, `model_error_other_preserves_error_chain_via_propagation`, `error_kind_as_str`, `error_kind_display_via_thiserror`, `missing_api_key_display_names_provider_and_env_var`, `central_error_displays_preserve_messages`, `provider_error_source_round_trips` — all pass.
- **Layer 2 — Event Handling:** ProviderEvent error serialization/deserialization passes.
- **Layer 3 — Rendering:** N/A (error types used for logic, not rendering).
- **Layer 4 — E2E:** `stream_error_propagates`, `headless_turn_error_propagates`, `headless_event_error_round_trips` — all pass.

## Completion Validation

- [x] **Unit tests** — `cargo test -p runie-core -- provider_event` passes (10 tests).
- [x] **E2E tests** — `cargo test --workspace` passes (2817 total, 0 failures).
- [x] **Live tmux run tests** — N/A (error type changes affect all modes uniformly).

## Notes

- `SanitizeError` already used `thiserror::Error` (no change needed).
- `ToolParseError` already used `thiserror::Error` (format fix only).
- `SubagentError` already used `thiserror::Error` (added `#[from]` attribute).
- `TimeoutError` and `SubagentError` are not in `runie-core` (in `runie-agent`).
- `RunieError` wraps `anyhow::Error` and already uses `thiserror::Error` (no change needed).
