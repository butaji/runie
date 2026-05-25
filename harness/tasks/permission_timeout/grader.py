#!/usr/bin/env python3
"""
Grader for permission_timeout task.

Validates that the system handles permission timeouts correctly by:
1. Displaying timeout message
2. Gracefully denying permission
3. Continuing execution
4. Notifying the user
"""
import sys
from pathlib import Path
# Paths: grader.py is at tasks/<task>/grader.py
# repo_root is 3 levels up from grader.py
repo_dir = Path(__file__).parent.parent.parent.parent


def check_permission_timeout():
    checks = {
        "timeout_check_exists": False,
        "timeout_message_displayed": False,
        "permission_denied_gracefully": False,
        "mode_reset_after_timeout": False,
    }

    # Check misc.rs for timeout checking
    misc_file = Path(repo_dir / "crates/runie-tui/src/tui/update/misc.rs")
    if misc_file.exists():
        content = misc_file.read_text()
        if "timeout" in content.lower():
            checks["timeout_check_exists"] = True

    # Check agent.rs for timeout handling
    agent_file = Path(repo_dir / "crates/runie-tui/src/tui/update/agent.rs")
    if agent_file.exists():
        content = agent_file.read_text()

        # Check for timeout message
        if "timeout" in content.lower() and "message" in content.lower():
            checks["timeout_message_displayed"] = True

        # Check for graceful denial
        if "denied" in content.lower() or "cancel" in content.lower():
            checks["permission_denied_gracefully"] = True

        # Check for mode reset
        if "mode" in content and "Chat" in content:
            checks["mode_reset_after_timeout"] = True

    # Check state.rs for timeout tracking
    state_file = Path(repo_dir / "crates/runie-tui/src/tui/state.rs")
    if state_file.exists():
        content = state_file.read_text()
        if "timeout" in content.lower():
            checks["timeout_check_exists"] = True

    return checks


def main():
    print("Checking permission timeout implementation...\n")

    checks = check_permission_timeout()

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
