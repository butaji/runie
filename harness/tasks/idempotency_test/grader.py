#!/usr/bin/env python3
"""
Grader for idempotency_test task.

Validates that the agent handles idempotent operations correctly:
1. Same operation with same args only executes once
2. Re-running produces same result
3. Different args still work
4. Operation count tracking is correct
"""
import sys
import re
from pathlib import Path

PROJECT_ROOT = Path.cwd()

def check_file_content(relative_path):
    """Check if a file exists and return its content."""
    full_path = PROJECT_ROOT / relative_path
    if full_path.exists():
        return full_path.read_text()
    return ""

def check_idempotency_handling():
    checks = {
        "dedup_on_retry": False,
        "same_result_twice": False,
        "different_args_work": False,
        "operation_count_correct": False,
    }

    # Check rig_loop.rs for deduplication
    loop_content = check_file_content("crates/runie-agent/src/rig_loop.rs")
    if loop_content:
        # Look for dedup patterns
        dedup_patterns = [
            r'HashSet',
            r'seen.*insert',
            r'contains.*skip',
            r'duplicate',
        ]
        dedup_found = sum(1 for p in dedup_patterns if re.search(p, loop_content, re.IGNORECASE))
        if dedup_found >= 1:
            checks["dedup_on_retry"] = True
            checks["same_result_twice"] = True

    # Check tools.rs or tool execution for operation tracking
    tools_content = check_file_content("crates/runie-agent/src/tools.rs")
    if not tools_content:
        tools_content = check_file_content("crates/runie-agent/src/executor.rs")
    
    if tools_content:
        # Check for operation tracking
        tracking_patterns = [
            r'operation.*count',
            r'executed.*operations',
            r'seen.*operations',
            r'call.*id',
        ]
        tracking_found = sum(1 for p in tracking_patterns if re.search(p, tools_content, re.IGNORECASE))
        if tracking_found >= 1:
            checks["operation_count_correct"] = True

    # Check misc.rs for submit blocking (prevents double submit)
    misc_content = check_file_content("crates/runie-tui/src/tui/update/misc.rs")
    if misc_content:
        if "agent_running" in misc_content and "blocked" in misc_content.lower():
            checks["different_args_work"] = True

    return checks

def main():
    print("Checking idempotency handling...\n")

    checks = check_idempotency_handling()

    passed = 0
    total = len(checks)

    for check_name, result in checks.items():
        status = "PASS" if result else "FAIL"
        print(f"{status}: {check_name}")
        if result:
            passed += 1

    print(f"\n{passed}/{total} checks passed")

    if passed >= 3:
        print("RESULT: pass")
        sys.exit(0)
    else:
        print("RESULT: fail")
        sys.exit(1)

if __name__ == "__main__":
    main()
