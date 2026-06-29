#!/bin/bash
# Check that all production .rs files are within the 500-line limit.
# This replaces the custom build.rs file-length linter.
set -euo pipefail

MAX_LINES=500
ERRORS=0

echo "=== Checking file line limits (max ${MAX_LINES} lines) ==="

# Find all .rs files under crates/, excluding target/, tests/, benches/
while IFS= read -r file; do
    # Skip target/ directories
    if [[ "$file" == *"/target/"* ]]; then continue; fi

    # Skip test files and benches from the line limit
    rel="${file#*/}"  # strip leading path prefix
    if [[ "$rel" == *"/tests/"* ]] \
        || [[ "$rel" == *"_tests.rs" ]] \
        || [[ "$rel" == *"_test.rs" ]] \
        || [[ "$rel" == */tests.rs ]] \
        || [[ "$rel" == *"/benches/"* ]]; then
        continue
    fi

    lines=$(wc -l < "$file")
    if [ "$lines" -gt "$MAX_LINES" ]; then
        echo "  ERROR: $rel has $lines lines (max $MAX_LINES)"
        ERRORS=$((ERRORS + 1))
    fi
done < <(find crates/ -name "*.rs" 2>/dev/null)

if [ "$ERRORS" -gt 0 ]; then
    echo ""
    echo "=== $ERRORS file(s) exceed the line limit ==="
    exit 1
fi

echo "=== All files are within the ${MAX_LINES}-line limit ==="
