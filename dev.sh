#!/usr/bin/env bash
set -e
cd "$(dirname "$0")"

export CARGO_INCREMENTAL=1

echo "=== runie dev ==="
echo "Hot reload enabled! Edit files to rebuild automatically."
echo "Press Ctrl+C to exit."
echo ""

# Hot reload with cargo watch
exec cargo watch -c -x 'run --bin runie-tui'
