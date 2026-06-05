#!/usr/bin/env bash
set -e
cd "$(dirname "$0")"

# Fast build settings
export RUSTFLAGS="-C codegen-units=1 -C target-cpu=native"

export PATH="$HOME/.cargo/bin:$PATH"

# Check for watchexec
if ! command -v watchexec &>/dev/null; then
    echo "Installing watchexec..."
    cargo install watchexec-cli
fi

echo ""
echo "=== runie dev mode ==="
echo "Edit .rs files to rebuild automatically"
echo "Press Ctrl+C to stop"
echo ""

# Initial build
echo "Building..."
cargo build -p runie-tui 2>&1 | grep -E "Compiling|error|Finished" | tail -5
echo ""

# Use watchexec to rebuild and restart on .rs/.toml changes
# -r: restart mode (kills old process before starting new one)
# -e rs,toml: only watch .rs and .toml files
# --debounce 0.5s: debounce file changes
watchexec -r \
    -e rs,toml \
    --ignore-glob '**/target/**' \
    --ignore-glob '**/.git/**' \
    --debounce 0.5s \
    -q \
    -- cargo build -p runie-tui && ./target/debug/runie-tui
