#!/bin/bash
set -euo pipefail

BINARY="${1:-./target/release/runie}"
SESSION="runie_smoke_$$"
LOG="/tmp/runie_smoke_$$.log"
TMUX="tmux -L runie_smoke_$$ -f /dev/null"

trap '$TMUX kill-session -t "$SESSION" 2>/dev/null || true; rm -f "$LOG"' EXIT

$TMUX new-session -d -s "$SESSION" -x 80 -y 24 "RUNIE_MOCK=1 $BINARY"
sleep 2

# Open help
$TMUX send-keys -t "$SESSION" "/help"
$TMUX send-keys -t "$SESSION" Enter
sleep 1
$TMUX capture-pane -t "$SESSION" -p >> "$LOG"

# Send a message and submit
$TMUX send-keys -t "$SESSION" "list files"
$TMUX send-keys -t "$SESSION" Enter
sleep 2
$TMUX capture-pane -t "$SESSION" -p >> "$LOG"

# Resize stress
for i in $(seq 1 5); do
    $TMUX resize-window -t "$SESSION" -x $((40 + i * 8)) -y $((10 + i * 3))
    sleep 0.1
done
$TMUX capture-pane -t "$SESSION" -p >> "$LOG"

# Rapid submit
$TMUX send-keys -t "$SESSION" "hello"
$TMUX send-keys -t "$SESSION" Enter
$TMUX send-keys -t "$SESSION" "hello"
$TMUX send-keys -t "$SESSION" Enter
sleep 2
$TMUX capture-pane -t "$SESSION" -p >> "$LOG"

# Quit
$TMUX send-keys -t "$SESSION" C-c
sleep 1

if grep -qiE "panic|thread.*panicked" "$LOG"; then
    echo "ERROR: panic detected in smoke log"
    exit 1
fi

if grep -qE '[0-9]{4}\.[0-9]s' "$LOG"; then
    echo "ERROR: stuck timer detected"
    exit 1
fi

if ! grep -q "/help" "$LOG"; then
    echo "ERROR: help command did not render"
    exit 1
fi

echo "Smoke test passed"
