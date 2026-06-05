#!/usr/bin/env bash
set -e
cd "$(dirname "$0")"

export RUSTFLAGS="-C codegen-units=1 -C target-cpu=native"

echo "=== runie dev ==="
echo "Building..."
cargo build -p runie-tui
echo "Starting TUI..."
./target/debug/runie-tui
