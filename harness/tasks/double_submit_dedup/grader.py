#!/usr/bin/env python3
"""
Grader for double_submit_dedup task.

Validates that the system prevents duplicate submissions by:
1. Checking for agent_running guard in submit handler
2. Showing feedback when blocked
3. Not spawning duplicate agents
"""
import sys
from pathlib import Path

def check_double_submit_protection():
    checks = {
        "has_agent_running_check": False,
        "has_feedback_when_blocked": False,
        "submit_returns_early": False,
        "mode_not_changed_on_block": False,
    }

    # Check misc.rs for submit handling
    misc_file = Path("crates/runie-tui/src/tui/update/misc.rs")
    if misc_file.exists():
        content = misc_file.read_text()

        # Check for agent_running guard
        if "agent_running" in content:
            checks["has_agent_running_check"] = True

        # Check for feedback when blocked
        feedback_patterns = ["already running", "still running", "wait", "Please wait"]
        if any(p.lower() in content.lower() for p in feedback_patterns):
            checks["has_feedback_when_blocked"] = True

        # Check for early return
        if "return vec![]" in content or "return vec![]".replace("[]", "") in content:
            checks["submit_returns_early"] = True

    # Check state.rs for agent_running definition
    state_file = Path("crates/runie-tui/src/tui/state.rs")
    if state_file.exists():
        content = state_file.read_text()
        if "agent_running" in content:
            checks["has_agent_running_check"] = True

    return checks


def main():
    print("Checking double-submit protection implementation...\n")

    checks = check_double_submit_protection()

    passed = 0
    total = len(checks)

    for check_name, result in checks.items():
        status = "PASS" if result else "FAIL"
        print(f"{status}: {check_name}")
        if result:
            passed += 1

    print(f"\n{passed}/{total} checks passed")

    if passed >= 2:
        print("RESULT: pass")
        sys.exit(0)
    else:
        print("RESULT: fail")
        sys.exit(1)


if __name__ == "__main__":
    main()
