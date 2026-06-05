#!/usr/bin/env bash
set -e
cd "$(dirname "$0")"

export CARGO_INCREMENTAL=1

echo "=== runie dev ==="
echo "Hot reload enabled! Edit files to rebuild automatically."
echo "Press Ctrl+Q or Ctrl+C to exit."
echo ""

# Use cargo watch for hot reload
cargo watch -c -x 'run --bin runie-tui'
