# Offload synchronous custom prompt file read

**Status**: done
**Milestone**: R4
**Category**: Core / State
**Priority**: P1
**Depends on**: none
**Blocks**: none
**Completed in**: current

## Description

`crate::prompts::load_prompts` called `std::fs::read_to_string` directly. The function is invoked from `AppState::apply_config`, which runs inside `UiActor::handle_event`, so a slow filesystem blocks the UI event loop.

## Acceptance Criteria

- [x] Wrap the custom prompt file read with `crate::async_io::block_in_place_if_runtime`.
- [x] Existing prompt-loading behavior is unchanged.
- [x] `cargo check -p runie-core` succeeds.
- [x] `cargo test -p runie-core --lib` succeeds.

## Tests

- [x] Layer 1 State/Logic: `prompt_loaded_from_file` test in `crates/runie-core/src/prompts.rs` still passes.
- [x] Layer 4 Smoke: `cargo test -p runie-core --lib` passes.

## Files touched

- `crates/runie-core/src/prompts.rs`

## Notes

Minimal change; no behavioral differences beyond avoiding blocking the async runtime.
