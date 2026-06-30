# Live TUI Smoke Test Report

**Date:** 2026-06-30
**Method:** `tmux` sessions with the release TUI binary
**Provider modes tested:** mock (`RUNIE_MOCK=1`)
**Provider modes pending:** real MiniMax (requires `MINIMAX_API_KEY`)

## Summary

The TUI was broken in several places that prevented even basic live interaction. While trying to run `tmux` smoke tests, the following critical bugs were found and fixed:

| Bug | Location | Fix |
|-----|----------|-----|
| `UiActor::run` was never spawned | `crates/runie-tui/src/main.rs` | Spawn `ui_actor.run(...)` after taking the render receiver. |
| `Event::Submit` was dropped by the input forwarder | `crates/runie-tui/src/main.rs` | Forward `Submit` on a dedicated channel to `UiActor` so it can capture input content before `InputActor` clears it. |
| `UiActor::handle_at_trigger` panicked on empty input | `crates/runie-tui/src/ui_actor.rs` | Return early when input is empty or whitespace. |
| `TurnActor` emitted `TurnStarted` but never triggered the agent | `crates/runie-core/src/actors/turn/ractor_turn.rs` | Call `handle_run_if_queued` after `SubmitUserMessage`; `UiActor` now dispatches `agent_handle.run(...)` on `TurnStarted`. |

After these fixes the TUI launches, accepts input, starts a turn, and renders provider responses in `tmux`.

## Test script

`scripts/tmux-smoke-test.sh [mock|minimax]`

```bash
# Run mock scenarios
scripts/tmux-smoke-test.sh mock

# Run a real MiniMax scenario (requires key)
MINIMAX_API_KEY=... scripts/tmux-smoke-test.sh minimax
```

The script creates an isolated `$HOME` with a minimal `~/.runie/config.toml`, starts the TUI in an 80×24 tmux pane, types a prompt, waits for the expected screen content, and then quits.

## Mock results

| Scenario | Expected | Result |
|----------|----------|--------|
| `launch` | "Type a message to start" | ✅ PASS |
| `hello` | "Working" (turn starts) | ✅ PASS |
| `list_files` | "Working" (tool-marker turn starts) | ✅ PASS |
| `native_tool` | "Run bash" (permission dialog appears) | ✅ PASS |

## Known remaining issues

1. **Simple text responses can render repetitively.** With the mock provider and the prompt "hello", the assistant response area fills with many repeated "hello" tokens and the turn stays in the "Working..." state. This may be a mock-provider quirk (it echoes the user input word-by-word) or a rendering/loop issue, and needs further investigation.
2. **No session persistence observed during live runs.** The `~/.runie/sessions/` directory was not created after a mock run that did not reach `TurnComplete`.
3. **Real MiniMax not tested.** No `MINIMAX_API_KEY` is available in the environment; the smoke test script skips real-provider scenarios unless the key is provided.

## Verification commands

```bash
# Unit / E2E tests (deterministic, no tmux)
cargo test --workspace

# Release TUI
cargo build --release -p runie-tui

# Mock tmux smoke tests
scripts/tmux-smoke-test.sh mock
```

All workspace tests pass. The mock tmux smoke tests pass.
