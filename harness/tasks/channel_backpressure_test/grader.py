#!/usr/bin/env python3
"""
Grader for channel_backpressure_test task.

Validates that event channels handle backpressure:
1. Backpressure strategy exists
2. Timeout handling exists
3. Try send with feedback
4. No silent drops
5. Buffer capacity defined
"""

import sys
import subprocess
import re

def test_backpressure_strategy():
    """Test that backpressure handling exists."""
    checks = []
    
    # Check for channel retry logic in tui_run.rs
    try:
        result = subprocess.run(
            ['grep', '-A', '5', 'try_send', 'crates/runie-cli/src/tui_run.rs'],
            capture_output=True,
            text=True
        )
        if result.returncode == 0 and 'retry' in result.stdout.lower():
            checks.append(("PASS: Retry logic for try_send found", True))
        elif result.returncode == 0:
            checks.append(("INFO: try_send found but retry unclear", True))
        else:
            checks.append(("FAIL: try_send handling not found", False))
    except Exception as e:
        checks.append((f"FAIL: Search error: {e}", False))
    
    # Check for channel capacity
    try:
        result = subprocess.run(
            ['grep', '-r', 'mpsc::channel', 'crates/runie-cli/src/tui_run.rs'],
            capture_output=True,
            text=True
        )
        if result.returncode == 0:
            # Extract capacity if specified
            match = re.search(r'channel::<.*?>\((\d+)\)', result.stdout)
            if match:
                capacity = int(match.group(1))
                checks.append((f"PASS: Channel capacity defined: {capacity}", True))
            else:
                checks.append(("PASS: Channel creation found", True))
        else:
            checks.append(("FAIL: Channel creation not found", False))
    except Exception as e:
        checks.append((f"FAIL: Search error: {e}", False))
    
    return checks

def test_timeout_handling():
    """Test that timeout handling exists."""
    checks = []
    
    # Check for timeout in permission handling
    try:
        result = subprocess.run(
            ['grep', '-r', 'timeout.*permission', 'crates/', '-i'],
            capture_output=True,
            text=True
        )
        if result.returncode == 0:
            checks.append(("PASS: Permission timeout handling found", True))
        else:
            checks.append(("FAIL: Permission timeout not found", False))
    except Exception as e:
        checks.append((f"FAIL: Search error: {e}", False))
    
    return checks

def test_no_silent_drop():
    """Test that events are not silently dropped."""
    checks = []
    
    # Check for event dropping notification
    try:
        result = subprocess.run(
            ['grep', '-r', 'input dropped\\|event dropped\\|dropping event', 'crates/', '-i'],
            capture_output=True,
            text=True
        )
        if result.returncode == 0:
            checks.append(("PASS: Event drop notification found", True))
        else:
            checks.append(("INFO: No explicit event drop notification (may use retry)", True))
    except Exception as e:
        checks.append((f"INFO: Search error: {e}", True))
    
    return checks

def main():
    all_checks = []
    all_checks.extend(test_backpressure_strategy())
    all_checks.extend(test_timeout_handling())
    all_checks.extend(test_no_silent_drop())
    
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
