#!/bin/bash
# Smoke test: Rapid submit stress
set -e

export TMUX_TMPDIR=/tmp

# Ensure tmux server is running
tmux start-server 2>/dev/null || true

BINARY="$(pwd)/target/release/runie"
SESSION="runie_smoke_rapid_$$"
LOG="/tmp/runie_smoke_rapid_$$.log"

# Ensure binary exists
if [ ! -f "$BINARY" ]; then
    echo "Building release binary..."
    cargo build --release 2>/dev/null
fi

trap 'tmux kill-session -t "$SESSION" 2>/dev/null || true' EXIT

echo "Starting smoke test: rapid submit"
tmux new-session -d -s "$SESSION" -x 80 -y 24 "$BINARY"
sleep 0.5

# Rapid submit stress test
for i in $(seq 1 5); do
    MSG="message $i"
    tmux send-keys -t "$SESSION" "$MSG"
    tmux send-keys -t "$SESSION" Enter
    sleep 0.3
done

# Wait for processing
sleep 2.0

# Capture output
tmux capture-pane -t "$SESSION" -p > "$LOG"
tmux send-keys -t "$SESSION" C-c

# Check for panics
if grep -i "panic\|thread.*panicked" "$LOG"; then
    echo "FAIL: Panic detected during rapid submit"
    exit 1
fi

# Check for stuck timers (elapsed > 1000s would indicate infinite loop)
if grep -E '[0-9]{4}\.[0-9]s' "$LOG"; then
    echo "FAIL: Stuck timer detected"
    exit 1
fi

echo "PASS: smoke_rapid_submit"
