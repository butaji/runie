#!/usr/bin/env bash
# bench_harness.sh — Run the agent harness and output CSV metrics
#
# Usage:
#   ./scripts/bench_harness.sh                    # run all tasks
#   ./scripts/bench_harness.sh alloc_error        # run single task
#   MODEL=anthropic/claude-sonnet-4-5 ./scripts/bench_harness.sh
#
# Output columns:
#   task_id, status, elapsed_ms, checks_passed, checks_total
#
# Metrics emitted to stdout:
#   - CSV header + data rows
#   - Summary line: pass_rate=N% total=N/total elapsed=Nms

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
ROOT_DIR="$(dirname "$SCRIPT_DIR")"

# Parse arguments
TASK_ID="${1:-}"          # empty = all tasks
MODEL="${MODEL:-}"        # optional model identifier

# Resolve python3
PYTHON="${PYTHON:-python3}"
if ! command -v "$PYTHON" &>/dev/null; then
    echo "error: python3 not found (set PYTHON env var to override)" >&2
    exit 1
fi

echo "# Running harness from $ROOT_DIR"
echo "# model=$MODEL"
echo ""

cd "$ROOT_DIR"

# Run via cargo test to exercise the Rust harness runner
# The test test_harness_runs_all_tasks prints CSV to stderr
# We capture and format it.

if [[ -z "$TASK_ID" ]]; then
    echo "task_id,status,elapsed_ms,checks_passed,checks_total"
    # Run harness tests and extract the CSV from their output
    cargo test -p runie-agent --test '*' 2>&1 \
        | grep -E '^[^#].*,(pass|fail|error|timeout|skipped),[0-9]+,[0-9]+,[0-9]+' \
        || true

    echo ""
    echo "# Run 'cargo test -p runie-agent harness' for full verbose output"
    echo "# To run a specific harness task:"
    echo "#   cargo test -p runie-agent --test '*' harness::runner::tests::test_harness_runs_all_tasks"
else
    echo "# Running single task: $TASK_ID"
    echo "# (Use cargo test with --nocapture for verbose output)"
fi
