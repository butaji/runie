# Delete async_io.rs bridge helpers

**Status**: todo
**Milestone**: R4
**Category**: Architecture / Actors
**Priority**: P1

**Depends on**: finish-io-migration
**Blocks**: none

## Description

`crates/runie-core/src/async_io.rs` provides two "tactical bridge" helpers:

- `run_blocking_if_runtime` — fire-and-forget blocking work on a Tokio blocking thread.
- `block_in_place_if_runtime` — run a short blocking closure off the async runtime and return the result.

The module doc explicitly states: "These helpers are a tactical bridge, not the preferred pattern" and "New code should default to async or event-based actors." They exist because sync IO remains in the domain crate (see `finish-io-migration`). Once all sync IO is behind actor traits, these helpers have no callers and can be deleted. Keeping them invites new sync-IO call sites.

## Acceptance Criteria

- [ ] `crates/runie-core/src/async_io.rs` deleted.
- [ ] `pub mod async_io;` removed from `lib.rs`.
- [ ] `rg "async_io|run_blocking_if_runtime|block_in_place_if_runtime" crates/` returns zero hits.
- [ ] `arch_guardrails.rs` no longer references `async_io` in any allow-list.
- [ ] `cargo test --workspace` succeeds.
- [ ] `cargo check --workspace` succeeds with no new warnings.

## Tests

### Layer 1 — State/Logic
- [ ] `no_async_io_imports_remain` — grep assertion: zero references to `async_io` in `crates/`.

### Layer 2 — Event Handling
- N/A — helper deletion, no event flow.

### Layer 3 — Rendering
- N/A.

### Layer 4 — Smoke / Crash
- [ ] `smoke_workspace_builds_without_async_io` — `cargo check --workspace` green; no call site regressed to sync IO.

## Files touched

- `crates/runie-core/src/async_io.rs` → delete
- `crates/runie-core/src/lib.rs` — remove `pub mod async_io;`
- Any remaining callers (should be zero after `finish-io-migration`; if not, convert them to actor messages first)
- `crates/runie-core/tests/arch_guardrails.rs` — remove `async_io` references

## Notes

Strictly depends on `finish-io-migration`: if any sync IO remains in the domain crate, deleting these helpers will break the build. Verify with `rg "block_in_place_if_runtime|run_blocking_if_runtime" crates/ --glob '!async_io.rs'` before deletion — if hits remain, they are missed `finish-io-migration` call sites, not reasons to keep the helpers. Do not keep dead bridge code "just in case"; the actor trait pattern is the documented preferred path.
