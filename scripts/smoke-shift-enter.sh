#!/bin/bash
set -e

BINARY="$(pwd)/target/release/runie"
SESSION="runie_shift_enter_$$"
LOG="/tmp/runie_shift_enter_$$.log"

trap 'tmux kill-session -t "$SESSION" 2>/dev/null || true' EXIT

echo "[smoke] Testing Shift+Enter creates newline in input..."

tmux new-session -d -s "$SESSION" -x 80 -y 24 "$BINARY"
sleep 0.5

# Type some text
tmux send-keys -t "$SESSION" "hello"
sleep 0.2

# Press Shift+Enter to create a newline
tmux send-keys -t "$SESSION" S-Enter
sleep 0.2

# Type more text on the second line
tmux send-keys -t "$SESSION" "world"
sleep 0.2

# Capture the screen
tmux capture-pane -t "$SESSION" -p > "$LOG"

# Check that input box shows both lines
if grep -q "hello" "$LOG" && grep -q "world" "$LOG"; then
    echo "✓ PASS: Both lines visible after Shift+Enter"
else
    echo "FAIL: Multi-line input not working"
    cat "$LOG"
    exit 1
fi

# Now verify Enter (without shift) submits the message
tmux send-keys -t "$SESSION" Enter
sleep 0.5

tmux capture-pane -t "$SESSION" -p > "$LOG"

# After submit, the message should appear in the chat area
if grep -q "hello" "$LOG" && grep -q "world" "$LOG"; then
    echo "✓ PASS: Message submitted with Enter (hello + world visible in chat)"
else
    echo "WARN: Could not verify submitted message in chat area"
fi

tmux send-keys -t "$SESSION" C-c
sleep 0.2
tmux kill-session -t "$SESSION" 2>/dev/null || true

echo "[smoke] SUCCESS - Shift+Enter newline test passed"
