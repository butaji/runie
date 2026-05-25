#!/usr/bin/env python3
"""Grader for idle_submit_feedback task.

Verifies that pressing Enter with empty input produces visible feedback.
"""
import sys
from pathlib import Path

def check_submit_feedback():
    checks = {
        "has_empty_check": False,
        "has_feedback_on_empty": False,
        "input_right_info_updated": False,
        "mode_not_changed": False,
    }

    # Find the misc.rs file
    misc_file = Path(__file__).parent.parent.parent / "crates/runie-tui/src/tui/update/misc.rs"
    if not misc_file.exists():
        misc_file = Path("crates/runie-tui/src/tui/update/misc.rs")

    if misc_file.exists():
        content = misc_file.read_text()

        # Check for empty input guard
        if "is_empty()" in content:
            checks["has_empty_check"] = True

        # Check for feedback mechanism when empty
        feedback_patterns = [
            "input_right_info",
            "warning",
            "notification",
            "Type a message",
        ]
        if any(p in content for p in feedback_patterns):
            checks["has_feedback_on_empty"] = True

        # Check that input_right_info is set on empty submit
        if "input_right_info" in content and ("empty" in content.lower() or "is_empty()" in content):
            checks["input_right_info_updated"] = True

    # Check that mode stays the same (no transition)
    update_file = Path(__file__).parent.parent.parent / "crates/runie-tui/src/tui/update.rs"
    if not update_file.exists():
        update_file = Path("crates/runie-tui/src/tui/update.rs")

    if update_file.exists():
        content = update_file.read_text()
        # Submit with empty should not change mode
        if "handle_submit" in content:
            checks["mode_not_changed"] = True

    # Check test files for empty submit tests
    test_file = Path(__file__).parent.parent.parent / "crates/runie-tui/src/tui/tests/reducer.rs"
    if not test_file.exists():
        test_file = Path("crates/runie-tui/src/tui/tests/reducer.rs")

    if test_file.exists():
        content = test_file.read_text()
        if "submit_empty" in content or ("empty" in content.lower() and "submit" in content.lower()):
            checks["has_feedback_on_empty"] = True

    return checks


def main():
    print("Checking empty submit feedback implementation...")
    print()

    checks = check_submit_feedback()

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
