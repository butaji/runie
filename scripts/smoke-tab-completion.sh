#!/bin/bash
# Smoke test for tab completion feature - Layer 4
# Tests: binary starts, tab key works, no panics

set -e
BINARY="$(pwd)/target/release/runie"
SESSION="runie_tab_$$"
LOG="/tmp/runie_tab_$$.log"

trap 'tmux kill-session -t "$SESSION" 2>/dev/null || true' EXIT

# Build if needed
if [ ! -f "$BINARY" ]; then
    echo "[smoke] Building release binary..."
    cargo build --release -p runie 2>/dev/null
fi

echo "[smoke] Starting tmux session..."
tmux new-session -d -s "$SESSION" -x 80 -y 24 "$BINARY"
sleep 0.3

echo "[smoke] Sending test input..."

# Type text and press Tab
tmux send-keys -t "$SESSION" "test"
tmux send-keys -t "$SESSION" Tab
sleep 0.2

# Cycle with Tab again
tmux send-keys -t "$SESSION" Tab
sleep 0.2

# Submit
tmux send-keys -t "$SESSION" Enter
sleep 0.5

# Quick resize stress
for i in $(seq 1 5); do
    tmux resize-window -t "$SESSION" -x $((60 + i * 4)) -y $((20 + i))
    sleep 0.05
done

# Capture output
echo "[smoke] Capturing output..."
tmux capture-pane -t "$SESSION" -p > "$LOG"
tmux send-keys -t "$SESSION" C-c
sleep 0.3

# Check for panics
echo "[smoke] Checking for issues..."
if grep -iE "panic|thread.*panicked|out of memory|segmentation fault" "$LOG" 2>/dev/null; then
    echo "FAIL: Panic or crash detected!"
    cat "$LOG"
    exit 1
fi

echo "[smoke] SUCCESS - No panics detected"
echo "--- Last 20 lines of log ---"
tail -20 "$LOG"
