#!/usr/bin/env bash
set -e
cd "$(dirname "$0")"

# Fast build settings
export RUSTFLAGS="-C codegen-units=1 -C target-cpu=native"

PID=""
LAST_BUILD=""

get_sources_mtime() {
    find crates -name '*.rs' -type f 2>/dev/null | xargs stat -f %m 2>/dev/null | sort -n | tail -1
}

build_and_run() {
    echo ""
    echo ">>> Rebuilding..."
    cargo build -p runie-tui 2>&1 | grep -E "Compiling|error|^error" | head -20
    local status=${PIPESTATUS[0]}
    
    if [[ $status -eq 0 ]]; then
        echo ">>> Build OK"
    else
        echo ">>> Build failed, keeping old binary"
        return 1
    fi
    
    # Kill old instance
    if [[ -n "$PID" ]] && kill -0 $PID 2>/dev/null; then
        kill -TERM $PID 2>/dev/null || true
        sleep 0.1
    fi
    
    # Start new instance
    ./target/debug/runie-tui &
    PID=$!
    echo ">>> Started PID: $PID"
}

echo "=== runie dev mode ==="
echo "Edit .rs files to rebuild automatically"
echo "Press Ctrl+C to stop"
echo ""

# Initial build
LAST_BUILD=$(get_sources_mtime)
build_and_run

# Watch loop
while true; do
    sleep 0.3
    
    # Check if app is still running
    if ! kill -0 $PID 2>/dev/null; then
        echo ""
        echo ">>> App exited unexpectedly"
        LAST_BUILD=$(get_sources_mtime)
        build_and_run
        continue
    fi
    
    # Check for file changes
    CURRENT=$(get_sources_mtime)
    if [[ "$CURRENT" != "$LAST_BUILD" ]]; then
        LAST_BUILD=$CURRENT
        build_and_run
    fi
done &
WATCH_PID=$!

cleanup() {
    echo ""
    echo ">>> Stopping..."
    kill -TERM $WATCH_PID 2>/dev/null || true
    if [[ -n "$PID" ]] && kill -0 $PID 2>/dev/null; then
        kill -TERM $PID 2>/dev/null || true
        wait $PID 2>/dev/null || true
    fi
    echo ">>> Done"
}
trap cleanup EXIT

wait $WATCH_PID
