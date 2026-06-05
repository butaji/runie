#!/usr/bin/env bash
set -e
cd "$(dirname "$0")"
export RUST_LOG=info
cargo run -p runie-tui "$@"
