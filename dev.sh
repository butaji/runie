#!/usr/bin/env bash
set -e
cd "$(dirname "$0")"

export PATH="$HOME/.cargo/bin:$PATH"

# Fast compilation settings
export CARGO_INCREMENTAL=1
export RUSTFLAGS="-C codegen-units=1 -C target-cpu=native"

# Use cranelift on nightly for blazing fast builds
CRANEFLIFT=""
if rustup run nightly rustc -Z codegen-backend=crane --help &>/dev/null 2>&1; then
    CRANEFLIFT="-Z codegen-backend=crane"
fi

TUI_PID=""
LAST_BUILD=""

build() {
    echo ">>> Building..."
    if cargo build -p runie-tui $CRANEFLIFT 2>&1 | grep -q "^error"; then
        echo ">>> Build failed"
        return 1
    fi
    echo ">>> Built"
}

start_tui() {
    ./target/debug/runie-tui &
    TUI_PID=$!
    echo ">>> TUI started (PID: $TUI_PID)"
}

stop_tui() {
    if [[ -n "$TUI_PID" ]] && kill -0 "$TUI_PID" 2>/dev/null; then
        kill "$TUI_PID" 2>/dev/null || true
        wait "$TUI_PID" 2>/dev/null || true
    fi
}

cleanup() {
    stop_tui
}
trap cleanup EXIT

echo "=== runie dev ==="
echo ""
echo "Hot reload enabled! Edit .rs files to rebuild automatically."
echo ""

# Initial build and start
build
start_tui

# Watch for file changes using inotifywait or polling fallback
if command -v inotifywait &>/dev/null; then
    # Linux: use inotify for instant notifications
    (
        while inotifywait -q -e modify -e create -e delete crates 2>/dev/null; do
            build && stop_tui && start_tui
        done
    ) &
else
    # macOS/Windows: use polling
    LAST_BUILD=$(find crates -name '*.rs' -type f -exec stat -f %m {} + 2>/dev/null | sort -rn | head -1 || echo 0)
    (
        while sleep 0.5; do
            CURRENT=$(find crates -name '*.rs' -type f -exec stat -f %m {} + 2>/dev/null | sort -rn | head -1 || echo 0)
            if [[ "$CURRENT" != "$LAST_BUILD" && "$CURRENT" != "0" ]]; then
                LAST_BUILD=$CURRENT
                build && stop_tui && start_tui
            fi
        done
    ) &
fi

WATCH_PID=$!

# Wait for TUI to exit
wait $TUI_PID 2>/dev/null || true
