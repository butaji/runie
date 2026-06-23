#!/bin/bash
# Tmux integration test for runie
# This script tests that the application starts and responds to basic commands

set -e

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
PROJECT_ROOT="$(dirname "$SCRIPT_DIR")"
BINARY="$PROJECT_ROOT/target/release/runie"
SESSION="runie-test-tmux"
PASS_COUNT=0
FAIL_COUNT=0

pass() {
    echo "✓ $1"
    ((PASS_COUNT++)) || true
}

fail() {
    echo "✗ $1"
    ((FAIL_COUNT++)) || true
}

echo "=== Runie Tmux Integration Test ==="
echo "Binary: $BINARY"

if [ ! -f "$BINARY" ]; then
    echo "ERROR: Binary not found at $BINARY"
    exit 1
fi

# Cleanup function
cleanup() {
    echo "Cleaning up..."
    tmux kill-session -t "$SESSION" 2>/dev/null || true
}
trap cleanup EXIT

# Start runie in a new tmux session
echo ""
echo "--- Starting runie in tmux ---"
tmux new-session -d -s "$SESSION" "$BINARY" 2>&1 || true

# Wait for startup (need longer wait for terminal to initialize)
sleep 5

# Check if session is running
if tmux has-session -t "$SESSION" 2>/dev/null; then
    pass "Session started successfully"
else
    fail "Session failed to start"
    # Capture any error output
    tmux capture-pane -t "$SESSION" -p 2>/dev/null | head -20 || true
    exit 1
fi

# Check if the process is running
if pgrep -f "runie$" > /dev/null; then
    pass "Process is running"
else
    fail "Process is not running"
fi

# Capture initial screen content
echo ""
echo "--- Initial screen content ---"
tmux capture-pane -t "$SESSION" -p 2>/dev/null | tail -15 || true

echo ""
echo "--- Verifying UI renders correctly ---"

# Check that the input area is visible
SCREEN_CONTENT=$(tmux capture-pane -t "$SESSION" -p 2>/dev/null)
if echo "$SCREEN_CONTENT" | grep -q "Type a message"; then
    pass "Input area renders with placeholder"
else
    fail "Input area not visible"
fi

if echo "$SCREEN_CONTENT" | grep -q "ctrl+o"; then
    pass "Help text renders with keybindings"
else
    fail "Help text not visible"
fi

# Note: Keyboard input testing in tmux has limitations with crossterm
# The dry-run mode already verifies config is valid
echo ""
echo "--- Note: Keyboard testing in tmux is limited ---"
echo "    Use 'q' key in a real terminal to quit"

echo ""
echo "=== Tmux Test Summary ==="
echo "Passed: $PASS_COUNT"
echo "Failed: $FAIL_COUNT"

if [ $FAIL_COUNT -eq 0 ]; then
    echo "Result: PASS"
    exit 0
else
    echo "Result: FAIL"
    exit 1
fi
