#!/bin/bash
set -e

echo "=== Runie Test Verification Script ==="
echo ""

EXPECTED_TOTAL=1321
MIN_TESTS=100

# List tests
echo "Listing tests..."
cargo test -- --list 2>&1 | tee /tmp/test_list.txt
TEST_COUNT=$(grep "test$" /tmp/test_list.txt | wc -l)

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
echo "=== Running Tests ==="
cargo test 2>&1 | tee /tmp/test_output.txt

echo ""
echo "=== Verifying Results ==="

# Check for failures
if grep -q "FAILED" /tmp/test_output.txt; then
    echo "ERROR: Some tests failed!"
    grep "FAILED" /tmp/test_output.txt
    exit 1
fi

# Check for panics
if grep -q "panicked at" /tmp/test_output.txt; then
    echo "ERROR: Test panic detected!"
    grep "panicked at" /tmp/test_output.txt
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
