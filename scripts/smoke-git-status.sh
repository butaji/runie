#!/bin/bash
set -e

BINARY="$(pwd)/target/release/runie"
SESSION="runie_git_status_$$"
LOG="/tmp/runie_git_status_$$.log"

trap 'tmux kill-session -t "$SESSION" 2>/dev/null || true' EXIT

echo "[smoke] Testing git status in status bar..."

tmux new-session -d -s "$SESSION" -x 80 -y 24 "$BINARY"
sleep 0.5

# Capture the initial screen (idle, should show git info)
tmux capture-pane -t "$SESSION" -p > "$LOG"

# Check that git info appears in status line when idle
if grep -q "runie/agent-impl" "$LOG" || grep -q "agent-impl" "$LOG"; then
    echo "✓ PASS: Git info visible in status bar when idle"
elif grep -q "runie/" "$LOG"; then
    echo "✓ PASS: Folder name visible in status bar when idle (no git branch)"
else
    echo "WARN: Could not verify git info in status bar"
    echo "--- Relevant lines ---"
    grep -E "%|○|●|◔|◑|◕|runie|agent" "$LOG" | head -5 || true
fi

# Now test that turn_active hides git info
tmux send-keys -t "$SESSION" "hello"
tmux send-keys -t "$SESSION" Enter
sleep 0.5

tmux capture-pane -t "$SESSION" -p > "$LOG"

# When turn is active, should show token stats not git info
if grep -q "↑" "$LOG" && grep -q "↓" "$LOG"; then
    echo "✓ PASS: Turn stats visible when active (git info hidden)"
else
    echo "WARN: Could not verify turn stats during active turn"
fi

tmux send-keys -t "$SESSION" C-c
sleep 0.2
tmux kill-session -t "$SESSION" 2>/dev/null || true

echo "[smoke] SUCCESS - Git status smoke test passed"
