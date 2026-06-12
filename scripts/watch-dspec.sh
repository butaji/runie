#!/usr/bin/env bash
# watch-dspec - Live preview of dspec changes
# Usage: ./scripts/watch-dspec.sh ui/dumps/scenarios/01_welcome_screen.dspec.json
# Requires: cargo, inotifywait (Linux) or fswatch (macOS)

set -euo pipefail

if [ $# -lt 1 ]; then
    echo "Usage: $0 <spec.dspec.json>"
    exit 1
fi

SPEC_FILE="$1"
if [ ! -f "$SPEC_FILE" ]; then
    echo "File not found: $SPEC_FILE"
    exit 1
fi

# Find the reference file
SPEC_NAME=$(basename "$SPEC_FILE" .dspec.json)
REF_FILE="${SPEC_FILE%.dspec.json}".txt
REF_FILE="${REF_FILE%/scenarios/}/grok/${SPEC_NAME}.txt"

if [ ! -f "$REF_FILE" ]; then
    echo "Reference file not found: $REF_FILE"
    REF_FILE=""
fi

# Check for inotifywait or fswatch
if command -v inotifywait &> /dev/null; then
    WATCH_CMD="inotifywait -e close_write -m -q"
    PLATFORM="linux"
elif command -v fswatch &> /dev/null; then
    WATCH_CMD="fswatch -1 -o"
    PLATFORM="macos"
else
    echo "Error: Requires inotifywait (Linux) or fswatch (macOS)"
    echo "  Linux: sudo apt install inotify-tools"
    echo "  macOS: brew install fswatch"
    exit 1
fi

# Change to project root
cd "$(dirname "$0")/.."

echo "Watching: $SPEC_FILE"
if [ -n "$REF_FILE" ]; then
    echo "Reference: $REF_FILE"
fi
echo ""
echo "Press Ctrl+C to stop"
echo ""

render() {
    local output
    if [ -n "$REF_FILE" ]; then
        output=$(cargo run -q -p runie-tui --bin runie-dspec "$SPEC_FILE" --diff "$REF_FILE" 2>/dev/null || echo "ERROR")
    else
        output=$(cargo run -q -p runie-tui --bin runie-dspec "$SPEC_FILE" 2>/dev/null || echo "ERROR")
    fi
    
    # Print just the relevant lines
    if echo "$output" | grep -q "✓ dspec matches reference"; then
        echo "✅ MATCH"
    elif echo "$output" | grep -q "^ERROR"; then
        echo "❌ ERROR"
    elif echo "$output" | grep -q "line [0-9]"; then
        echo "❌ DIFF"
        echo "$output" | grep -E "(line [0-9]:|  -|  \+)"
    else
        echo "$output"
    fi
    echo ""
}

# Initial render
render

# Watch loop
if [ "$PLATFORM" = "linux" ]; then
    $WATCH_CMD "$SPEC_FILE" 2>/dev/null | while read -r; do
        render
    done
else
    $WATCH_CMD "$SPEC_FILE" 2>/dev/null | while read -r; do
        render
    done
fi
