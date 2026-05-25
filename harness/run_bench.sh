#!/bin/bash
# Runie Agent Harness - Evaluation Script
#
# Runs the agent harness against a task suite and outputs metrics.
#
# Usage:
#   ./run_bench.sh                  # Run all tasks
#   ./run_bench.sh --model <model>  # Run with specific model
#   ./run_bench.sh --task <task>    # Run single task
#   ./run_bench.sh --verbose        # Verbose output
#   ./run_bench.sh --csv            # Output as CSV
#
# Environment Variables:
#   RUNIE_MODEL         - Model to use (e.g., gpt-4o, claude-sonnet-4)
#   RUNIE_API_KEY       - API key for provider
#   RUNIE_PROVIDER     - Provider to use (openai, anthropic, etc.)
#   PYTHON             - Python interpreter (default: python3)

set -e

# Debug: uncomment to see verbose output
# set -x

# Defaults
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
HARNESS_DIR="$(dirname "$SCRIPT_DIR")"
VERBOSE=0
OUTPUT_CSV=0
SINGLE_TASK=""
MODEL=""
PYTHON="${PYTHON:-python3}"

# Change to harness directory so tasks path is correct
cd "$SCRIPT_DIR"

# TASKS_DIR must be relative to current directory after cd
TASKS_DIR="tasks"

# Parse arguments
while [[ $# -gt 0 ]]; do
    case $1 in
        --model)
            MODEL="$2"
            shift 2
            ;;
        --task)
            SINGLE_TASK="$2"
            shift 2
            ;;
        --verbose|-v)
            VERBOSE=1
            shift
            ;;
        --csv)
            OUTPUT_CSV=1
            shift
            ;;
        --help|-h)
            echo "Usage: $0 [OPTIONS]"
            echo ""
            echo "Options:"
            echo "  --model <model>    Model to use (e.g., gpt-4o)"
            echo "  --task <task>      Run specific task only"
            echo "  --verbose, -v      Verbose output"
            echo "  --csv              Output as CSV"
            echo "  --help, -h         Show this help"
            exit 0
            ;;
        *)
            echo "Unknown option: $1"
            exit 1
            ;;
    esac
done

# CSV output format: task_id,status,elapsed_ms,checks_passed,checks_total
print_csv_header() {
    echo "task_id,status,elapsed_ms,checks_passed,checks_total"
}

# Run a single task
run_task() {
    local task_id="$1"
    local task_dir="$TASKS_DIR/$task_id"
    local start_time=$(date +%s)
    
    # Check task exists
    if [[ ! -d "$task_dir" ]]; then
        echo "ERROR: Task not found: $task_id"
        return 1
    fi
    
    # Check required files
    if [[ ! -f "$task_dir/task.json" ]]; then
        echo "ERROR: task.json not found for $task_id"
        return 1
    fi
    
    if [[ ! -f "$task_dir/grader.py" ]]; then
        echo "ERROR: grader.py not found for $task_id"
        return 1
    fi
    
    # Create temp workspace
    local workspace=$(mktemp -d)
    trap "rm -rf $workspace" RETURN
    
    # Copy setup files
    if [[ -d "$task_dir/setup" ]]; then
        cp -r "$task_dir/setup/"* "$workspace/" 2>/dev/null || true
    fi
    
    # Run grader
    local grader_output
    if [[ $VERBOSE -eq 1 ]]; then
        echo "[TASK] $task_id"
        echo "[TASK] Workspace: $workspace"
    fi
    
    # Execute grader and capture output
    grader_output=$("$PYTHON" "$task_dir/grader.py" 2>&1) || true
    local grader_exit=$?
    
    local end_time=$(date +%s)
    local elapsed=$(((end_time - start_time) * 1000))
    
    # Parse grader output
    local status="error"
    local checks_passed=0
    local checks_total=0
    
    if [[ $grader_exit -eq 0 ]]; then
        status="pass"
    else
        status="fail"
    fi
    
    # Extract check counts
    if [[ "$grader_output" =~ ([0-9]+)/([0-9]+)\ checks\ passed ]]; then
        checks_passed="${BASH_REMATCH[1]}"
        checks_total="${BASH_REMATCH[2]}"
    fi
    
    # Output
    if [[ $OUTPUT_CSV -eq 1 ]]; then
        echo "$task_id,$status,$elapsed,$checks_passed,$checks_total"
    else
        local status_icon="❌"
        [[ "$status" == "pass" ]] && status_icon="✅"
        
        echo "$status_icon [$status] $task_id - ${checks_passed}/${checks_total} checks (${elapsed}ms)"
        
        if [[ $VERBOSE -eq 1 ]]; then
            echo "$grader_output" | sed 's/^/    /'
        fi
    fi
}

# List available tasks
list_tasks() {
    find "$TASKS_DIR" -maxdepth 1 -type d | tail -n +2 | xargs -I {} basename {}
}

# Main
main() {
    echo "═══════════════════════════════════════════════════════════════"
    echo "  Runie Agent Harness - Evaluation Suite"
    echo "═══════════════════════════════════════════════════════════════"
    echo ""
    
    if [[ -n "$MODEL" ]]; then
        echo "Model: $MODEL"
    fi
    echo "Python: $PYTHON"
    echo ""
    
    if [[ $OUTPUT_CSV -eq 1 ]]; then
        print_csv_header
    else
        echo "Task Results:"
        echo "───────────────────────────────────────────────────────────────"
    fi
    
    local total_start=$(date +%s)
    local pass_count=0
    local fail_count=0
    local total_checks_passed=0
    local total_checks=0
    
    # Get task list
    if [[ -n "$SINGLE_TASK" ]]; then
        tasks=("$SINGLE_TASK")
    else
        tasks=($(list_tasks))
    fi
    
    # Run each task
    for task in "${tasks[@]}"; do
        local task_output
        task_output=$(run_task "$task" 2>&1)
        local exit_code=$?
        
        echo "$task_output"
        
        if [[ $exit_code -eq 0 ]]; then
            ((pass_count++)) || true
        else
            ((fail_count++)) || true
        fi
    done
    
    local total_end=$(date +%s)
    local total_elapsed=$(((total_end - total_start) * 1000))
    
    # Summary
    if [[ $OUTPUT_CSV -eq 0 ]]; then
        echo ""
        echo "───────────────────────────────────────────────────────────────"
        echo "Summary:"
        echo "  Tasks: ${#tasks[@]}"
        echo "  Passed: $pass_count"
        echo "  Failed: $fail_count"
        if [[ ${#tasks[@]} -gt 0 ]]; then
            local pass_rate=$((100 * pass_count / ${#tasks[@]}))
            echo "  Pass Rate: $pass_rate%"
        fi
        echo "  Total Time: ${total_elapsed}ms"
    fi
}

main "$@"
