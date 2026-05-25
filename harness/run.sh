#!/usr/bin/env bash
# Runie Agent Harness Runner
#
# Based on SWE-bench / Terminal Bench 2.0 patterns.
# Runs agent micro-tasks against a sandbox workspace and
# outputs metrics as CSV.
#
# Usage:
#   ./run.sh              # Run all tasks
#   ./run.sh --task foo   # Run single task
#   ./run.sh --model gpt-4o  # Specify model
#   ./run.sh --verbose    # Verbose output
#
# Output:
#   CSV: task_id, status, elapsed_ms, checks_passed, checks_total, detail
#   Summary: pass_rate, total_tasks, total_time

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
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
    # Find all task directories
    for d in "$HARNESS_DIR"/*; do
        if [[ -d "$d" ]]; then
            TASKS+=("$(basename "$d")")
        fi
    done
fi

echo "=== Runie Agent Harness ==="
echo "Model: $MODEL"
echo "Tasks: ${TASKS[*]:-none}"
echo "Python: $PYTHON"
echo ""

# Output CSV header
echo "task_id,status,elapsed_ms,checks_passed,checks_total,detail"

# Temp file for results
RESULTS_FILE=$(mktemp)
TOTAL_START=$(date +%s)

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
    dir_path = os.path.dirname(full_path)
    if dir_path:
        os.makedirs(dir_path, exist_ok=True)
    with open(full_path, 'w') as f:
        f.write(content)
" 2>/dev/null || true
    fi

    # Run grader
    local start_time=$(date +%s)
    local grader_output
    local grader_exit=0

    if [[ $VERBOSE -eq 1 ]]; then
        echo "[$task_id] Running grader..."
    fi

    # Run grader from project root (not sandbox) so it can find source files
    grader_output=$(cd "$SCRIPT_DIR/.." && "$PYTHON" "$grader" 2>&1) || grader_exit=$?

    local end_time=$(date +%s)
    local elapsed=$(((end_time - start_time) * 1000))

    # Parse grader output
    local status="fail"
    local checks_passed=0
    local checks_total=0
    local detail=""

    # Parse PASS/FAIL lines and RESULT line
    local grader_result=""
    while IFS= read -r line; do
        if [[ "$line" == PASS:* ]]; then
            checks_passed=$((checks_passed + 1))
        fi
        if [[ "$line" == FAIL:* ]]; then
            checks_total=$((checks_total + 1))
        fi
        if [[ "$line" == RESULT:* ]]; then
            grader_result="$line"
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
    local pass_count=0
    local fail_count=0
    while IFS= read -r line; do
        if [[ "$line" == PASS:* ]]; then
            pass_count=$((pass_count + 1))
        fi
        if [[ "$line" == FAIL:* ]]; then
            fail_count=$((fail_count + 1))
        fi
    done <<< "$grader_output"

    checks_passed=$pass_count
    checks_total=$((pass_count + fail_count))

    # Determine status based on results
    # Respect grader's explicit RESULT when it exits 0
    if [[ $grader_exit -ne 0 ]]; then
        status="error"
    elif [[ -n "$grader_result" ]]; then
        # Grader gave explicit RESULT - trust it
        if [[ "$grader_result" == *"pass"* ]]; then
            status="pass"
        else
            status="fail"
        fi
    elif [[ $fail_count -eq 0 ]] && [[ $pass_count -gt 0 ]]; then
        # All checks passed (no explicit RESULT)
        status="pass"
    elif [[ $fail_count -gt 0 ]]; then
        status="fail"
    else
        status="fail"
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

    # Store results in temp file (compatible with bash 3.2)
    echo "$task_id:$status:$checks_passed:$checks_total" >> "$RESULTS_FILE"

    # Cleanup
    rm -rf "$sandbox"

    return 0
}

# Run all tasks
for task_id in "${TASKS[@]}"; do
    run_task "$task_id" || true
done

# Summary
TOTAL_END=$(date +%s)
TOTAL_MS=$(( (TOTAL_END - TOTAL_START) * 1000 ))

echo ""
echo "=== Summary ==="
echo "Total time: ${TOTAL_MS}ms"

# Count pass/fail from results file
pass_count=0
fail_count=0
error_count=0
total_checks=0
passed_checks=0

if [[ -f "$RESULTS_FILE" ]]; then
    while IFS=: read -r tid status cp ct; do
        case "$status" in
            pass) pass_count=$((pass_count + 1)) ;;
            fail) fail_count=$((fail_count + 1)) ;;
            error) error_count=$((error_count + 1)) ;;
        esac
        total_checks=$((total_checks + ct))
        passed_checks=$((passed_checks + cp))
    done < "$RESULTS_FILE"
fi

total_tasks=${#TASKS[@]}
pass_rate=0
if [[ $total_tasks -gt 0 ]]; then
    pass_rate=$(( pass_count * 100 / total_tasks ))
fi

echo "Tasks: $pass_count pass / $fail_count fail / $error_count error ($total_tasks total)"
echo "Checks: $passed_checks / $total_checks passed"
echo "Pass rate: ${pass_rate}%"

# Cleanup
rm -f "$RESULTS_FILE"

exit 0
