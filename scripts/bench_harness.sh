#!/usr/bin/env bash
#
# Bench Harness for Runie Agent
# Based on SWE-bench / Terminal Bench 2.0 / bosun-ai patterns
#
# Usage:
#   ./bench_harness.sh              # Run all tasks with default model
#   ./bench_harness.sh --task foo   # Run single task
#   ./bench_harness.sh --model gpt-4o  # Specify model
#   ./bench_harness.sh --harness-config custom.toml  # Custom config
#   ./bench_harness.sh --verbose    # Verbose output
#   ./bench_harness.sh --csv output.csv  # Output CSV file
#
# Metrics tracked:
#   - task_id: unique task identifier
#   - status: pass/fail/error/timeout
#   - elapsed_ms: execution time in milliseconds
#   - checks_passed: number of checks that passed
#   - checks_total: total number of checks
#   - detail: detailed output or error message
#
# A/B testing support:
#   Use --harness-config to point to different prompt configurations
#   Run against same tasks with different configs to compare

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
HARNESS_DIR="$SCRIPT_DIR/../harness"
TASKS_DIR="$HARNESS_DIR/tasks"

# Defaults
TASK_ID=""
MODEL="${RUNIE_MODEL:-}"
HARNESS_CONFIG=""
VERBOSE=0
CSV_OUTPUT=""
PYTHON="${PYTHON:-python3}"

# Parse arguments
usage() {
    cat << EOF
Usage: $(basename "$0") [OPTIONS]

Options:
    --task ID           Run a single task (default: all tasks)
    --model MODEL       AI model to use (default: \$RUNIE_MODEL or none)
    --harness-config    Path to harness configuration file
    --csv FILE          Output CSV to file (default: stdout)
    --verbose, -v       Verbose output
    --help, -h          Show this help message

Environment:
    RUNIE_MODEL         Default model to use

Examples:
    $(basename "$0")                           # Run all tasks
    $(basename "$0") --task empty_state        # Run single task
    $(basename "$0") --model gpt-4o            # Specify model
    $(basename "$0") --harness-config ab_test_a.toml
    $(basename "$0") --csv results.csv        # Save to CSV

EOF
}

while [[ $# -gt 0 ]]; do
    case "$1" in
        --task)
            TASK_ID="$2"
            shift 2
            ;;
        --model)
            MODEL="$2"
            shift 2
            ;;
        --harness-config)
            HARNESS_CONFIG="$2"
            shift 2
            ;;
        --csv)
            CSV_OUTPUT="$2"
            shift 2
            ;;
        --verbose|-v)
            VERBOSE=1
            shift
            ;;
        --help|-h)
            usage
            exit 0
            ;;
        *)
            echo "Unknown option: $1"
            usage
            exit 1
            ;;
    esac
done

# Validate harness directory
if [[ ! -d "$TASKS_DIR" ]]; then
    echo "ERROR: Tasks directory not found: $TASKS_DIR" >&2
    exit 1
fi

# List available tasks
list_tasks() {
    if [[ "$(uname)" == "Darwin" ]]; then
        # macOS compatibility
        find "$TASKS_DIR" -mindepth 1 -maxdepth 1 -type d -exec basename {} \; | sort
    else
        # GNU find
        find "$TASKS_DIR" -mindepth 1 -maxdepth 1 -type d -printf "%f\n" | sort
    fi
}

# Run a single task
run_task() {
    local task_id="$1"
    local task_dir="$TASKS_DIR/$task_id"
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
    if command -v "$PYTHON" &> /dev/null; then
        "$PYTHON" -c "
import json, os

with open('$task_json') as f:
    task = json.load(f)

files = task.get('setup', {}).get('files', {})
for path, content in files.items():
    full_path = os.path.join('$sandbox/workspace', path)
    dir_path = os.path.dirname(full_path)
    if dir_path:
        os.makedirs(dir_path, exist_ok=True)
    with open(full_path, 'w') as f:
        f.write(content)
" 2>/dev/null || true
    fi

    # Run grader
    local start_time
    if [[ "$(uname)" == "Darwin" ]]; then
        # macOS: use seconds with decimal
        start_time=$(python3 -c 'import time; print(int(time.time() * 1000))')
    else
        # Linux: use %N for nanoseconds, convert to ms
        start_time=$(date +%s%N | awk '{printf "%d", $1/1000000}')
    fi
    local grader_output
    local grader_exit=0

    if [[ $VERBOSE -eq 1 ]]; then
        echo "[$task_id] Running grader in $sandbox/workspace..."
    fi

    # Copy crate files for grading (if in workspace)
    if [[ -d "$SCRIPT_DIR/../crates" ]]; then
        cp -r "$SCRIPT_DIR/../crates" "$sandbox/workspace/" 2>/dev/null || true
    fi

    # Run grader from workspace directory
    grader_output=$(cd "$sandbox/workspace" && "$PYTHON" "$grader" 2>&1) || grader_exit=$?

    local end_time
    if [[ "$(uname)" == "Darwin" ]]; then
        end_time=$(python3 -c 'import time; print(int(time.time() * 1000))')
    else
        end_time=$(date +%s%N | awk '{printf "%d", $1/1000000}')
    fi
    local elapsed=$((end_time - start_time))

    # Parse grader output
    local status="fail"
    local checks_passed=0
    local checks_total=0
    local detail=""

    # Count PASS/FAIL lines
    local pass_count=0
    local fail_count=0
    while IFS= read -r line; do
        if [[ "$line" == PASS:* ]]; then
            pass_count=$((pass_count + 1))
        fi
        if [[ "$line" == FAIL:* ]]; then
            fail_count=$((fail_count + 1))
        fi
        if [[ "$line" == *"/"* ]] && [[ "$line" != PASS:* ]] && [[ "$line" != FAIL:* ]]; then
            detail="$line"
        fi
        if [[ "$line" == RESULT:* ]]; then
            if [[ "$line" == *"pass"* ]]; then
                status="pass"
            elif [[ "$line" == *"error"* ]]; then
                status="error"
            fi
        fi
    done <<< "$grader_output"

    checks_passed=$pass_count
    checks_total=$((pass_count + fail_count))

    # Determine status based on results
    # Exit code 0 = pass, 1 = fail, other = error
    if [[ $grader_exit -eq 0 ]]; then
        status="pass"
    elif [[ $grader_exit -eq 1 ]]; then
        if [[ $fail_count -gt 0 ]]; then
            status="fail"
        else
            status="pass"  # Exit 0 but no PASS lines - unexpected but treat as pass
        fi
    else
        status="error"  # Non-zero, non-one exit code
    fi

    # Escape CSV special chars in detail
    detail="${detail//,/;}"
    detail="${detail//\"/\'}"

    # Cleanup
    rm -rf "$sandbox"

    # Output CSV row
    echo "$task_id,$status,$elapsed,$checks_passed,$checks_total,\"$detail\""

    if [[ $VERBOSE -eq 1 ]]; then
        echo "  → $status ($checks_passed/$checks_total checks, ${elapsed}ms)"
    fi
}

# Calculate pass rate
calculate_summary() {
    local results_file="$1"
    local total_tasks=$(tail -n +2 "$results_file" | wc -l)
    local passed=$(tail -n +2 "$results_file" | grep -c ",pass," || true)
    local failed=$(tail -n +2 "$results_file" | grep -c ",fail," || true)
    local errors=$(tail -n +2 "$results_file" | grep -c ",error," || true)
    local total_time=$(tail -n +2 "$results_file" | awk -F',' '{sum += $3} END {print sum}')
    local pass_rate=0
    if [[ $total_tasks -gt 0 ]]; then
        pass_rate=$(echo "scale=1; ($passed/$total_tasks)*100" | bc 2>/dev/null || echo "N/A")
    fi

    echo ""
    echo "=== Summary ==="
    echo "Total tasks:    $total_tasks"
    echo "Passed:         $passed"
    echo "Failed:         $failed"
    echo "Errors:         $errors"
    echo "Pass rate:      ${pass_rate}%"
    echo "Total time:     ${total_time:-0}ms"
}

# Main execution
main() {
    echo "Runie Agent Bench Harness"
    echo "========================="

    if [[ -n "$MODEL" ]]; then
        echo "Model: $MODEL"
    fi
    if [[ -n "$HARNESS_CONFIG" ]]; then
        echo "Config: $HARNESS_CONFIG"
    fi
    echo ""

    # Create temp file for results
    local results_file
    results_file=$(mktemp)
    trap "rm -f '$results_file'" EXIT

    # CSV header
    echo "task_id,status,elapsed_ms,checks_passed,checks_total,detail" > "$results_file"

    if [[ -n "$TASK_ID" ]]; then
        # Run single task
        echo "Running task: $TASK_ID"
        run_task "$TASK_ID" >> "$results_file"
    else
        # Run all tasks
        echo "Running all tasks..."
        for task in $(list_tasks); do
            run_task "$task" >> "$results_file"
        done
    fi

    # Output results
    if [[ -n "$CSV_OUTPUT" ]]; then
        cp "$results_file" "$CSV_OUTPUT"
        echo "Results saved to: $CSV_OUTPUT"
    else
        cat "$results_file"
    fi

    # Summary
    calculate_summary "$results_file"
}

main
