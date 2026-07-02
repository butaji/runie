# Replace vacuous print mode test with replay test

## Status

`done`

## Context

`crates/runie-cli/src/print.rs` previously had an always-true assertion `result.is_err() || result.is_ok()` that required a real provider/config.

## Goal

Replace with a deterministic test using `MockProvider`/`HeadlessOptions`.

## Implementation

Replaced the vacuous test with:
1. `print_mode_emits_jsonl_events` — uses `MockProvider::default()` to emit a deterministic text turn; verifies at least one `Text` and one `End` event; validates JSONL round-trip for all emitted events.
2. `print_mode_run_smoke` — verifies `run()` doesn't panic (fire-and-forget smoke test).

No real provider needed.

## Acceptance Criteria

- [x] Delete vacuous test.
- [x] Add replay-based test using `MockProvider`/`HeadlessOptions`.
- [x] No real provider needed.
- [x] `cargo test --workspace` passes.

## Tests

### Layer 4 — E2E
- [x] `print_mode_emits_jsonl_events` — MockProvider text turn produces `HeadlessEvent::Text` and `HeadlessEvent::End`; all events round-trip through JSONL.
- [x] `print_mode_run_smoke` — `run("hello")` completes without panic.

## Files touched

- `crates/runie-cli/src/print.rs` — replaced vacuous test with proper `MockProvider`-based tests.

## Validation

- [x] **Unit tests** — `cargo test --workspace` passes.
- [x] **E2E tests** — `print_mode_emits_jsonl_events` and `print_mode_run_smoke` pass.
- [x] **Live tmux run tests** — N/A.
