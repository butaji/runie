#!/bin/bash
# Run all smoke tests
# Requires: tmux installed and properly configured

set -e

# Check if tmux is available
if ! command -v tmux &> /dev/null; then
    echo "SKIP: tmux not installed"
    exit 0
fi

# Try to start tmux server, skip if permission issues
export TMUX_TMPDIR=/tmp

# Check if tmux works
if ! tmux start-server 2>/dev/null && ! tmux list-sessions 2>/dev/null; then
    # Try to use existing socket
    if [ -S "/private/tmp/runie-tmux/tmux" ]; then
        export TMUX_SOCKET=/private/tmp/runie-tmux
    else
        echo "SKIP: tmux server cannot be started (permission issues)"
        echo "This is common on macOS - ensure /private/tmp/tmux-* has correct permissions"
        exit 0
    fi
fi

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
cd "$SCRIPT_DIR/../.."

echo "=== Runie Smoke Test Suite ==="
echo ""

PASSED=0
FAILED=0
SKIPPED=0

for test in smoke_*.sh; do
    if [ "$test" = "run_all.sh" ]; then
        continue
    fi
    
    echo "Running: $test"
    if bash "$test" 2>&1; then
        PASSED=$((PASSED + 1))
        echo "PASS: $test"
    else
        EXIT_CODE=$?
        if [ $EXIT_CODE -eq 0 ]; then
            # Test explicitly skipped
            SKIPPED=$((SKIPPED + 1))
            echo "SKIP: $test"
        else
            FAILED=$((FAILED + 1))
            echo "FAIL: $test"
        fi
    fi
    echo ""
done

echo "=== Results ==="
echo "Passed: $PASSED"
echo "Failed: $FAILED"
echo "Skipped: $SKIPPED"

if [ "$FAILED" -gt 0 ]; then
    exit 1
fi

echo "All smoke tests passed!"
