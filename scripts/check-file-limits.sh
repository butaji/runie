#!/bin/bash
# Check that all production .rs files are within the 500-line limit.
# This replaces the custom build.rs file-length linter.
set -euo pipefail

MAX_LINES=500
ERRORS=0

# Exception list: files that exceed the limit but are accepted.
# These are documented in tasks/fix-ci-gates-on-dev.md.
# Note: paths are stripped of the "crates/" prefix by ${file#*/}.
EXEMPT_PATTERN="^runie-core/src/update/dispatch\.rs$|^runie-core/src/shell\.rs$|^runie-core/src/model_catalog/mod\.rs$|^runie-core/src/config/config_impl\.rs$|^runie-core/src/actors/session/session_handlers\.rs$|^runie-core/src/event/generated\.rs$|^runie-core/src/event/mod\.rs$|^runie-core/src/session/store\.rs$|^runie-core/src/session/tree\.rs$|^runie-core/src/session/replay\.rs$|^runie-core/build\.rs$|^runie-core/src/subagents/mod\.rs$|^runie-agent/src/stream_response\.rs$|^runie-agent/src/tool/find_definitions\.rs$|^runie-tui/src/ui_actor/mod\.rs$|^runie-tui/src/bootstrap\.rs$|^runie-tui/src/popups/panel/form\.rs$|^runie-provider/src/mock\.rs$|^runie-provider/src/openai/protocol\.rs$|^runie-cli/src/inspect/mod\.rs$|^runie-testing/src/keystroke_dsl\.rs$"

echo "=== Checking file line limits (max ${MAX_LINES} lines) ==="

# Find all .rs files under crates/, excluding target/, tests/, benches/
while IFS= read -r file; do
    # Skip target/ directories
    if [[ "$file" == *"/target/"* ]]; then continue; fi

    # Skip test files and benches from the line limit
    rel="${file#*/}"  # strip leading path prefix
    if [[ "$rel" == *"/tests/"* ]] \
        || [[ "$rel" == *"/tests/"* ]] \
        || [[ "$rel" == *"_tests.rs" ]] \
        || [[ "$rel" == *"_test.rs" ]] \
        || [[ "$rel" == */tests.rs ]] \
        || [[ "$rel" == *"/benches/"* ]]; then
        continue
    fi

    # Skip exempt files
    if echo "$rel" | grep -qE "$EXEMPT_PATTERN"; then continue; fi

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
