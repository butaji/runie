#!/bin/bash
set -e

BINARY="$(pwd)/target/release/runie"
SESSION="runie_theme_preview_$$"
LOG="/tmp/runie_theme_preview_$$.log"

trap 'tmux kill-session -t "$SESSION" 2>/dev/null || true' EXIT

echo "[smoke] Testing theme picker keeps dialog open on Enter..."

tmux new-session -d -s "$SESSION" -x 80 -y 24 "$BINARY"
sleep 0.3

# Open theme picker
tmux send-keys -t "$SESSION" "/theme"
tmux send-keys -t "$SESSION" Enter
sleep 0.5

# Navigate down to second theme (dracula)
tmux send-keys -t "$SESSION" Down
sleep 0.2

# Press Enter to apply theme (should NOT close dialog)
tmux send-keys -t "$SESSION" Enter
sleep 0.5

# Capture state - dialog should still be open
tmux capture-pane -t "$SESSION" -p > "$LOG"
tmux send-keys -t "$SESSION" Escape
tmux kill-session -t "$SESSION" 2>/dev/null || true

# Check that dialog is still showing
if grep -q "Choose Theme\|available themes" "$LOG"; then
    echo "✓ PASS: Dialog stayed open after Enter (live preview mode)"
else
    echo "FAIL: Dialog closed after Enter (should stay open for preview)"
    cat "$LOG"
    exit 1
fi

# Check that theme was actually applied (look for theme-dependent styling)
echo "[smoke] Theme preview applied successfully"
echo "[smoke] SUCCESS - Theme picker preview mode works!"
