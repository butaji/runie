#!/usr/bin/env bash
# Runie Agent Harness Runner
#
# Based on SWE-bench / Terminal Bench 2.0 patterns.
# Runs agent micro-tasks against a sandbox workspace and
# outputs metrics as CSV.
#
# Usage:
#   ./harness/run.sh              # Run all tasks
#   ./harness/run.sh --task foo  # Run single task
#   ./harness/run.sh --model gpt-4o  # Specify model
#   ./harness/run.sh --verbose   # Verbose output
#
# Output:
#   CSV: task_id, status, elapsed_ms, checks_passed, checks_total, detail
#   Summary: pass_rate, total_tasks, total_time

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"
HARNESS_DIR="$SCRIPT_DIR/tasks"

# Defaults
TASKS=()
MODEL="${RUNIE_MODEL:-mock}"
VERBOSE=0
PYTHON="${PYTHON3:-python3}"

# Parse args
while [[ $# -gt 0 ]]; do
    case $1 in
        --task)
            TASKS+=("$2")
            shift 2
            ;;
        --model)
            MODEL="$2"
            shift 2
            ;;
        --verbose|-v)
            VERBOSE=1
            shift
            ;;
        --help|-h)
            echo "Usage: $0 [--task TASK_ID] [--model MODEL] [--verbose]"
            echo ""
            echo "Environment variables:"
            echo "  RUNIE_MODEL    Model to use (default: mock)"
            echo "  PYTHON3        Python interpreter (default: python3)"
            exit 0
            ;;
        *)
            echo "Unknown option: $1"
            exit 1
            ;;
    esac
done

# Default to all tasks if none specified
if [[ ${#TASKS[@]} -eq 0 ]]; then
    TASKS=(empty_state ctrl_c permission_rollback)
fi

# Colors
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m'

echo "=== Runie Agent Harness ==="
echo "Model: $MODEL"
echo "Tasks: ${TASKS[*]}"
echo "Python: $PYTHON"
echo ""

# Output CSV header
echo "task_id,status,elapsed_ms,checks_passed,checks_total,detail"

# Track results
declare -A RESULTS
declare -A CHECKS_PASSED
declare -A CHECKS_TOTAL
declare -A ELAPSED_MS
TOTAL_START=$(date +%s%N)

run_task() {
    local task_id="$1"
    local task_dir="$HARNESS_DIR/$task_id"
    local task_json="$task_dir/task.json"
    local grader="$task_dir/grader.py"

    if [[ ! -d "$task_dir" ]]; then
        echo "$task_id,error,0,0,0,task directory not found"
        return 1
    fi

    if [[ ! -f "$task_json" ]]; then
        echo "$task_id,error,0,0,0,task.json not found"
        return 1
    fi

    if [[ ! -f "$grader" ]]; then
        echo "$task_id,error,0,0,0,grader.py not found"
        return 1
    fi

    # Create temp workspace
    local sandbox="/tmp/runie-harness-$task_id-$$"
    mkdir -p "$sandbox/workspace"

    # Read setup files from task.json and create them in workspace
    if command -v python3 &> /dev/null; then
        python3 -c "
import json
with open('$task_json') as f:
    task = json.load(f)
setup = task.get('setup', {})
files = setup.get('files', {})
for path, content in files.items():
    import os
    full_path = os.path.join('$sandbox/workspace', path)
    os.makedirs(os.path.dirname(full_path), exist_ok=True)
    with open(full_path, 'w') as f:
        f.write(content)
" 2>/dev/null || true
    fi

    # Run grader
    local start_time=$(date +%s%3N)
    local grader_output
    local grader_exit=0

    if [[ $VERBOSE -eq 1 ]]; then
        echo "[$task_id] Running grader in $sandbox/workspace..."
    fi

    # Run grader from workspace directory
    grader_output=$(cd "$sandbox/workspace" && "$PYTHON" "$grader" 2>&1) || grader_exit=$?

    local end_time=$(date +%s%3N)
    local elapsed=$((end_time - start_time))

    # Parse grader output
    local status="fail"
    local checks_passed=0
    local checks_total=0
    local detail=""

    # Parse PASS/FAIL lines and RESULT line
    while IFS= read -r line; do
        if [[ "$line" == PASS:* ]]; then
            ((checks_passed++)) || true
        fi
        if [[ "$line" == FAIL:* ]]; then
            ((checks_total++)) || true
            ((checks_passed++)) || true  # Count as both total and passed
        fi
        if [[ "$line" == RESULT:* ]]; then
            if [[ "$line" == *"pass"* ]]; then
                status="pass"
            elif [[ "$line" == *"error"* ]]; then
                status="error"
            fi
        fi
        if [[ "$line" == *"/"* ]] && [[ "$line" != PASS:* ]] && [[ "$line" != FAIL:* ]]; then
            detail="$line"
        fi
    done <<< "$grader_output"

    # Count total checks from grader output (PASS + FAIL lines)
    checks_total=$checks_passed
    local pass_count=0
    local fail_count=0
    while IFS= read -r line; do
        if [[ "$line" == PASS:* ]]; then
            ((pass_count++)) || true
        fi
        if [[ "$line" == FAIL:* ]]; then
            ((fail_count++)) || true
        fi
    done <<< "$grader_output"

    checks_passed=$pass_count
    checks_total=$((pass_count + fail_count))

    if [[ $grader_exit -eq 0 ]] && [[ "$status" != "fail" ]]; then
        status="pass"
    elif [[ $grader_exit -ne 0 ]]; then
        status="error"
    fi

    # Escape CSV special chars in detail
    detail=$(echo "$detail" | tr ',' ';' | tr '\n' ' ' | xargs)

    echo "$task_id,$status,$elapsed,$checks_passed,$checks_total,$detail"

    # Verbose output
    if [[ $VERBOSE -eq 1 ]]; then
        echo "--- grader output ---"
        echo "$grader_output"
        echo "---------------------"
    fi

    # Store results
    RESULTS["$task_id"]=$status
    CHECKS_PASSED["$task_id"]=$checks_passed
    CHECKS_TOTAL["$task_id"]=$checks_total
    ELAPSED_MS["$task_id"]=$elapsed

    # Cleanup
    rm -rf "$sandbox"

    return 0
}

# Run all tasks
for task_id in "${TASKS[@]}"; do
    run_task "$task_id" || true
done

# Summary
TOTAL_END=$(date +%s%N)
TOTAL_MS=$(( (TOTAL_END - TOTAL_START) / 1000000 ))

echo ""
echo "=== Summary ==="
echo "Total time: ${TOTAL_MS}ms"

# Count pass/fail
pass_count=0
fail_count=0
error_count=0
total_checks=0
passed_checks=0

for task_id in "${TASKS[@]}"; do
    status="${RESULTS[$task_id]:-error}"
    case "$status" in
        pass) ((pass_count++)) || true ;;
        fail) ((fail_count++)) || true ;;
        error) ((error_count++)) || true ;;
    esac
    ((total_checks += ${CHECKS_TOTAL[$task_id]:-0})) || true
    ((passed_checks += ${CHECKS_PASSED[$task_id]:-0})) || true
done

total_tasks=${#TASKS[@]}
pass_rate=0
if [[ $total_tasks -gt 0 ]]; then
    pass_rate=$(( pass_count * 100 / total_tasks ))
fi

echo "Tasks: $pass_count pass / $fail_count fail / $error_count error ($total_tasks total)"
echo "Checks: $passed_checks / $total_checks passed"
echo "Pass rate: ${pass_rate}%"

exit 0
