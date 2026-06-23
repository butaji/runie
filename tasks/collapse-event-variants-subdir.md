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
| `variants/constructors.rs` | 122 | **cross-crate tests** (`runie-tui/src/tests/` and `runie-core/src/tests/` both call `Event::submit()`, `Event::input()`, etc.) |
| `variants/name.rs` | 155 | **production** (`keybindings/mod.rs:101` `event_from_name` calls `Event::from_name`); tests (`variants_tests.rs`) call `Event::name()` |
| `variants/to_durable.rs` | 39 | production (`session_actor.rs:96,205` calls `Event::to_durable()`) |

The split forces every reader to follow three indirections to understand the `Event` API. None of the three sub-files shares state or types — they could be merged into `variants.rs` (which would push it to ~730 LOC, over the 500-line limit) **or** all three can move to flat sibling files next to `variants.rs`.

## Acceptance Criteria

- [ ] `event/variants/` directory deleted.
- [ ] `to_durable()` impl moved to `event/to_durable.rs` (flat, sibling of `variants.rs`), `mod to_durable;` declared in `event/mod.rs`.
- [ ] Constructor helpers (`Event::input`, `backspace`, `newline`, `submit`, `scroll_up`, etc.) moved to `event/constructors.rs` (flat, sibling of `variants.rs`, **must remain `pub` because `runie-tui`'s tests call them cross-crate**). NOT `#[cfg(test)]` gated.
- [ ] `name()` / `from_name()` impl moved to `event/name.rs` (flat, sibling of `variants.rs`, must remain `pub` — `keybindings::event_from_name` is a production caller).
- [ ] `rg "event::variants::" crates/` returns zero hits (the sub-module path is gone; callers still resolve via `event::*`).
- [ ] `rg "Event::submit\b|Event::input\b|Event::from_name\b|Event::to_durable\b" crates/` returns the same hits as before the refactor (cross-crate visibility preserved).
- [ ] `cargo check --workspace` succeeds with no new warnings.
- [ ] `cargo test --workspace` succeeds (keybinding round-trip and event-persistence tests still pass; all `Event::submit()` / `Event::input()` callers in `runie-tui/src/tests/` still resolve).

## Tests

### Layer 1 — State/Logic
- N/A — type relocation, no logic change.

### Layer 2 — Event Handling
- [ ] `event_from_name_still_round_trips` — `Event::from_name("Quit") == Some(Event::Quit)`; `Event::Quit.name() == Some("Quit")`.
- [ ] `event_to_durable_filters_transients` — `Event::ResponseDelta.to_durable() == None`; `Event::Response.to_durable()` is `Some(...)`.
- [ ] `event_submit_constructor_cross_crate` — `runie_core::Event::submit()` is callable from `runie-tui`'s test suite (verifies `pub` visibility preserved).

### Layer 3 — Rendering
- N/A.

### Layer 4 — Smoke / Crash
- [ ] `smoke_event_module_compiles_flat` — `cargo check -p runie-core` green; no `event::variants` path remains; `cargo test -p runie-tui` green (cross-crate `Event::submit()` usage still compiles).

## Files touched

- `crates/runie-core/src/event/variants.rs` (drop `mod` declarations)
- `crates/runie-core/src/event/mod.rs` (declare flat `to_durable`, `constructors`, `name`)
- `crates/runie-core/src/event/variants/` (delete directory)
- new: `crates/runie-core/src/event/to_durable.rs`
- new: `crates/runie-core/src/event/constructors.rs`
- new: `crates/runie-core/src/event/name.rs`

## Notes

All three sub-files have production callers (directly or indirectly through cross-crate tests), so none of them can be `#[cfg(test)]`-gated or moved into `variants_tests.rs`. The flat split keeps each impl discoverable next to the `Event` enum without forcing a 730-LOC `variants.rs`.
