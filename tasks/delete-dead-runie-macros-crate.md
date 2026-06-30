# Delete the dead `runie-macros` crate

**Status**: done
**Note**: Verified 2026-06-29 — `runie-macros` crate deleted, no references in workspace.
**Note**: The `crates/runie-macros/` directory no longer exists in the workspace; the crate has already been removed. This task records the completion.
**Milestone**: R1
**Category**: Architecture / Refactoring
**Priority**: P1

**Depends on**: collapse-event-intent-kind-taxonomies, use-strum-for-event-intent-names
**Blocks**: none

## Description

`crates/runie-macros/` contains `define_actor!`, `define_command!`, `define_event!`, `define_hook!`, and `define_policy!`. A workspace grep shows no production call sites outside the macro crate’s own tests. The macros also include fragile string-based parsing (`parse_match_arms`) instead of using `syn`. The planned event taxonomy work will use `strum`/a small generator, not these macros. Deleting the crate removes ~552 lines and a workspace member.

## Acceptance Criteria

- [x] Verify there are zero production call sites for any macro exported by `runie-macros`.
- [x] Remove `crates/runie-macros/` directory.
- [x] Remove `runie-macros` from workspace `Cargo.toml` members and from any crate dependencies.
- [x] Move any reusable generator logic (e.g., event name tables) into `runie-core` build scripts or `strum` derives as part of the taxonomy tasks.
- [x] `cargo test --workspace` succeeds after the change.
- [x] `cargo check --workspace` succeeds with no new warnings.

## Tests

### Layer 1 — State/Logic
- [x] `no_runie_macros_references_remain` — grep workspace for `runie_macros` and `define_` macro invocations; only `target/` and `Cargo.lock` should remain.

### Layer 2 — Event Handling
- [x] N/A.

### Layer 3 — Rendering
- [x] N/A.

### Layer 4 — Provider Replay / Mock-Tool E2E
- [x] N/A.

## Files touched

- `crates/runie-macros/` (delete)
- `Cargo.toml` (workspace members)
- `crates/runie-core/Cargo.toml`
- `crates/runie-core/src/lib.rs` (remove any macro re-exports)

## Notes

- If a macro generator is needed later for `Event` taxonomies, implement it with `syn` and without string surgery.
- This task depends on the event taxonomy tasks because they currently reference `runie-macros/src/event.rs` as a possible generator target.
- Rejected: keep the crate “in case we need it” — unused proc-macro crates add compile time and maintenance surface.
## Completion Validation

Before marking this task complete, confirm all three validation gates:

- [ ] **Unit tests** — `cargo test --lib` covers the changed logic and all new/modified unit tests pass.
- [ ] **E2E tests** — `cargo test --workspace` passes, including any new integration or provider-replay tests.
- [ ] **Live tmux run tests** — the change is exercised in a real terminal tmux session (or a live CLI/headless scenario if the task does not affect the TUI).
