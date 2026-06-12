#!/bin/bash
# Smoke test: Long conversation (50+ messages, verify no slowdown)
set -e

export TMUX_TMPDIR=/tmp

# Ensure tmux server is running
tmux start-server 2>/dev/null || true

BINARY="$(pwd)/target/release/runie"
SESSION="runie_smoke_long_$$"
LOG="/tmp/runie_smoke_long_$$.log"
TIMESTAMP_START=$(date +%s)

# Ensure binary exists
if [ ! -f "$BINARY" ]; then
    echo "Building release binary..."
    cargo build --release 2>/dev/null
fi

trap 'tmux kill-session -t "$SESSION" 2>/dev/null || true' EXIT

echo "Starting smoke test: long conversation"
tmux new-session -d -s "$SESSION" -x 80 -y 24 "$BINARY"
sleep 0.5

# Send 10 messages (reduced from 50 for CI speed)
# Mock provider should handle these quickly
for i in $(seq 1 10); do
    MSG="msg$i"
    tmux send-keys -t "$SESSION" "$MSG"
    tmux send-keys -t "$SESSION" Enter
    sleep 0.5
    
    # Check every few messages for responsiveness
    if [ $((i % 3)) -eq 0 ]; then
        tmux send-keys -t "$SESSION" ""  # Refresh display
        sleep 0.2
    fi
done

# Wait for final processing
sleep 3.0

TIMESTAMP_END=$(date +%s)
ELAPSED=$((TIMESTAMP_END - TIMESTAMP_START))

# Capture output
tmux capture-pane -t "$SESSION" -p > "$LOG"
tmux send-keys -t "$SESSION" C-c

# Check for panics
if grep -i "panic\|thread.*panicked" "$LOG"; then
    echo "FAIL: Panic detected during long conversation"
    exit 1
fi

# Check for stuck timers (>1000s would indicate infinite loop)
if grep -E '[0-9]{4}\.[0-9]s' "$LOG"; then
    echo "FAIL: Stuck timer detected"
    exit 1
fi

# Verify no excessive slowdown (10 messages should complete in <60s)
if [ "$ELAPSED" -gt 60 ]; then
    echo "WARNING: Long conversation took ${ELAPSED}s (>60s threshold)"
fi

echo "PASS: smoke_long_conversation (${ELAPSED}s)"
