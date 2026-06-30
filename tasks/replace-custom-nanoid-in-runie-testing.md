# Replace custom nanoid in runie-testing

## Status

`todo`

## Context

`crates/runie-testing/src/runner.rs:116-119` truncates a UUID v4 to 8 characters and calls it `nanoid()`. This is misleading and unnecessary.

## Goal

Replace the custom function with `uuid::Uuid::new_v4()` at the call site, or add the `nanoid` crate if nanoid-format IDs are genuinely required.

**Design impact:** No change to TUI element design or composition. Only internal test ID generation changes.

## Acceptance Criteria

- [ ] Delete the custom `nanoid()` helper.
- [ ] Use `uuid::Uuid::new_v4()` or the `nanoid` crate.
- [ ] Keep IDs unique and suitable for test session/run names.

## Tests

- **Layer 1 — State/Logic:** Unit test that generated IDs are unique across many calls.
- **Layer 2 — Event Handling:** N/A.
- **Layer 3 — Rendering:** N/A.
- **Layer 4 — E2E:** Test runner creates sessions without collisions.
- **Live tmux validation:** N/A.
