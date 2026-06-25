#!/bin/bash
# Lint script to detect direct field access on AppState and inner state structs.
# Production code should use accessors; direct field mutation is forbidden.
# Tests are exempt from this check (but should still compile).

set -e

cd "$(dirname "$0")/.."

echo "Checking for direct field access in production code..."

# Direct field writes in production code (excluding tests, benches, examples)
# Pattern: state.something.field = or self.something.field =
# Also exclude comments (//! and //)
VIOLATIONS=$(rg \
  --type rust \
  --glob '!**/tests/**' \
  --glob '!**/*_tests.rs' \
  --glob '!**/benches/**' \
  --glob '!**/examples/**' \
  -n \
  --line-buffered \
  '^\s*(?!//)[^/].*(state|self)\.(session|input|agent|view|config|completion)\.[a-z_]+\s*=' \
  crates/runie-core/src \
  2>/dev/null | grep -v '^\s*//' || true)

if [ -n "$VIOLATIONS" ]; then
    echo "ERROR: Direct field writes detected in production code:"
    echo "$VIOLATIONS"
    echo ""
    echo "Use accessors instead: state.session_mut().field = value"
    exit 1
fi

echo "✓ No direct field writes in production code"
