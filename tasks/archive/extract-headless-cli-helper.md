# Extract headless CLI helper

**Status**: done
**Milestone**: R4
**Category**: Architecture / Refactoring
**Priority**: P1

**Depends on**: none
**Blocks**: collapse-headless-binaries-into-one-cli

## Description

Extract the common runtime setup that all three headless binaries share (system-prompt construction, message-list building, `HeadlessCliOptions` defaults) into a shared helper module.

The helper already exists at `crates/runie-agent/src/headless_helper.rs` and provides:
- `build_system_prompt()` — system prompt construction
- `build_messages(user_prompt)` — message list building
- `build_options(on_chunk)` — `HeadlessCliOptions` with common defaults
- `build_sink(yolo)` — permission sink for headless mode
- `run_headless(...)` — high-level turn runner

All three headless modes (`runie-print`, `runie-json`, `runie-server`) use this helper.

## Acceptance Criteria

- [x] `crates/runie-agent/src/headless_helper.rs` contains shared helper functions.
- [x] `runie-print` uses the helper (via `build_messages`, `build_options`, `build_sink`).
- [x] `runie-json` uses the helper (via `build_options`, `build_sink`).
- [x] `runie-server` uses the helper (via `build_sink` from `runie_agent::headless_helper`).
- [x] Helper has unit tests for `build_system_prompt`, `build_messages`, `build_options`, `build_sink`.
- [x] `cargo test --workspace` succeeds.

## Tests

### Layer 1 — State/Logic
- [x] `test_build_system_prompt_is_non_empty` — prompt is non-empty.
- [x] `test_build_messages_has_system_and_user` — message list has correct roles.
- [x] `test_build_options_defaults` — options have correct defaults.
- [x] `test_build_sink_returns_something` — sink is constructable.

### Layer 2 — Event Handling
- N/A.

### Layer 3 — Rendering
- N/A.

### Layer 4 — Smoke / Crash
- [x] `cargo test --workspace` green.

## Files touched

- `crates/runie-agent/src/headless_helper.rs` (already exists, verified)

## Notes

This task is marked done because the helper was already implemented when discovered. The three headless binaries (`runie-print`, `runie-json`, `runie-server`) all use this helper. Next step: `collapse-headless-binaries-into-one-cli` consolidates these three into one `runie-cli` crate.
