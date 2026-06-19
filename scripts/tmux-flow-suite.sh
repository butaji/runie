#!/bin/bash
# Comprehensive tmux flow suite for runie.
# Runs the existing smoke/audit scripts plus the real-MiniMax flow suite and
# reports a combined pass/fail count. Uses an isolated HOME for reproducibility.
set -uo pipefail

cd "$(dirname "$0")/.."

TOTAL_PASS=0
TOTAL_FAIL=0

run_suite() {
    local name="$1"
    local cmd="$2"
    local log="/tmp/runie_suite_$$_${name// /_}.log"
    local passed=0
    local failed=0
    echo ""
    echo "--- $name ---"
    eval "$cmd" > "$log" 2>&1 || true
    if grep -qE "Results: [0-9]+ passed, [0-9]+ failed" "$log"; then
        passed=$(grep -oE "Results: [0-9]+ passed" "$log" | tail -1 | grep -oE "[0-9]+")
        failed=$(grep -oE "Results: [0-9]+ passed, *[0-9]+ failed" "$log" | tail -1 | grep -oE ", *[0-9]+ failed" | grep -oE "[0-9]+")
    elif grep -qE "passed|failed" "$log"; then
        # Scripts like smoke-tmux report a single pass/fail line.
        if grep -qE "passed$|passed \(" "$log"; then
            passed=1
        else
            failed=1
        fi
    else
        failed=1
    fi
    TOTAL_PASS=$((TOTAL_PASS + passed))
    TOTAL_FAIL=$((TOTAL_FAIL + failed))
    if [ "$failed" -eq 0 ]; then
        echo "  ✓ $name passed ($passed flows)"
    else
        echo "  ✗ $name failed ($failed flows, see $log)"
    fi
}

# Isolated HOME for deterministic mock-provider runs.
MOCK_HOME="/tmp/runie_suite_mock_home_$$"
mkdir -p "$MOCK_HOME/.runie"
cat > "$MOCK_HOME/.runie/config.toml" <<'TOML'
provider = "mock"
[model_providers.mock]
base_url = "http://test"
api_key = "testkey"
models = ["echo"]
[models]
default = "echo"
TOML

echo "========================================"
echo "  Runie Tmux Flow Suite"
echo "========================================"

cargo build --release -p runie-tui --bin runie 2>&1 | tail -n 3

run_suite "smoke-tmux" "RUNIE_MOCK=1 ./scripts/smoke-tmux.sh"
run_suite "smoke-onboarding" "HOME=$MOCK_HOME ./scripts/smoke-onboarding.sh"
run_suite "smoke-model-selector" "HOME=$MOCK_HOME RUNIE_MOCK=1 ./scripts/smoke-model-selector.sh"
run_suite "smoke-session-io" "HOME=$MOCK_HOME RUNIE_MOCK=1 ./scripts/smoke-session-io.sh"
run_suite "tmux-login-logout" "./tmux_login_logout_test.sh"
run_suite "tmux-collapse-expand" "./tmux_collapse_expand_test.sh"
run_suite "ux-audit" "./scripts/ux-audit.sh"
run_suite "ui-deep-audit" "HOME=$MOCK_HOME RUNIE_MOCK=1 ./scripts/ui-deep-audit.sh"
run_suite "minimax-real-flows" "./scripts/minimax-real-flows.sh"

echo ""
echo "========================================"
echo "  Total flows: $((TOTAL_PASS + TOTAL_FAIL))"
echo "  Passed: $TOTAL_PASS"
echo "  Failed: $TOTAL_FAIL"
echo "========================================"

rm -rf "$MOCK_HOME"

if [ "$TOTAL_FAIL" -gt 0 ]; then
    exit 1
fi
