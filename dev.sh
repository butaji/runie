#!/usr/bin/env bash
set -e
cd "$(dirname "$0")"

export CARGO_INCREMENTAL=1

echo "=== runie dev ==="
echo "Hot reload enabled! Edit files to rebuild automatically."
echo "Press Ctrl+C to exit."
echo ""

# Check if in tmux
if [ -n "$TMUX" ]; then
    echo "Running in tmux - using direct TUI"
    cargo watch -c -x 'run --bin runie-tui'
else
    echo "Running in terminal - using PTY wrapper"
    cargo watch -c -x 'run --bin runie-pty'
fi
