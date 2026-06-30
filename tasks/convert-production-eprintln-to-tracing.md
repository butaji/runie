# Convert production `eprintln!` to `tracing`

**Status**: done
**Milestone**: R7

**Note**: Verified 2026-06-29 — `keybindings/mod.rs` has no `eprintln!`; `embedded_commands.rs` now uses `tracing::warn!` instead.
**Category**: Observability
**Priority**: P2

**Depends on**: initialize-tracing-subscriber-in-binaries
**Blocks**: none

## Description

Production code uses `eprintln!` for warnings in `keybindings/mod.rs` and `commands/dsl/embedded_commands.rs`. Convert them to `tracing::warn!` / `tracing::error!`.

## Acceptance Criteria

- [x] Replace production `eprintln!` calls with `tracing` events.
- [x] `cargo check --workspace` and `cargo test --workspace` pass.

## Tests

### Layer 2 — Event Handling
- [x] `keybinding_warning_emits_tracing_event` — a test subscriber captures the warning.

## Files touched

- `crates/runie-core/src/keybindings/mod.rs`
- `crates/runie-core/src/commands/dsl/embedded_commands.rs`

## Notes

- Tests are allowed to use `eprintln!` if they are not part of the production path.
