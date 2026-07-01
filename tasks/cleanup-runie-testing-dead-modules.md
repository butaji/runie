# Cleanup runie-testing dead modules

## Status

`done`

## Context

`runie-testing/src/timeout.rs` and `events.rs` are unused; `macros.rs` is misnamed.

## Goal

Delete or adopt dead modules; rename `macros.rs` to `conditional.rs`.

## Acceptance Criteria
- [x] Delete `timeout.rs` and its tests. — **Already gone**; `timeout.rs` does not exist in the crate.
- [x] Delete or adopt `events.rs` builders. — **Already adopted**; `events.rs` is used internally by `event_helpers.rs` for building mock events in tests.
- [x] Rename `macros.rs` and update `lib.rs`. — **Already done**; `conditional.rs` exists and is exported in `lib.rs`.

## Design Impact

No change to TUI element design or composition unless explicitly noted. Only implementation behavior, dependency graph, or internal architecture changes.

## Tests

- **Layer 4 — E2E:** `cargo test -p runie-testing` passes (21 tests).
- **Live tmux validation:** N/A.

## Completion Validation

- [x] **Unit tests** — `cargo test --lib` covers the changed logic and all new/modified unit tests pass.
- [x] **E2E tests** — `cargo test --workspace` passes.

## Implementation Notes

Inspection of `crates/runie-testing/src/` on 2026-07-01 confirmed:
- `timeout.rs` does not exist in the crate (already deleted).
- `events.rs` is used by `event_helpers.rs:52` — functions `ev_completed`, `ev_output_text_delta`, `ev_response_created` are imported and used in `count_events` tests. The module is also re-exported in `lib.rs` for potential external use.
- `conditional.rs` exists (renamed from `macros.rs`) and is exported in `lib.rs`.
- `cargo test -p runie-testing`: 21 passed, 0 failed.
