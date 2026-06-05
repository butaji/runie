#!/usr/bin/env bash
# Dev runner for runie with hot-reload.
# Usage: ./dev.sh

set -euo pipefail

# Add cargo bin to PATH
export PATH="$HOME/.cargo/bin:$PATH"

# Wipe tmp_config for clean dev state
if [ -d "tmp_config" ]; then
    rm -rf tmp_config
fi

# Set env for dev mode
export RUST_BACKTRACE=full

# Run with cargo watch for hot-reload
cargo watch -x "run -p runie-cli -- --dev-folder=./tmp_config"
