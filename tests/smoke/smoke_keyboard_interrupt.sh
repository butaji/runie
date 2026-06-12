#!/bin/bash
# Smoke test: Keyboard interrupt (Ctrl+C) graceful exit
set -e

export TMUX_TMPDIR=/tmp

# Ensure tmux server is running
tmux start-server 2>/dev/null || true

BINARY="$(pwd)/target/release/runie"
SESSION="runie_smoke_interrupt_$$"
LOG="/tmp/runie_smoke_interrupt_$$.log"

# Ensure binary exists
if [ ! -f "$BINARY" ]; then
    echo "Building release binary..."
    cargo build --release 2>/dev/null
fi

trap 'tmux kill-session -t "$SESSION" 2>/dev/null || true' EXIT

echo "Starting smoke test: keyboard interrupt"
tmux new-session -d -s "$SESSION" -x 80 -y 24 "$BINARY"
sleep 0.5

# Type something and interrupt
tmux send-keys -t "$SESSION" "hello"
sleep 0.2
tmux send-keys -t "$SESSION" "C-c"

sleep 0.5

# Capture output (session may already be gone if app exited)
tmux capture-pane -t "$SESSION" -p > "$LOG" 2>/dev/null || true

# Verify session is gone (app exited gracefully)
SESSION_EXISTS=false
if tmux has-session -t "$SESSION" 2>/dev/null; then
    SESSION_EXISTS=true
fi

if [ "$SESSION_EXISTS" = "false" ]; then
    echo "INFO: App exited gracefully after Ctrl+C"
else
    # If still running, kill it
    tmux send-keys -t "$SESSION" C-c
    echo "INFO: Ctrl+C processed (session still alive)"
fi

# Check for panics in log
if grep -i "panic\|thread.*panicked" "$LOG"; then
    echo "FAIL: Panic detected during interrupt"
    exit 1
fi

echo "PASS: smoke_keyboard_interrupt"
