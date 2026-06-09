#!/bin/bash
# Smoke test: Session tree operations (create, fork, navigate)
set -e

export TMUX_TMPDIR=/tmp

# Ensure tmux server is running
tmux start-server 2>/dev/null || true

BINARY="$(pwd)/target/release/runie"
SESSION="runie_smoke_tree_$$"
LOG="/tmp/runie_smoke_tree_$$.log"

# Ensure binary exists
if [ ! -f "$BINARY" ]; then
    echo "Building release binary..."
    cargo build --release 2>/dev/null
fi

trap 'tmux kill-session -t "$SESSION" 2>/dev/null || true' EXIT

echo "Starting smoke test: session tree operations"
tmux new-session -d -s "$SESSION" -x 80 -y 24 "$BINARY"
sleep 0.5

# Type messages to build a conversation
tmux send-keys -t "$SESSION" "message one"
tmux send-keys -t "$SESSION" Enter
sleep 1.5

tmux send-keys -t "$SESSION" "message two"
tmux send-keys -t "$SESSION" Enter
sleep 1.5

# Open tree dialog
tmux send-keys -t "$SESSION" "/tree"
tmux send-keys -t "$SESSION" Enter
sleep 0.5

# Cycle filter
tmux send-keys -t "$SESSION" "f"
sleep 0.3

# Close dialog with Escape
tmux send-keys -t "$SESSION" Escape
sleep 0.3

# Fork at first message
tmux send-keys -t "$SESSION" "/fork 1"
tmux send-keys -t "$SESSION" Enter
sleep 1.0

# Capture output
tmux capture-pane -t "$SESSION" -p > "$LOG"
tmux send-keys -t "$SESSION" C-c

# Check for panics
if grep -i "panic\|thread.*panicked" "$LOG" 2>/dev/null; then
    echo "FAIL: Panic detected during session tree operations"
    exit 1
fi

# Check for stuck timers
if grep -E '[0-9]{4}\.[0-9]s' "$LOG" 2>/dev/null; then
    echo "FAIL: Stuck timer detected"
    exit 1
fi

echo "PASS: smoke_session_tree_operations"
