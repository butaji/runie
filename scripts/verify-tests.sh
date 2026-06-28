#!/bin/bash
set -euo pipefail

echo "=== Runie Test Verification Script ==="
echo ""

EXPECTED_TOTAL=2657
MIN_TESTS=100

# Test timeout: tests exceeding this are killed (seconds).
# Override with: RUST_TEST_TIMEOUT=60 ./scripts/verify-tests.sh
RUST_TEST_TIMEOUT="${RUST_TEST_TIMEOUT:-120}"
export RUST_TEST_TIMEOUT
echo "Test timeout: ${RUST_TEST_TIMEOUT}s"

# List tests
echo "Listing tests..."
cargo test --workspace -- --list > /tmp/test_list.txt 2>&1
TEST_COUNT=$(grep -c "test$" /tmp/test_list.txt || true)

echo ""
echo "=== Test Count Verification ==="
echo "Found: $TEST_COUNT tests"
echo "Expected: $EXPECTED_TOTAL tests"
echo "Minimum: $MIN_TESTS tests"
echo ""

if [ "$TEST_COUNT" -lt "$MIN_TESTS" ]; then
    echo "ERROR: Test count ($TEST_COUNT) is below minimum threshold ($MIN_TESTS)"
    exit 1
fi

if [ "$TEST_COUNT" -ne "$EXPECTED_TOTAL" ]; then
    echo "WARNING: Test count differs from expected"
    echo "Update EXPECTED_TOTAL in this script if this is intentional"
fi

# Run tests
echo ""
echo "=== Running Tests (timeout: ${RUST_TEST_TIMEOUT}s) ==="
set +e
cargo test --workspace > /tmp/test_output.txt 2>&1
TEST_EXIT=$?
set -e

echo ""
echo "=== Verifying Results ==="

if [ "$TEST_EXIT" -ne 0 ]; then
    echo "ERROR: cargo test exited with status $TEST_EXIT"
    tail -n 40 /tmp/test_output.txt
    exit 1
fi

# Check for failures (non-zero failed count)
if grep -qE "FAILED|^test result:.*[1-9][0-9]* failed" /tmp/test_output.txt; then
    echo "ERROR: Some tests failed!"
    grep -E "FAILED|^test result:.*[1-9][0-9]* failed" /tmp/test_output.txt
    exit 1
fi

# Check for compilation errors from doc tests / harness
if grep -qE "^error:|panicked at" /tmp/test_output.txt; then
    echo "ERROR: Test run contained errors or panics!"
    grep -E "^error:|panicked at" /tmp/test_output.txt
    exit 1
fi

# Verify passed/ignored counts
PASSED=$(grep "test result:" /tmp/test_output.txt | grep -oE "[0-9]+ passed" | awk '{sum+=$1} END {print sum}')
IGNORED=$(grep "test result:" /tmp/test_output.txt | grep -oE "[0-9]+ ignored" | awk '{sum+=$1} END {print sum}')
RUNNING=$(grep "^running [0-9]* test" /tmp/test_output.txt | awk '{sum+=$2} END {print sum}')

echo "Tests running: $RUNNING"
echo "Tests passed: $PASSED"
echo "Tests ignored: $IGNORED"

if [ "$PASSED" -eq 0 ]; then
    echo "ERROR: No tests passed!"
    exit 1
fi

if [ "$PASSED" -lt "$MIN_TESTS" ]; then
    echo "ERROR: Passed test count ($PASSED) is below minimum ($MIN_TESTS)"
    exit 1
fi

# Ignored tests are counted in `running` but not in `passed`.
TOTAL=$((PASSED + IGNORED))
if [ "$TOTAL" -ne "$RUNNING" ]; then
    echo "ERROR: Not all running tests were accounted for ($PASSED passed + $IGNORED ignored != $RUNNING running)"
    exit 1
fi

echo ""
echo "=== All tests passed! ($PASSED/$RUNNING) ==="
