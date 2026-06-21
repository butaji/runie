# Delete duplicate `Reply` re-export in `actors/mod.rs`

**Status**: todo
**Milestone**: R4
**Category**: Architecture / Actors
**Priority**: P0

**Depends on**: none
**Blocks**: none

## Description

`crates/runie-core/src/actors/mod.rs` declares a local type `pub struct Reply<T>(...)` on line 27, then immediately re-exports it on line 50 with `pub use self::Reply;`. The re-import collides with the local definition and fails to compile under `cargo test --workspace`:

```
error[E0255]: the name `Reply` is defined multiple times
  --> crates/runie-core/src/actors/mod.rs:50:9
   |
27 | pub struct Reply<T>(Arc<Mutex<Option<oneshot::Sender<T>>>>);
   | ------------------------------------------------------------ previous definition of the type `Reply` here
...
50 | pub use self::Reply;
   |         ^^^^^^^^^^^ `Reply` reimported here
```

A library build succeeds because the re-export is only flagged as `unused_imports` outside test compilation, but `cargo test` turns it into a hard error. Drop the redundant re-export.

## Acceptance Criteria

- [ ] `pub use self::Reply;` removed from `crates/runie-core/src/actors/mod.rs:50`.
- [ ] `Reply<T>` still public (defined locally at line 27).
- [ ] No callers need updating (the type is only re-imported by the same module).
- [ ] `cargo build --workspace` succeeds.
- [ ] `cargo test --workspace` (after `arch-review-2026-06` P0 #1 also lands) succeeds.

## Tests

### Layer 1 — State/Logic
- N/A — type-system cleanup, no behavior.

### Layer 2 — Event Handling
- N/A.

### Layer 3 — Rendering
- N/A.

### Layer 4 — Smoke / Crash
- [ ] `build_workspace_compiles` — `cargo check --workspace --all-targets` exits 0.

## Files touched

- `crates/runie-core/src/actors/mod.rs`

## Notes

- Same class as `delete-tui-ipc-reexport-shim` (P0) and `delete-config-reload-shim` (P2). Trivial one-line fix; bundle with other dead-code removals if convenient.
