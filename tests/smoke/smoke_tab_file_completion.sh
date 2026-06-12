#!/bin/bash
# Smoke test: Tab file/folder completion
# Layer 4 test - verifies no panics during tab completion workflows
set -e

export TMUX_TMPDIR=/tmp
tmux start-server 2>/dev/null || true

BINARY="$(pwd)/target/release/runie"
SESSION="runie_tab_file_$$"
LOG="/tmp/runie_tab_file_$$.log"

# Ensure binary exists
if [ ! -f "$BINARY" ]; then
    echo "[smoke] Building release binary..."
    cargo build --release 2>/dev/null
fi

trap 'tmux kill-session -t "$SESSION" 2>/dev/null || true' EXIT

echo "[smoke] Starting tmux session for tab file completion..."
tmux new-session -d -s "$SESSION" -x 80 -y 24 "$BINARY"
sleep 0.5

# ============================================================================
# Test 1: Simple prefix completion (no @ or ./ prefix needed)
# ============================================================================
echo "[smoke] Test 1: Simple prefix completion..."
tmux send-keys -t "$SESSION" "Cargo"
tmux send-keys -t "$SESSION" Tab
sleep 0.3

# Second Tab - should cycle or complete
tmux send-keys -t "$SESSION" Tab
sleep 0.3

# Clear and continue
tmux send-keys -t "$SESSION" C-u
sleep 0.2

# ============================================================================
# Test 2: Folder completion with trailing slash
# ============================================================================
echo "[smoke] Test 2: Folder completion..."
tmux send-keys -t "$SESSION" "crate"
tmux send-keys -t "$SESSION" Tab
sleep 0.3

# Accept completion
tmux send-keys -t "$SESSION" Tab
sleep 0.2

# Clear and continue
tmux send-keys -t "$SESSION" C-u
sleep 0.2

# ============================================================================
# Test 3: Relative path with ./
# ============================================================================
echo "[smoke] Test 3: Relative path completion..."
tmux send-keys -t "$SESSION" "./scr"
tmux send-keys -t "$SESSION" Tab
sleep 0.3

# Second Tab to complete
tmux send-keys -t "$SESSION" Tab
sleep 0.2

# Clear and continue
tmux send-keys -t "$SESSION" C-u
sleep 0.2

# ============================================================================
# Test 4: Multiple cycles through matches
# ============================================================================
echo "[smoke] Test 4: Multiple tab cycles..."
tmux send-keys -t "$SESSION" "C"
tmux send-keys -t "$SESSION" Tab
sleep 0.2
tmux send-keys -t "$SESSION" Tab  # cycle
sleep 0.2
tmux send-keys -t "$SESSION" Tab  # cycle
sleep 0.2
tmux send-keys -t "$SESSION" Tab  # cycle
sleep 0.2

# Clear
tmux send-keys -t "$SESSION" C-u
sleep 0.2

# ============================================================================
# Test 5: Tab on empty input (should flash, not panic)
# ============================================================================
echo "[smoke] Test 5: Tab on empty input..."
tmux send-keys -t "$SESSION" Tab
sleep 0.3

# ============================================================================
# Test 6: Tab with no matching files (should flash, not panic)
# ============================================================================
echo "[smoke] Test 6: Tab with no match..."
tmux send-keys -t "$SESSION" "zzzzzzzxyz"
tmux send-keys -t "$SESSION" Tab
sleep 0.3

# Clear
tmux send-keys -t "$SESSION" C-u
sleep 0.2

# ============================================================================
# Test 7: Submit after completion
# ============================================================================
echo "[smoke] Test 7: Submit with ghost completion..."
tmux send-keys -t "$SESSION" "Cargo"
tmux send-keys -t "$SESSION" Tab
sleep 0.3
tmux send-keys -t "$SESSION" Enter
sleep 1.0

# ============================================================================
# Resize stress during completion
# ============================================================================
echo "[smoke] Test 8: Resize stress during completion..."
for i in $(seq 1 5); do
    tmux resize-window -t "$SESSION" -x $((60 + i * 4)) -y $((20 + i))
    tmux send-keys -t "$SESSION" Tab
    sleep 0.1
done

# ============================================================================
# Rapid tab submissions
# ============================================================================
echo "[smoke] Test 9: Rapid tab submissions..."
tmux send-keys -t "$SESSION" "crate"
tmux send-keys -t "$SESSION" Tab
tmux send-keys -t "$SESSION" Enter
sleep 0.5
tmux send-keys -t "$SESSION" "C"
tmux send-keys -t "$SESSION" Tab
tmux send-keys -t "$SESSION" Enter
sleep 0.5

# ============================================================================
# Capture and check
# ============================================================================
echo "[smoke] Capturing output..."
tmux capture-pane -t "$SESSION" -p > "$LOG"
tmux send-keys -t "$SESSION" C-c
sleep 0.3

echo "[smoke] Checking for issues..."

# Assert no stuck timers
if grep -E '[0-9]{4}\.[0-9]s' "$LOG" 2>/dev/null; then
    echo "FAIL: STUCK TIMER detected!"
    tail -30 "$LOG"
    exit 1
fi

# Assert no panics
if grep -iE "panic|thread.*panicked|out of memory|segmentation fault|core dumped" "$LOG" 2>/dev/null; then
    echo "FAIL: Panic or crash detected!"
    tail -30 "$LOG"
    exit 1
fi

echo "[smoke] SUCCESS - All tab completion tests passed!"
echo "--- Sample output ---"
tail -20 "$LOG"
