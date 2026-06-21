# Delete unused `ActorHandles` tuple struct fields

**Status**: todo
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

The struct is constructed once at line 123 inside `bootstrap_app` and bound to `_actors` at line 74. The five fields are never read; only the struct itself is kept alive so the underlying tasks are not dropped (each `ActorHandle`'s `Drop` impl aborts its task). This is the correct lifetime pattern, but the fields don't need to be named — `_0`..`_5` work, or `#[allow(dead_code)]` with a justifying comment.

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
