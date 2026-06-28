# Delete the dead `runie-macros` crate

**Status**: todo
**Milestone**: R1
**Category**: Architecture / Refactoring
**Priority**: P1

**Depends on**: collapse-event-intent-kind-taxonomies, use-strum-for-event-intent-names
**Blocks**: none

## Description

`crates/runie-macros/` contains `define_actor!`, `define_command!`, `define_event!`, `define_hook!`, and `define_policy!`. A workspace grep shows no production call sites outside the macro crate’s own tests. The macros also include fragile string-based parsing (`parse_match_arms`) instead of using `syn`. The planned event taxonomy work will use `strum`/a small generator, not these macros. Deleting the crate removes ~552 lines and a workspace member.

## Acceptance Criteria

- [ ] Verify there are zero production call sites for any macro exported by `runie-macros`.
- [ ] Remove `crates/runie-macros/` directory.
- [ ] Remove `runie-macros` from workspace `Cargo.toml` members and from any crate dependencies.
- [ ] Move any reusable generator logic (e.g., event name tables) into `runie-core` build scripts or `strum` derives as part of the taxonomy tasks.
- [ ] `cargo test --workspace` succeeds after the change.
- [ ] `cargo check --workspace` succeeds with no new warnings.

## Tests

### Layer 1 — State/Logic
- [ ] `no_runie_macros_references_remain` — grep workspace for `runie_macros` and `define_` macro invocations; only `target/` and `Cargo.lock` should remain.

### Layer 2 — Event Handling
- [ ] N/A.

### Layer 3 — Rendering
- [ ] N/A.

### Layer 4 — Provider Replay / Mock-Tool E2E
- [ ] N/A.

## Files touched

- `crates/runie-macros/` (delete)
- `Cargo.toml` (workspace members)
- `crates/runie-core/Cargo.toml`
- `crates/runie-core/src/lib.rs` (remove any macro re-exports)

## Notes

- If a macro generator is needed later for `Event` taxonomies, implement it with `syn` and without string surgery.
- This task depends on the event taxonomy tasks because they currently reference `runie-macros/src/event.rs` as a possible generator target.
- Rejected: keep the crate “in case we need it” — unused proc-macro crates add compile time and maintenance surface.
