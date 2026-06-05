#!/usr/bin/env bash
cd "$(dirname "$0")"

export PATH="$HOME/.cargo/bin:$PATH"
export CARGO_INCREMENTAL=1
export RUSTFLAGS="-C codegen-units=16"

APP_PID=""

# Detect OS for library extension
case "$(uname -s)" in
    Darwin*) LIB_EXT="dylib" ;;
    *)       LIB_EXT="so" ;;
esac

build_app() {
    cargo build -p runie-app 2>&1
}

echo "=== runie dev ==="
echo ""

# Initial build
if ! build_app | tail -1 | grep -q "Finished"; then
    echo "Build failed"
    exit 1
fi
echo "App lib built"

# Start host (which loads the app lib)
./target/debug/runie-tui &
APP_PID=$!
echo "TUI started (PID: $APP_PID)"
echo ""
echo "Hot reload enabled! Edit .rs files to rebuild automatically."
echo ""

# Watch for changes
LAST_BUILD=$(find crates/runie-app -name '*.rs' -type f -exec stat -f %m {} + 2>/dev/null | sort -rn | head -1 || echo 0)

while kill -0 $APP_PID 2>/dev/null; do
    sleep 0.3
    
    CURRENT=$(find crates/runie-app -name '*.rs' -type f -exec stat -f %m {} + 2>/dev/null | sort -rn | head -1 || echo 0)
    
    if [[ "$CURRENT" != "$LAST_BUILD" && "$CURRENT" != "0" ]]; then
        LAST_BUILD=$CURRENT
        echo ""
        echo ">>> Change detected, rebuilding app lib..."
        if build_app | tail -1 | grep -q "Finished"; then
            echo ">>> App lib built (hot reload automatic)"
        else
            echo ">>> Build failed"
        fi
    fi
done
