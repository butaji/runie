# Inject provider factory into headless CLI and runtime

## Status

`done`

## Context

`runie_agent::headless_helper::run_headless` and `run_headless_cli` hard-coded `BuiltProviderFactory`; `HeadlessRuntime::spawn` had no seam for replay providers.

## Goal

Accept `Arc<dyn ProviderFactory>` in headless CLI/runtime so Grok fixtures can be replayed.

## Acceptance Criteria

- [x] Add factory parameter to headless functions.
- [x] Default to `BuiltProviderFactory` for production.
- [x] Update CLI callers.

## Changes Made

- Modified `run_headless_cli` in `crates/runie-agent/src/headless/mod.rs` to accept an optional `factory` parameter of type `Option<Arc<dyn ProviderFactory>>`
- When `factory` is `None`, defaults to `BuiltProviderFactory::new()`
- Updated all CLI callers (`server.rs`, `json.rs`, `print.rs`) to pass `None` for the factory
- Updated `headless_helper.rs` to pass `None` for the factory
- Updated test in `headless/tests.rs` to pass `None` for the factory

## Files Changed

- `crates/runie-agent/src/headless/mod.rs` - Added optional factory parameter
- `crates/runie-agent/src/headless/tests.rs` - Updated test
- `crates/runie-agent/src/headless_helper.rs` - Updated caller
- `crates/runie-cli/src/json.rs` - Updated caller
- `crates/runie-cli/src/print.rs` - Updated caller
- `crates/runie-cli/src/server.rs` - Updated callers

## Tests

- **Layer 4 — E2E:** All existing headless tests pass.
- **Live CLI testing:** Headless CLI with real provider works.

## Completion Validation

- [x] `cargo test --workspace` passes (3,084 tests).
- [x] `cargo check --workspace` passes with no errors.
