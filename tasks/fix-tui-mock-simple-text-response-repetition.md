# Fix TUI mock simple-text response repetition

**Status**: todo
**Milestone**: R7
**Category**: TUI / Rendering
**Priority**: P0

**Depends on**: reconnect-tui-agent-actor-channel
**Blocks**: live-tui-smoke-test-real-minimax, investigate-session-persistence-not-created-in-live-tui

## Description

During live `tmux` smoke tests with the mock provider, typing `hello` and pressing Enter causes the assistant response area to fill with repeated `hello` tokens and the status bar stays in `Working... (1 queued)` indefinitely. The headless CLI `runie-headless print hello` returns a single `hello ` and stops, so the bug is in the TUI/agent integration, not the mock provider itself.

## Live Evidence

```
  ❯ hello

  ◆ Thought 0.0s

  →  ◐ 0.0s

 hello hello hello hello hello hello hello hello hello hello hello hello hello
 hello hello hello hello hello hello hello hello hello hello hello hello hello
 ...

  ⠹ Working... 8.0s (1 queued)                        ↑0 ↓121.2k -/s 0%/128k ⛀
```

Captured stderr was empty because the release binary predated the temporary diagnostics; a fresh build with `eprintln!` instrumentation in `runie-tui`, `runie-agent`, and `RactorTurnActor` is needed to trace event flow.

## Acceptance Criteria

- [ ] A simple prompt (e.g. `hello`) in mock TUI mode renders a single, non-repeating echo response.
- [ ] The status bar returns to idle (`Type a message to start...`) within a few seconds.
- [ ] The turn queue is cleared after the turn completes.
- [ ] `cargo test --workspace` passes.
- [ ] `scripts/tmux-smoke-test.sh mock` passes for the `hello` scenario.

## Tests

### Layer 2 — Event Handling
- [ ] `mock_hello_events_not_repeated` — feed a `Submit` event and assert exactly one `TurnStarted`, one `ResponseDelta`, and one `Done`/`TurnComplete` sequence per user message.

### Layer 3 — Rendering
- [ ] `mock_hello_renders_single_echo` — use `TestBackend` to assert the assistant area contains `hello ` once and the status line leaves `Working`.

### Layer 4 — Provider Replay / Mock-Tool E2E
- [ ] `tmux_mock_hello_reaches_idle` — `scripts/tmux-smoke-test.sh mock` variant checks the captured pane for a single `hello` and an idle status.

## Files touched

- `crates/runie-tui/src/ui_actor.rs`
- `crates/runie-core/src/actors/turn/ractor_turn.rs`
- `crates/runie-agent/src/actor.rs`
- `crates/runie-agent/src/stream_response.rs`
- `scripts/tmux-smoke-test.sh`

## Validation

This task is not complete until the fix is validated with all three levels:

1. **Unit tests** — cover the state/logic change in isolation.
2. **E2E tests** — cover the event handling and/or provider-replay path.
3. **Live tmux tests** — `scripts/tmux-smoke-test.sh mock` (or the relevant scenario) passes in a real terminal.

## Notes

- A previous fix removed redundant `run_if_queued` calls from `UiActor::handle_event_inner`, but the live symptom still reproduces in the current binary. Root cause is not yet confirmed.
- Possible causes: `TurnActor` re-running the queued request on every `Done`, the agent actor re-emitting `ResponseDelta` in a loop, or `Event::Submit` being forwarded multiple times.
- Fixing this task is a prerequisite for meaningful multi-turn, session persistence, and MiniMax live tests.
