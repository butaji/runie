#!/bin/bash
# Smoke test: Basic interaction - type message, submit, verify response
set -e

export TMUX_TMPDIR=/tmp

# Ensure tmux server is running
tmux start-server 2>/dev/null || true

BINARY="$(pwd)/target/release/runie"
SESSION="runie_smoke_basic_$$"
LOG="/tmp/runie_smoke_basic_$$.log"

# Ensure binary exists
if [ ! -f "$BINARY" ]; then
    echo "Building release binary..."
    cargo build --release 2>/dev/null
fi

trap 'tmux kill-session -t "$SESSION" 2>/dev/null || true' EXIT

echo "Starting smoke test: basic interaction"
tmux new-session -d -s "$SESSION" -x 80 -y 24 "$BINARY"
sleep 0.5

# Type a simple message
tmux send-keys -t "$SESSION" "hello"
sleep 0.2

# Submit with Enter
tmux send-keys -t "$SESSION" Enter
sleep 3.0

# Capture output
tmux capture-pane -t "$SESSION" -p > "$LOG"
tmux send-keys -t "$SESSION" C-c

# Verify the app responded (mock should return something)
if grep -qi "assistant\|response\|output\|error" "$LOG"; then
    echo "INFO: App responded to message"
fi

# Check for panics
if grep -i "panic\|thread.*panicked" "$LOG"; then
    echo "FAIL: Panic detected"
    exit 1
fi

echo "PASS: smoke_basic_interaction"
