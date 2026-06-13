# Replace `update` Dispatcher Wildcard with Explicit No-Op List

**Status**: todo
**Milestone**: R3
**Category**: Core Architecture
**Priority**: P1

## Description

`crates/runie-core/src/update/mod.rs::update()` ends its main `match`
with `_ => {}`. This silently swallows any event that lacks a handler,
including:

- `MouseClick`, `MouseRelease`, `MouseDrag`, `MouseMove`
- `FocusGained`, `FocusLost`
- `AgentsManagerSetField { .. }`
- `SettingsSwitchCategory { .. }`
- `ScopedModelToggleProvider { .. }`
- `TransientMessage`, `TransientError` (already handled earlier but also
  reachable in the wildcard)

Silently dropping events makes it easy to add a variant, register it in
`Event::name()`, but forget to handle it. The dispatcher should either
handle the event or explicitly declare it as intentionally no-op.

## Acceptance Criteria

- [ ] The `_ => {}` fallback is removed from `AppState::update`.
- [ ] Every `Event` variant is matched exactly once at the top level.
- [ ] Events that are intentionally no-op (e.g. mouse motion, focus
  events) are listed in an explicit `match event { ... }` arm or a
  helper that documents the no-op decision.
- [ ] Events that currently have no handler but should have one
  (e.g. `AgentsManagerSetField`, `SettingsSwitchCategory`,
  `ScopedModelToggleProvider`) are routed to the correct module or
  marked with a `TODO` and a compile-time warning.
- [ ] `cargo build --workspace` succeeds.
- [ ] `cargo test --workspace` succeeds.

## Tests

### Layer 1 — State/Logic
- [ ] `event_dispatch_exhaustive` — after the change, adding a new
  `Event` variant without a match arm causes a compile error (the match
  is exhaustive).

### Layer 2 — Event Handling
- [ ] `mouse_click_is_noop` — `MouseClick` does not panic or modify
  state.
- [ ] `focus_events_are_noop` — `FocusGained`/`FocusLost` do not modify
  state.
- [ ] `agents_manager_set_field_routes` — `AgentsManagerSetField`
  updates pending edits.

### Layer 3 — Rendering
- [ ] No rendering changes.

### Layer 4 — Smoke
- [ ] `./dev.sh` starts and responds to input.

## Notes

**Strategy:**
1. Add an explicit `noop_events` arm for mouse/focus events.
2. Route `AgentsManagerSetField` to `agents_manager_event`.
3. Route `SettingsSwitchCategory` to `settings_dialog`.
4. Route `ScopedModelToggleProvider` to `scoped_models`.
5. Remove the wildcard.

**Out of scope:**
- Implementing actual mouse/focus behavior.
- Splitting the `Event` enum (see `event-subenums.md`).

## Verification

```bash
cargo build --workspace
cargo test --workspace
# Verify no wildcard in update/mod.rs
grep -n "_ => {}" crates/runie-core/src/update/mod.rs
```
