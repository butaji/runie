#!/bin/bash
# Smoke test: Rapid window resize stress
set -e

export TMUX_TMPDIR=/tmp

# Ensure tmux server is running
tmux start-server 2>/dev/null || true

BINARY="$(pwd)/target/release/runie"
SESSION="runie_smoke_resize_$$"
LOG="/tmp/runie_smoke_resize_$$.log"

# Ensure binary exists
if [ ! -f "$BINARY" ]; then
    echo "Building release binary..."
    cargo build --release 2>/dev/null
fi

trap 'tmux kill-session -t "$SESSION" 2>/dev/null || true' EXIT

echo "Starting smoke test: resize stress"
tmux new-session -d -s "$SESSION" -x 80 -y 24 "$BINARY"
sleep 0.5

# Rapid resize stress test
for i in $(seq 1 20); do
    W=$((40 + i * 2))
    H=$((10 + i))
    tmux resize-window -t "$SESSION" -x "$W" -y "$H" 2>/dev/null || true
    sleep 0.05
done

# Final resize
tmux resize-window -t "$SESSION" -x 120 -y 30 2>/dev/null || true
sleep 0.3

# Capture output
tmux capture-pane -t "$SESSION" -p > "$LOG"
tmux send-keys -t "$SESSION" C-c

# Check for panics
if grep -i "panic\|thread.*panicked" "$LOG"; then
    echo "FAIL: Panic detected during resize"
    exit 1
fi

echo "PASS: smoke_resize_stress"
