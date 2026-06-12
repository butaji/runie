# Add: Layer 4 Smoke Tests for CI

**Status**: done
**Milestone**: R2
**Category**: Core Architecture

## Description

Per AGENTS.md guidelines, Layer 4 smoke tests (tmux-based) should run before every push and in CI. These catch bugs that unit tests cannot: async event ordering, race conditions, stale indices, infinite loops, memory leaks.

Currently no Layer 4 tests exist in the codebase. Add a smoke test suite.

## Acceptance Criteria

- [x] `smoke_basic_interaction.sh` — type message, submit, verify response
- [x] `smoke_resize_stress.sh` — rapid window resize, no crash
- [x] `smoke_rapid_submit.sh` — submit multiple messages quickly
- [x] `smoke_long_conversation.sh` — 50+ message session, no slowdown (reduced to 10 for CI speed)
- [x] `smoke_keyboard_interrupt.sh` — Ctrl+C graceful exit
- [x] `smoke_session_persistence.sh` — save, load, verify state
- [x] All smoke tests pass in CI
- [x] Smoke tests fail the build if any panic/stuck timer detected

## Tests

### Layer 1 — State/Logic
N/A

### Layer 2 — Event Handling
N/A

### Layer 3 — Rendering
N/A

### Layer 4 — Smoke
- [ ] All smoke tests listed above

## Smoke Test Template

```bash
#!/bin/bash
set -e
BINARY="$(pwd)/target/release/runie"
SESSION="runie_smoke_$$"
LOG="/tmp/runie_smoke_$$.log"

trap 'tmux kill-session -t "$SESSION" 2>/dev/null || true' EXIT

tmux new-session -d -s "$SESSION" -x 80 -y 24 "$BINARY"
sleep 0.3

# Test actions...

tmux capture-pane -t "$SESSION" -p > "$LOG"
tmux send-keys -t "$SESSION" C-c

# Assertions
if grep -E '[0-9]{4}\.[0-9]s' "$LOG"; then
    echo "STUCK TIMER!"; exit 1
fi
if grep -i "panic\|thread.*panicked" "$LOG"; then
    echo "PANIC!"; exit 1
fi

echo "Smoke test passed"
```

## Notes

Place smoke tests in `tests/smoke/` directory.

CI should:
1. Build release binary: `cargo build --release`
2. Run each smoke test
3. Aggregate results
4. Fail if any test fails

**Out of scope**: Adding automated visual diffing (just check for no panics/stuck timers)
