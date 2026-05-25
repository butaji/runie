#!/usr/bin/env bash
# =============================================================================
# Agent Harness Benchmark Runner
# =============================================================================
# Inspired by SWE-bench, Terminal Bench 2.0, and bosun-ai/agent-test-harness
#
# Usage:
#   ./bench_harness.sh [--model MODEL] [--harness-config CONFIG] [--verbose]
#   ./bench_harness.sh --list          # List available tasks
#   ./bench_harness.sh --task TASK_ID # Run single task
#
# Output: CSV with columns: task_id, status, elapsed_ms, checks_passed, checks_total
# =============================================================================

# Don't use set -e - we want to continue after task failures
set -uo pipefail

# Configuration
HARNESS_DIR="${HARNESS_DIR:-crates/runie-agent/src/harness}"
TASKS_DIR="${HARNESS_DIR}/tasks"
TEMP_DIR="${TMPDIR:-/tmp}/runie-harness"
VERBOSE=false
SINGLE_TASK=""
MODEL="auto"

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

# =============================================================================
# Functions
# =============================================================================

list_tasks() {
    echo "Available tasks:"
    echo ""
    if [[ -d "$TASKS_DIR" ]]; then
        for task_dir in "$TASKS_DIR"/*/; do
            if [[ -d "$task_dir" ]]; then
                task_id=$(basename "$task_dir")
                task_json="$task_dir/task.json"
                if [[ -f "$task_json" ]]; then
                    name=$(python3 -c "import json; print(json.load(open('$task_json')).get('name', '$task_id'))" 2>/dev/null || echo "$task_id")
                    echo "  $task_id - $name"
                else
                    echo "  $task_id"
                fi
            fi
        done
    else
        echo "  (no tasks found in $TASKS_DIR)"
    fi
    echo ""
    echo "Total: $(find "$TASKS_DIR" -maxdepth 1 -type d 2>/dev/null | tail -n +2 | wc -l) tasks"
}

usage() {
    cat << EOF
Usage: $(basename "$0") [OPTIONS]

Options:
    --model MODEL         Set the model to use (default: auto)
    --harness-config CFG  Set harness configuration (default: default)
    --verbose, -v         Enable verbose output
    --list               List available tasks
    --task TASK_ID        Run a single task
    --help, -h            Show this help message

Examples:
    $(basename "$0") --list
    $(basename "$0") --task alloc_error --verbose
    $(basename "$0") --model anthropic/claude-3-sonnet
EOF
}

run_task() {
    local task_id="$1"
    local task_dir="$TASKS_DIR/$task_id"
    local workspace_dir="$TEMP_DIR/$task_id/workspace"
    local result_file="$TEMP_DIR/$task_id/result.txt"
    
    # Check if task exists
    if [[ ! -d "$task_dir" ]]; then
        echo "Task not found: $task_id"
        return 1
    fi
    
    local task_json="$task_dir/task.json"
    if [[ ! -f "$task_json" ]]; then
        echo "Task definition not found: $task_json"
        return 1
    fi
    
    # Create temp workspace
    mkdir -p "$workspace_dir"
    
    # Copy setup files
    if [[ -d "$task_dir/setup" ]]; then
        cp -r "$task_dir/setup/"* "$workspace_dir/" 2>/dev/null || true
    fi
    
    # Extract setup from task.json
    python3 << PYEOF 2>/dev/null || true
import json
with open('$task_json') as f:
    task = json.load(f)
    for path, content in task.get('setup', {}).get('files', {}).items():
        import os
        full_path = os.path.join('$workspace_dir', path)
        os.makedirs(os.path.dirname(full_path), exist_ok=True)
        with open(full_path, 'w') as out:
            out.write(content)
PYEOF
    
    [[ "$VERBOSE" == true ]] && echo "  Workspace: $workspace_dir"
    
    # Run grader (placeholder - in full implementation, would run agent)
    local start_time
    start_time=$(python3 -c 'import time; print(int(time.time() * 1000))')
    local status="skip"
    local checks_passed=0
    local checks_total=0
    local detail=""
    
    if [[ -f "$task_dir/grader.py" ]]; then
        # Run the grader
        if python3 "$task_dir/grader.py" > "$result_file" 2>&1; then
            status="pass"
        else
            status="fail"
        fi
        
        # Parse result
        detail=$(cat "$result_file" | head -5 | tr '\n' ' ')
        
        # Count checks
        checks_total=$(grep -c "^PASS:\|^FAIL:" "$result_file" 2>/dev/null || echo 0)
        checks_passed=$(grep -c "^PASS:" "$result_file" 2>/dev/null || echo 0)
    else
        detail="No grader found"
        checks_total=1
    fi
    
    local end_time
    end_time=$(python3 -c 'import time; print(int(time.time() * 1000))')
    local elapsed=$((end_time - start_time))
    
    # Output CSV row
    echo "$task_id,$status,$elapsed,$checks_passed,$checks_total,\"$detail\""
    
    [[ "$VERBOSE" == true ]] && cat "$result_file" 2>/dev/null || true
}

run_all_tasks() {
    echo "task_id,status,elapsed_ms,checks_passed,checks_total,detail"
    
    for task_dir in "$TASKS_DIR"/*/; do
        if [[ -d "$task_dir" ]]; then
            task_id=$(basename "$task_dir")
            run_task "$task_id" || true
        fi
    done
}

# =============================================================================
# Argument Parsing
# =============================================================================

while [[ $# -gt 0 ]]; do
    case $1 in
        --model)
            MODEL="$2"
            shift 2
            ;;
        --harness-config)
            HARNESS_CONFIG="$2"
            shift 2
            ;;
        --verbose|-v)
            VERBOSE=true
            shift
            ;;
        --list)
            list_tasks
            exit 0
            ;;
        --task)
            SINGLE_TASK="$2"
            shift 2
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

# =============================================================================
# Main
# =============================================================================

main() {
    echo -e "${BLUE}Runie Agent Harness${NC}"
    echo -e "Model: ${YELLOW}$MODEL${NC}"
    echo -e "Tasks: ${YELLOW}$TASKS_DIR${NC}"
    echo ""
    
    # Clean temp dir
    rm -rf "$TEMP_DIR"
    mkdir -p "$TEMP_DIR"
    
    if [[ -n "$SINGLE_TASK" ]]; then
        echo -e "${BLUE}Running single task: $SINGLE_TASK${NC}"
        run_task "$SINGLE_TASK"
    else
        echo -e "${BLUE}Running all tasks...${NC}"
        run_all_tasks
    fi
    
    echo ""
    echo -e "${BLUE}Done!${NC}"
}

main
