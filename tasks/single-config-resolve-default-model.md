# Single `Config::resolve_default_model`

## Status

`done` (2026-07-02)

## Changes

`ConfigState` lacked `provider` and `default_model` fields that `Config` has, causing `ConfigState::resolve_default_model` to take a simplified fallback path. Fixed by:

1. Added `provider: Option<String>` and `default_model: Option<String>` to `ConfigState`.
2. Updated `apply_config` to populate these fields from `Config`.
3. Updated `ConfigState::resolve_default_model` to mirror `Config::resolve_default_model`'s full fallback chain:
   - `provider` field → first model for that provider → `default_model` → first configured provider.
   - Mock provider check preserved.

## Acceptance criteria

- [x] All fallback branches covered by the single resolver.
- [x] Unit tests pass.
- [x] `cargo check --workspace` and `cargo test --workspace` pass.
