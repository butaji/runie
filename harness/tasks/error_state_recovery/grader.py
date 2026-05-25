#!/usr/bin/env python3
"""
Grader for error_state_recovery task.

Validates that the agent correctly handles errors by:
1. Displaying error messages to user
2. Returning to Chat mode
3. Preserving scroll position
4. Allowing continuation
"""
import sys
from pathlib import Path

def check_error_handling():
    checks = {
        "error_displayed_to_user": False,
        "mode_returns_to_chat": False,
        "scroll_position_preserved": False,
        "user_can_continue": False,
        "no_panic_on_error": False,
    }

    # Check agent.rs for error handling
    agent_file = Path("crates/runie-tui/src/tui/update/agent.rs")
    if agent_file.exists():
        content = agent_file.read_text()

        # Check for on_agent_error function
        if "on_agent_error" in content or "AgentEvent::Error" in content:
            checks["error_displayed_to_user"] = True

        # Check for mode reset on error
        if "mode" in content and "Chat" in content:
            checks["mode_returns_to_chat"] = True

    # Check state.rs for scroll handling
    state_file = Path("crates/runie-tui/src/tui/state.rs")
    if state_file.exists():
        content = state_file.read_text()
        if "scroll" in content.lower():
            checks["scroll_position_preserved"] = True

    # Check update.rs for continue handling
    update_file = Path("crates/runie-tui/src/tui/update.rs")
    if update_file.exists():
        content = update_file.read_text()
        if "agent_running" in content and "false" in content:
            checks["user_can_continue"] = True

    # Check tui.rs for panic hook
    tui_file = Path("crates/runie-tui/src/tui.rs")
    if tui_file.exists():
        content = tui_file.read_text()
        if "panic" in content.lower() and "hook" in content.lower():
            checks["no_panic_on_error"] = True

    return checks


def main():
    print("Checking error state recovery implementation...\n")

    checks = check_error_handling()

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
