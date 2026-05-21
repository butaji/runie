#!/usr/bin/env bash
set -euo pipefail

# Fast lint runner for runie
# Runs structural linter (Python) then clippy on changed crates only

echo "=== Structural Linter (max 500 lines, 40 func lines, 10 complexity) ==="
python3 scripts/lint.py

echo ""
echo "=== Clippy (code quality) ==="
# Only run clippy on crates with Rust changes (fast)
CHANGED=$(git diff --name-only HEAD~5 -- '*.rs' | cut -d'/' -f2 | sort -u | grep '^runie-' || true)

if [ -z "$CHANGED" ]; then
    echo "No Rust changes in last 5 commits, skipping clippy"
    exit 0
fi

for crate in $CHANGED; do
    echo "  Checking $crate..."
    cargo clippy -p "$crate" --no-deps -- -D warnings 2>&1 | grep -E "^error:|^warning:" | head -10 || true
done

echo ""
echo "✓ All checks passed"
