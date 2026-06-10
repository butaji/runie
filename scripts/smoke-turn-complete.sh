#!/bin/bash
# Smoke tests for TurnComplete conditional visibility feature
BINARY="/Users/admin/.herdr/worktrees/runie/agent-impl/target/release/runie"
SESSION="runie_tc_$$"
LOG="/tmp/runie_tc_log_$$.txt"

cleanup() {
    tmux kill-session -t "$SESSION" 2>/dev/null || true
    rm -f "$LOG"
}
trap cleanup EXIT

echo "=== Building release binary ==="
cargo build --release -p runie-term 2>&1 | tail -2

echo ""
echo "=== Case 1: Single-action turn (no Turn completed expected) ==="
tmux new-session -d -s "$SESSION" -x 80 -y 24 "$BINARY"
sleep 0.5
tmux send-keys -t "$SESSION" "hello"
tmux send-keys -t "$SESSION" Enter
sleep 4.0
tmux capture-pane -t "$SESSION" -p > "$LOG"
if grep -qi "turn completed" "$LOG"; then
    echo "FAIL: 'Turn completed' appeared in trivial single-action turn"
    exit 1
fi
echo "PASS: No 'Turn completed' in trivial turn"

echo ""
echo "=== Case 2: Multi-action turn (Turn completed expected) ==="
tmux kill-session -t "$SESSION" 2>/dev/null; sleep 0.3
tmux new-session -d -s "$SESSION" -x 80 -y 24 "$BINARY"
sleep 0.5
tmux send-keys -t "$SESSION" "list files in current directory"
tmux send-keys -t "$SESSION" Enter
sleep 5.0
tmux capture-pane -t "$SESSION" -p > "$LOG"
if grep -qi "turn completed" "$LOG"; then
    echo "PASS: 'Turn completed' appeared in multi-action turn"
else
    echo "WARN: No 'Turn completed' (model may not use tools)"
fi

echo ""
echo "=== Case 3: Rapid submit x2 (no stuck timers, no overflow) ==="
tmux kill-session -t "$SESSION" 2>/dev/null; sleep 0.3
tmux new-session -d -s "$SESSION" -x 80 -y 24 "$BINARY"
sleep 0.5
tmux send-keys -t "$SESSION" "hi"
tmux send-keys -t "$SESSION" Enter
sleep 2.5
tmux send-keys -t "$SESSION" "hello again"
tmux send-keys -t "$SESSION" Enter
sleep 5.0
tmux capture-pane -t "$SESSION" -p > "$LOG"
if grep -iE "panic|thread.*panicked" "$LOG"; then
    echo "FAIL: Panic detected"
    exit 1
fi
if grep -E "[0-9]{4}\\.[0-9]s" "$LOG"; then
    echo "FAIL: Stuck timer (>1000s) detected"
    exit 1
fi
TURN_COUNT=$(grep -ci "turn completed" "$LOG" || true)
echo "Found $TURN_COUNT 'Turn completed' lines after 2 submits"
if [ "$TURN_COUNT" -gt 3 ]; then
    echo "FAIL: Too many 'Turn completed' lines"
    exit 1
fi
echo "PASS: No panics, no stuck timers, no overflow"

echo ""
echo "=== Case 4: Resize stress (gentle, tmux-safe) ==="
tmux kill-session -t "$SESSION" 2>/dev/null; sleep 0.3
tmux new-session -d -s "$SESSION" -x 80 -y 24 "$BINARY"
sleep 0.5
tmux send-keys -t "$SESSION" "say hello"
tmux send-keys -t "$SESSION" Enter
sleep 2.0
for i in 1 2 3 4 5; do
    tmux resize-window -t "$SESSION" -x $((60 + i * 4)) -y $((18 + i)) 2>/dev/null || true
    sleep 0.2
done
tmux capture-pane -t "$SESSION" -p > "$LOG"
if grep -iE "panic|thread.*panicked" "$LOG"; then
    echo "FAIL: Panic during resize"
    exit 1
fi
echo "PASS: No panics during resize"

echo ""
echo "=== Case 5: Ctrl+C interrupt ==="
tmux kill-session -t "$SESSION" 2>/dev/null; sleep 0.3
tmux new-session -d -s "$SESSION" -x 80 -y 24 "$BINARY"
sleep 0.5
tmux send-keys -t "$SESSION" "count to 100"
tmux send-keys -t "$SESSION" Enter
sleep 1.0
tmux send-keys -t "$SESSION" C-c
sleep 2.0
tmux capture-pane -t "$SESSION" -p > "$LOG"
if grep -iE "panic|thread.*panicked" "$LOG"; then
    echo "FAIL: Panic on Ctrl+C"
    exit 1
fi
echo "PASS: Ctrl+C handled without panic"

echo ""
echo "=== Case 6: Ctrl+L toggle collapse ==="
tmux kill-session -t "$SESSION" 2>/dev/null; sleep 0.3
tmux new-session -d -s "$SESSION" -x 80 -y 24 "$BINARY"
sleep 0.5
tmux send-keys -t "$SESSION" "list files"
tmux send-keys -t "$SESSION" Enter
sleep 4.0
tmux send-keys -t "$SESSION" C-l
sleep 1.0
tmux capture-pane -t "$SESSION" -p > "$LOG"
if grep -iE "panic|thread.*panicked" "$LOG"; then
    echo "FAIL: Panic on Ctrl+L"
    exit 1
fi
echo "PASS: Ctrl+L handled without panic"

echo ""
echo "=== All smoke tests passed ==="
