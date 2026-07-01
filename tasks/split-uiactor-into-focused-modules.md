# Split `ui_actor.rs` into focused modules

## Status

`todo`

## Description

`crates/runie-tui/src/ui_actor.rs` is 794 lines and mixes event routing, input dispatch, effect dispatch, animation, autocomplete, and submit logic.

## Acceptance criteria

- Split into modules such as `ui_actor/input.rs`, `ui_actor/effects.rs`, `ui_actor/submit.rs`, `ui_actor/animation.rs`.
- No module exceeds 500 lines.
- Public API of `UiActor` remains unchanged.

## Tests

### Layer 2 — Event Handling
- Key/submit/autocomplete events still route correctly.

### Layer 3 — Rendering
- `TestBackend` snapshots match before and after the split.
