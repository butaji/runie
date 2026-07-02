#!/bin/bash
# Run all tests using cargo-nextest + doctests
# This script is kept for backward compatibility with CI and scripts that call it.

set -euo pipefail

echo "=== Runie Test Verification ==="
echo ""

# Run nextest tests (with 120s slow timeout per test from .config/nextest.toml)
echo "Running unit and integration tests..."
cargo nextest run --workspace

# Run doctests separately (nextest skips them by default)
echo ""
echo "Running doctests..."
cargo test --workspace --doc

echo ""
echo "=== All tests passed! ==="
