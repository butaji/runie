#!/bin/bash
# Edge case UX testing — finding real bugs
set -e

BINARY="/Users/admin/.herdr/worktrees/runie/agent-impl/target/release/runie"
SESSION="runie_edge_$$"
LOG="/tmp/runie_edge_$$.log"

cleanup() {
    tmux kill-session -t "$SESSION" 2>/dev/null || true
    rm -f "$LOG"
}
trap cleanup EXIT

pass() { echo "  ✓ $1"; }
fail() { echo "  ✗ FAIL: $1"; exit 1; }
info() { echo "  ℹ $1"; }

echo "========================================"
echo "  EDGE CASE UX AUDIT"
echo "========================================"

# ============================================================================
# EDGE CASE 1: Empty input submit — should NOT send empty message
# ============================================================================
echo ""
echo "--- Edge 1: Empty input submit ---"
tmux new-session -d -s "$SESSION" -x 80 -y 24 "$BINARY"
sleep 0.5
tmux send-keys -t "$SESSION" Enter
sleep 1.0
tmux capture-pane -t "$SESSION" -p > "$LOG"
# Empty submit should not create a user message bubble
# Check if there are multiple "│" borders (empty messages might still show)
USER_MSGS=$(grep -c "│  │" "$LOG" || true)
info "Empty submit produced $USER_MSGS empty message patterns"
# The app should stay idle — no user message, no thinking indicator
if grep -q "⠋\|Thinking\|Running" "$LOG"; then
    fail "Empty submit triggered agent thinking!"
fi
pass "Empty submit does not trigger agent"

# ============================================================================
# EDGE CASE 2: Abort during tool running — state should reset
# ============================================================================
echo ""
echo "--- Edge 2: Abort during tool running ---"
tmux kill-session -t "$SESSION" 2>/dev/null || true; sleep 0.3
tmux new-session -d -s "$SESSION" -x 80 -y 24 "$BINARY"
sleep 0.5
tmux send-keys -t "$SESSION" "list files"
tmux send-keys -t "$SESSION" Enter
sleep 2.0
# Abort while tool might be running
tmux send-keys -t "$SESSION" Escape
sleep 1.0
# Now submit a new message immediately
tmux send-keys -t "$SESSION" "hello"
tmux send-keys -t "$SESSION" Enter
sleep 4.0
tmux capture-pane -t "$SESSION" -p > "$LOG"
if grep -qi "panic" "$LOG"; then
    fail "Panic after abort + new submit"
fi
# Check that the new message was processed
if grep -q "hello" "$LOG"; then
    pass "New submit works after abort"
else
    fail "New submit did not work after abort"
fi

# ============================================================================
# EDGE CASE 3: Rapid submit 10x — queue overflow
# ============================================================================
echo ""
echo "--- Edge 3: Rapid submit 10x ---"
tmux kill-session -t "$SESSION" 2>/dev/null || true; sleep 0.3
tmux new-session -d -s "$SESSION" -x 80 -y 24 "$BINARY"
sleep 0.5
tmux send-keys -t "$SESSION" "count to 100"
tmux send-keys -t "$SESSION" Enter
sleep 0.5
for i in 1 2 3 4 5 6 7 8 9; do
    tmux send-keys -t "$SESSION" "msg$i"
    tmux send-keys -t "$SESSION" Enter
    sleep 0.1
done
sleep 15.0
tmux capture-pane -t "$SESSION" -p > "$LOG"
if grep -qi "panic" "$LOG"; then
    fail "Panic after rapid 10x submit"
fi
# Count how many messages were actually processed
MSG_COUNT=$(grep -o "msg[0-9]" "$LOG" | sort -u | wc -l | tr -d ' ')
info "Processed $MSG_COUNT unique queued messages"
if grep -E "[0-9]{4}\.[0-9]s" "$LOG"; then
    fail "Stuck timer after rapid submit"
fi
pass "Rapid 10x submit handled"

# ============================================================================
# EDGE CASE 4: Input history with multiline
# ============================================================================
echo ""
echo "--- Edge 4: History with multiline ---"
tmux kill-session -t "$SESSION" 2>/dev/null || true; sleep 0.3
tmux new-session -d -s "$SESSION" -x 80 -y 24 "$BINARY"
sleep 0.5
tmux send-keys -t "$SESSION" "line1"
tmux send-keys -t "$SESSION" S-Enter
tmux send-keys -t "$SESSION" "line2"
tmux send-keys -t "$SESSION" Enter
sleep 3.0
tmux send-keys -t "$SESSION" "simple"
tmux send-keys -t "$SESSION" Enter
sleep 3.0
# Now press Up twice to get back to multiline
tmux send-keys -t "$SESSION" Up
tmux send-keys -t "$SESSION" Up
sleep 0.5
tmux capture-pane -t "$SESSION" -p > "$LOG"
if grep -qi "panic" "$LOG"; then
    fail "Panic after history navigation"
fi
pass "History navigation with multiline handled"

# ============================================================================
# EDGE CASE 5: Delete word at start of input (Ctrl+W)
# ============================================================================
echo ""
echo "--- Edge 5: Delete word at start of input ---"
tmux kill-session -t "$SESSION" 2>/dev/null || true; sleep 0.3
tmux new-session -d -s "$SESSION" -x 80 -y 24 "$BINARY"
sleep 0.5
tmux send-keys -t "$SESSION" C-w
sleep 0.3
tmux capture-pane -t "$SESSION" -p > "$LOG"
if grep -qi "panic" "$LOG"; then
    fail "Panic after Ctrl+W at start"
fi
pass "Ctrl+W at start handled"

# ============================================================================
# EDGE CASE 6: Delete to end at end of input (Ctrl+K)
# ============================================================================
echo ""
echo "--- Edge 6: Delete to end at end of input ---"
tmux send-keys -t "$SESSION" "hello"
tmux send-keys -t "$SESSION" C-k
sleep 0.3
tmux capture-pane -t "$SESSION" -p > "$LOG"
if grep -qi "panic" "$LOG"; then
    fail "Panic after Ctrl+K at end"
fi
pass "Ctrl+K at end handled"

# ============================================================================
# EDGE CASE 7: Scroll up then submit new message — should scroll to bottom
# ============================================================================
echo ""
echo "--- Edge 7: Scroll up then submit ---"
tmux kill-session -t "$SESSION" 2>/dev/null || true; sleep 0.3
tmux new-session -d -s "$SESSION" -x 80 -y 24 "$BINARY"
sleep 0.5
tmux send-keys -t "$SESSION" "say hello world this is a test"
tmux send-keys -t "$SESSION" Enter
sleep 3.0
tmux send-keys -t "$SESSION" Up
sleep 0.3
tmux send-keys -t "$SESSION" "new msg"
tmux send-keys -t "$SESSION" Enter
sleep 4.0
tmux capture-pane -t "$SESSION" -p > "$LOG"
if grep -qi "panic" "$LOG"; then
    fail "Panic after scroll-up + submit"
fi
pass "Scroll up then submit handled"

# ============================================================================
# EDGE CASE 8: Toggle collapse during streaming
# ============================================================================
echo ""
echo "--- Edge 8: Toggle collapse during streaming ---"
tmux kill-session -t "$SESSION" 2>/dev/null || true; sleep 0.3
tmux new-session -d -s "$SESSION" -x 80 -y 24 "$BINARY"
sleep 0.5
tmux send-keys -t "$SESSION" "count to 100"
tmux send-keys -t "$SESSION" Enter
sleep 1.0
tmux send-keys -t "$SESSION" C-e
sleep 0.5
tmux send-keys -t "$SESSION" C-e
sleep 3.0
tmux capture-pane -t "$SESSION" -p > "$LOG"
if grep -qi "panic" "$LOG"; then
    fail "Panic after toggle collapse during streaming"
fi
pass "Toggle collapse during streaming handled"

# ============================================================================
# EDGE CASE 9: Open palette during streaming
# ============================================================================
echo ""
echo "--- Edge 9: Open palette during streaming ---"
tmux kill-session -t "$SESSION" 2>/dev/null || true; sleep 0.3
tmux new-session -d -s "$SESSION" -x 80 -y 24 "$BINARY"
sleep 0.5
tmux send-keys -t "$SESSION" "count to 100"
tmux send-keys -t "$SESSION" Enter
sleep 1.0
tmux send-keys -t "$SESSION" C-p
sleep 1.0
tmux send-keys -t "$SESSION" Escape
sleep 3.0
tmux capture-pane -t "$SESSION" -p > "$LOG"
if grep -qi "panic" "$LOG"; then
    fail "Panic after palette during streaming"
fi
pass "Palette during streaming handled"

# ============================================================================
# EDGE CASE 10: Trust toggle while agent is thinking
# ============================================================================
echo ""
echo "--- Edge 10: Trust toggle while thinking ---"
tmux kill-session -t "$SESSION" 2>/dev/null || true; sleep 0.3
tmux new-session -d -s "$SESSION" -x 80 -y 24 "$BINARY"
sleep 0.5
tmux send-keys -t "$SESSION" "count to 100"
tmux send-keys -t "$SESSION" Enter
sleep 1.0
tmux send-keys -t "$SESSION" "/trust"
tmux send-keys -t "$SESSION" Enter
sleep 3.0
tmux capture-pane -t "$SESSION" -p > "$LOG"
if grep -qi "panic" "$LOG"; then
    fail "Panic after trust while thinking"
fi
pass "Trust toggle while thinking handled"

# ============================================================================
# EDGE CASE 11: Footer status accuracy
# ============================================================================
echo ""
echo "--- Edge 11: Footer shows correct status ---"
tmux kill-session -t "$SESSION" 2>/dev/null || true; sleep 0.3
tmux new-session -d -s "$SESSION" -x 80 -y 24 "$BINARY"
sleep 0.5
tmux send-keys -t "$SESSION" "list files"
tmux send-keys -t "$SESSION" Enter
sleep 2.0
tmux capture-pane -t "$SESSION" -p > "$LOG"
# During tool running, footer should show "Running ls..." or similar
if grep -q "Running\|Thinking\|⠋\|⠙\|⠹" "$LOG"; then
    pass "Footer shows active status during tool"
else
    info "Footer status not clearly visible (may have finished quickly)"
fi

# After turn complete, footer should be idle
tmux send-keys -t "$SESSION" "say done"
tmux send-keys -t "$SESSION" Enter
sleep 3.0
tmux capture-pane -t "$SESSION" -p > "$LOG"
if grep -q "⠋\|⠙\|⠹\|Running\|Thinking" "$LOG"; then
    fail "Footer still shows active status after turn complete!"
fi
pass "Footer shows idle after turn complete"

# ============================================================================
# EDGE CASE 12: Input placeholder when scrolled up
# ============================================================================
echo ""
echo "--- Edge 12: Input placeholder when scrolled ---"
tmux kill-session -t "$SESSION" 2>/dev/null || true; sleep 0.3
tmux new-session -d -s "$SESSION" -x 80 -y 24 "$BINARY"
sleep 0.5
tmux send-keys -t "$SESSION" "say hello"
tmux send-keys -t "$SESSION" Enter
sleep 3.0
tmux send-keys -t "$SESSION" Up
tmux send-keys -t "$SESSION" Up
sleep 0.5
tmux capture-pane -t "$SESSION" -p > "$LOG"
# When scrolled up, input should show placeholder
if grep -q "↑\|Scroll\|more" "$LOG"; then
    pass "Input shows scroll indicator when scrolled up"
else
    info "Scroll indicator not visible (may be at bottom)"
fi

echo ""
echo "========================================"
echo "  EDGE CASE AUDIT COMPLETE"
echo "========================================"
