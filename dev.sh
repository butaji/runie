#!/usr/bin/env bash
cd "$(dirname "$0")"

export PATH="$HOME/.cargo/bin:$PATH"
export CARGO_INCREMENTAL=1

echo "=== runie dev ==="
echo ""
echo "Starting with hot reload..."
echo "Edit files and save to rebuild automatically."
echo ""
echo "Press Ctrl+C to exit."
echo ""

# Use cargo watch for hot reload
cargo watch -c -x 'run --bin runie-tui'
