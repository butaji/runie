# Collapse `runie-core/src/event/variants/` subdir into one file

**Status**: todo
**Milestone**: R4
**Category**: Core / State
**Priority**: P2

**Depends on**: none
**Blocks**: none

## Description

`crates/runie-core/src/event/variants.rs` (413 LOC, the canonical `Event` enum) declares three sub-files via `mod constructors; mod name; mod to_durable;`. Each sub-file is just an `impl Event { … }` block, totalling 316 LOC across three files:

| File | LOC | Production callers |
|------|-----|---------------------|
| `variants/constructors.rs` | 122 | tests only (`runie-tui/src/tests/`, `runie-core/src/tests/`) |
| `variants/name.rs` | 155 | tests only (`keybindings/tests.rs`, `variants_tests.rs`); one production read via `event::EVENT_NAMES` |
| `variants/to_durable.rs` | 39 | production (`session_actor.rs` calls `Event::to_durable`) |

The split forces every reader to follow three indirections to understand the `Event` API. None of the three sub-files shares any state or types — they could be merged into `variants.rs` (which would push it to ~730 LOC, over the 500 cap) **or** the test-only `constructors.rs` and `name.rs` can move into `variants_tests.rs` while `to_durable.rs` (the only production-relevant split) stays in a single flat `to_durable.rs` next to `variants.rs`.

## Acceptance Criteria

- [ ] `event/variants/` directory deleted.
- [ ] `to_durable()` impl moved to `event/to_durable.rs` (flat, sibling of `variants.rs`), `mod to_durable;` declared in `event/mod.rs`.
- [ ] Test-only constructor helpers (`Event::input`, `backspace`, `newline`, `submit`, `scroll_up`, etc.) moved to `event/variants_tests.rs` or kept in a flat `event/constructors.rs` gated `#[cfg(test)]`.
- [ ] `name()` / `from_name()` impl moved into `event/variants_tests.rs` (test-only) or into `event/names.rs` (where `EVENT_NAMES` already lives).
- [ ] `rg "event::variants::" crates/` returns zero hits (the sub-module path is gone; callers still resolve via `event::*`).
- [ ] `cargo check --workspace` succeeds with no new warnings.
- [ ] `cargo test --workspace` succeeds (keybinding round-trip and event-persistence tests still pass).

## Tests

### Layer 1 — State/Logic
- N/A — type relocation, no logic change.

### Layer 2 — Event Handling
- [ ] `event_from_name_still_round_trips` — `Event::from_name("Quit") == Some(Event::Quit)`; `Event::Quit.name() == Some("Quit")`.
- [ ] `event_to_durable_filters_transients` — `Event::ResponseDelta.to_durable() == None`; `Event::Response.to_durable()` is `Some(...)`.

### Layer 3 — Rendering
- N/A.

### Layer 4 — Smoke / Crash
- [ ] `smoke_event_module_compiles_flat` — `cargo check -p runie-core` green; no `event::variants` path remains.

## Files touched

- `crates/runie-core/src/event/variants.rs` (drop `mod` declarations)
- `crates/runie-core/src/event/mod.rs` (declare flat `to_durable`)
- `crates/runie-core/src/event/variants_tests.rs` (absorb constructor + name helpers)
- `crates/runie-core/src/event/variants/` (delete directory)

## Notes

`to_durable.rs` is the only file with a real production consumer (`session_actor.rs`); the other two are test-only conveniences. Keep the production-relevant split flat and consolidate test helpers into the existing `variants_tests.rs` to avoid one-file-per-impl inflation.
