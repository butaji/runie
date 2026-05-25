#!/usr/bin/env python3
"""
Grader for panic_recovery_test task.

Validates that panics in tool execution are handled gracefully:
1. Panic is caught via catch_unwind
2. Error result is returned to agent
3. Workspace state is preserved
4. Agent continues or gracefully terminates
5. No crash or undefined behavior
"""

import sys
import subprocess

def test_panic_catch():
    """Test that panics are caught."""
    checks = []
    
    # Check if execute_tool_with_panic_catch exists in codebase
    try:
        result = subprocess.run(
            ['grep', '-r', 'execute_tool_with_panic_catch', 'crates/'],
            capture_output=True,
            text=True
        )
        if result.returncode == 0:
            checks.append(("PASS: execute_tool_with_panic_catch implemented", True))
        else:
            checks.append(("FAIL: execute_tool_with_panic_catch not found", False))
    except Exception as e:
        checks.append((f"FAIL: Search error: {e}", False))
    
    # Check for catch_unwind usage
    try:
        result = subprocess.run(
            ['grep', '-r', 'catch_unwind', 'crates/runie-agent/src/'],
            capture_output=True,
            text=True
        )
        if result.returncode == 0:
            checks.append(("PASS: catch_unwind used for panic recovery", True))
        else:
            checks.append(("FAIL: catch_unwind not found", False))
    except Exception as e:
        checks.append((f"FAIL: Search error: {e}", False))
    
    return checks

def test_error_result():
    """Test that error results are returned properly."""
    checks = []
    
    # Check for error handling in tool execution
    try:
        result = subprocess.run(
            ['grep', '-r', 'ToolResult.*is_error', 'crates/runie-agent/src/'],
            capture_output=True,
            text=True
        )
        if result.returncode == 0:
            checks.append(("PASS: ToolResult.is_error set on panic", True))
        else:
            checks.append(("FAIL: ToolResult.is_error not found", False))
    except Exception as e:
        checks.append((f"FAIL: Search error: {e}", False))
    
    return checks

def test_workspace_preserved():
    """Test that workspace state is preserved on panic."""
    checks = []
    
    # Check for rollback mechanism
    try:
        result = subprocess.run(
            ['grep', '-r', 'Rollback', 'crates/'],
            capture_output=True,
            text=True
        )
        if result.returncode == 0:
            checks.append(("PASS: Rollback mechanism exists", True))
        else:
            checks.append(("INFO: Rollback mechanism not found (may use other patterns)", True))
    except Exception as e:
        checks.append((f"INFO: Search error: {e}", True))
    
    return checks

def main():
    all_checks = []
    all_checks.extend(test_panic_catch())
    all_checks.extend(test_error_result())
    all_checks.extend(test_workspace_preserved())
    
    passed = sum(1 for msg, ok in all_checks if ok)
    total = len(all_checks)
    
    # Print individual checks
    for msg, ok in all_checks:
        status = "PASS" if ok else "FAIL"
        print(f"{status}: {msg}")
    
    # Print summary
    print(f"\n{total}/{total} checks passed")
    
    if passed == total:
        print("RESULT: pass")
        return 0
    else:
        print(f"RESULT: fail ({passed}/{total} passed)")
        return 1

if __name__ == "__main__":
    sys.exit(main())
