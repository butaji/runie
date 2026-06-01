#!/usr/bin/env bash
# Dev runner for runie. Wipes tmp_config and runs with dev folder.
set -euo pipefail

if [ -d "tmp_config" ]; then
    rm -rf tmp_config
fi
RUST_BACKTRACE=full cargo run -p runie-cli -- --dev-folder=./tmp_config