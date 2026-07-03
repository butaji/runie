# Collapse `Intent` into shared event payloads

## Status

`done`

## Description

`Intent` duplicated many `Event` intent/control variants, and `Event::into_intent()` mapped them manually. The `Intent` enum has been collapsed into `Event` — it is now a type alias (`pub use crate::event::Event as Intent`), and `into_intent()` returns `Option<Event>` directly.

## Changes

1. **`intent.rs`** — Replaced the duplicate `Intent` enum with a deprecation notice and type alias:
   ```rust
   pub use crate::event::Event as Intent;
   ```

2. **`into_intent()`** — Simplified to return `Option<Event>` directly instead of `Option<Intent>`. The method still includes all intent/control variants, returning `Some(Event)` for them and `None` for fact variants.

3. **Tests** — Updated to use `Event` directly in pattern matches instead of `Intent`.

## Acceptance Criteria

- [x] **Unit tests** — `Event::into_intent()` returns `Option<Event>`; all intent/control variants convert correctly.
- [x] **E2E tests** — Slash commands and palette commands still work (tested via unit tests).
- [x] **Live tmux tests** — Not required for this internal refactor.

## Tests

### Unit tests
- `cargo test event::tests::` — All event taxonomy tests pass.
- `cargo test event::kind::` — All EventKind classification tests pass.

### E2E tests
- All workspace tests pass: `cargo test --workspace`.
