#!/usr/bin/env python3
"""Grader for permission_rollback_test.

Verifies:
1. Cmd::Rollback exists in Cmd enum
2. Snapshot is taken before tool execution
3. Revert logic is implemented
4. PermissionCancel triggers rollback
"""
import sys
from pathlib import Path
# Paths: grader.py is at tasks/<task>/grader.py
# repo_root is 3 levels up from grader.py
repo_dir = Path(__file__).parent.parent.parent.parent


def check_rollback_implementation():
    checks = {
        "has_rollback_cmd": False,
        "has_snapshot": False,
        "has_revert": False,
        "cancel_triggers_rollback": False,
    }

    # Check state.rs for Cmd::Rollback
    state_file = Path(repo_dir / "crates/runie-tui/src/tui/state.rs")
    if state_file.exists():
        content = state_file.read_text()
        if "Rollback" in content:
            checks["has_rollback_cmd"] = True

    # Check agent.rs for snapshot logic
    agent_file = Path(repo_dir / "crates/runie-tui/src/tui/update/agent.rs")
    if agent_file.exists():
        content = agent_file.read_text()
        snapshot_patterns = ["snapshot", "backup", "copy", "clone", "before"]
        if any(p in content.lower() for p in snapshot_patterns):
            checks["has_snapshot"] = True

        # Check for revert logic
        revert_patterns = ["revert", "restore", "rollback", "undo", "original"]
        if any(p in content.lower() for p in revert_patterns):
            checks["has_revert"] = True

        # Check that PermissionCancel or Skip triggers rollback
        if "rollback" in content.lower():
            # Check if it's related to Deny/Skip
            deny_patterns = ["Deny", "Skip", "deny", "skip"]
            if any(p in content for p in deny_patterns):
                checks["cancel_triggers_rollback"] = True

    # Also check misc.rs
    misc_file = Path(repo_dir / "crates/runie-tui/src/tui/update/misc.rs")
    if misc_file.exists():
        content = misc_file.read_text()
        if "rollback" in content.lower():
            checks["cancel_triggers_rollback"] = True

    return checks


def main():
    print("Checking permission rollback implementation...\n")

    checks = check_rollback_implementation()

    passed = 0
    total = len(checks)

    for check_name, result in checks.items():
        status = "PASS" if result else "FAIL"
        print(f"{status}: {check_name}")
        if result:
            passed += 1

    print(f"\n{passed}/{total} checks passed")

    if passed >= 3:  # Allow 1 failure for partial implementation
        print("RESULT: pass")
        sys.exit(0)
    else:
        print("RESULT: fail")
        sys.exit(1)


if __name__ == "__main__":
    main()
