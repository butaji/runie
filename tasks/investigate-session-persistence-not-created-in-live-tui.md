# Investigate session persistence not created during live TUI runs

**Status**: todo
**Milestone**: R7
**Category**: Sessions
**Priority**: P2

**Depends on**: fix-tui-mock-simple-text-response-repetition
**Blocks**: none

## Description

During live `tmux` smoke tests, the `~/.runie/sessions/` directory was not created after a mock turn. Session persistence should create at least one session file on startup or after a completed turn.

## Acceptance Criteria

- [ ] Determine whether session persistence is triggered by `TurnComplete`, `/save`, or another event.
- [ ] Verify sessions are written after a successful mock turn in the TUI.
- [ ] `cargo test --workspace` passes.

## Tests

### Layer 4 — Provider Replay / Mock-Tool E2E
- [ ] `live_tui_creates_session_file` — after a completed mock turn, a session JSONL file exists.

## Files touched

- `crates/runie-core/src/actors/session/ractor_session_actor.rs`
- `crates/runie-core/src/session/persistence/`
- `crates/runie-tui/src/ui_actor.rs`

## Notes

- The missing session may be because the mock turn did not reach `TurnComplete` in the current tests.
