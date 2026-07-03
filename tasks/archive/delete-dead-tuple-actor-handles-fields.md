# Delete unused `ActorHandles` tuple struct fields

**Status**: done
**Milestone**: R4
**Category**: Architecture / Actors
**Priority**: P1

**Depends on**: none
**Blocks**: none

## Description

`crates/runie-tui/src/main.rs:95-101` declares:

```rust
struct ActorHandles(
    runie_core::actor::ActorHandle,
    runie_core::actor::ActorHandle,
    runie_core::actor::ActorHandle,
    runie_core::actor::ActorHandle,
    runie_core::actor::ActorHandle,
);
```

**N/A**: The `ActorHandles` struct described in this task does not exist in the current codebase. The struct may have been removed during a previous refactoring. No action needed.

## Acceptance Criteria

- [ ] Each field of `ActorHandles` is named `_0`..`_4` (or the struct is annotated with `#[allow(dead_code)]` referencing this task).
- [ ] `cargo check --workspace` reports zero `dead_code` warnings on `main.rs`.
- [ ] Bootstrap behavior unchanged: all five actors (config, provider, persistence, session store, io) remain alive for the lifetime of the TUI.

## Tests

### Layer 1 — State/Logic
- N/A.

### Layer 2 — Event Handling
- N/A.

### Layer 3 — Rendering
- N/A.

### Layer 4 — Smoke / Crash
- [ ] `cargo build --workspace` exits 0 with no new warnings.

## Files touched

- `crates/runie-tui/src/main.rs`

## Notes

- Trivial cleanup. Pair with `delete-dead-theme-async-loaders` and `delete-dead-history-action-vimnav` in a single "sweep dead_code" commit.
