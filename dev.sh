#!/usr/bin/env bash
set -e
cd "$(dirname "$0")"

export RUSTFLAGS="-C codegen-units=1 -C target-cpu=native"

TUI_PID=""
LAST_BUILD=0

get_sources_time() {
    find crates -name '*.rs' -type f -exec stat -f %m {} + 2>/dev/null | sort -rn | head -1 || echo 0
}

build_and_run() {
    echo ">>> Building..."
    if ! cargo build -p runie-tui 2>&1 | grep -qv "^error"; then
        echo ">>> Build failed"
        return 1
    fi
    echo ">>> Build OK"
    
    # Kill old TUI
    if [[ -n "$TUI_PID" ]] && kill -0 "$TUI_PID" 2>/dev/null; then
        kill "$TUI_PID" 2>/dev/null || true
        wait "$TUI_PID" 2>/dev/null || true
    fi
    
    # Start TUI
    ./target/debug/runie-tui &
    TUI_PID=$!
}

cleanup() {
    [[ -n "$TUI_PID" ]] && kill "$TUI_PID" 2>/dev/null || true
}
trap cleanup EXIT

echo "=== runie dev ==="
echo ""

# Initial build
LAST_BUILD=$(get_sources_time)
build_and_run

# Watch loop
while true; do
    sleep 0.5
    
    # Check if TUI is still running
    if ! kill -0 "$TUI_PID" 2>/dev/null; then
        echo ">>> TUI exited, restarting..."
        LAST_BUILD=$(get_sources_time)
        build_and_run
    fi
    
    # Check for changes
    CURRENT=$(get_sources_time)
    if [[ "$CURRENT" != "$LAST_BUILD" && "$CURRENT" != "0" ]]; then
        LAST_BUILD=$CURRENT
        build_and_run
    fi
done
