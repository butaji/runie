#!/bin/bash
set -e

# Smoke test: verify snapshot creation stays fast with 50+ messages
BINARY="$(pwd)/target/release/runie"
SESSION="runie_smoke_$$"
LOG="/tmp/runie_smoke_$$.log"

trap 'tmux kill-session -t "$SESSION" 2>/dev/null || true' EXIT

cargo build --release --bin runie 2>/dev/null

tmux new-session -d -s "$SESSION" -x 80 -y 24 "$BINARY"
sleep 0.3

# Send 50+ messages rapidly
for i in $(seq 1 55); do
    tmux send-keys -t "$SESSION" "msg $i"
    tmux send-keys -t "$SESSION" Enter
    sleep 0.05
done

sleep 2.0

# Resize stress
for i in $(seq 1 10); do
    tmux resize-window -t "$SESSION" -x $((20 + i * 6)) -y $((5 + i * 2))
    sleep 0.05
done

# Capture and check
tmux capture-pane -t "$SESSION" -p > "$LOG"
tmux send-keys -t "$SESSION" C-c

# Assert no stuck timers
if grep -E '[0-9]{4}\.[0-9]s' "$LOG"; then
    echo "STUCK TIMER!"; exit 1
fi

# Assert no panics
if grep -i "panic\|thread.*panicked" "$LOG"; then
    echo "PANIC!"; exit 1
fi

echo "Smoke test passed"
