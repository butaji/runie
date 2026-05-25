#!/usr/bin/env python3
"""
Grader for cancellation_clean_state task.

Validates that cancellation properly:
1. Aborts the running agent task
2. Resets agent_running flag
3. Returns to Chat mode
4. Preserves workspace integrity
5. Allows new submissions
"""
import sys
import re
from pathlib import Path

# Project root is the working directory when this script runs
PROJECT_ROOT = Path.cwd()

def check_file_content(relative_path):
    """Check if a file exists and return its content."""
    full_path = PROJECT_ROOT / relative_path
    if full_path.exists():
        return full_path.read_text()
    return ""

def check_cancellation_handling():
    checks = {
        "task_aborted": False,
        "agent_running_reset": False,
        "mode_returns_to_chat": False,
        "workspace_preserved": False,
        "new_submit_allowed": False,
    }

    # Check tui_run.rs for interrupt handling
    tui_run_content = check_file_content("crates/runie-cli/src/tui_run.rs")
    if tui_run_content:
        # Check for task abort
        if "handle.abort()" in tui_run_content or "task.abort()" in tui_run_content:
            checks["task_aborted"] = True
        # Check for interrupt command
        if "Cmd::Interrupt" in tui_run_content:
            checks["task_aborted"] = True

    # Check update/agent.rs for agent_running reset
    agent_update_content = check_file_content("crates/runie-tui/src/tui/update/agent.rs")
    if agent_update_content:
        if re.search(r'agent_running\s*=\s*false', agent_update_content):
            checks["agent_running_reset"] = True

    # Also check update.rs
    update_content = check_file_content("crates/runie-tui/src/tui/update.rs")
    if update_content:
        if re.search(r'agent_running\s*=\s*false', update_content):
            checks["agent_running_reset"] = True

    # Check state.rs for mode handling
    state_content = check_file_content("crates/runie-tui/src/tui/state.rs")
    if state_content:
        if "mode" in state_content and "Chat" in state_content:
            checks["mode_returns_to_chat"] = True

    # Check for Stop/Cancel handling in update.rs
    if update_content:
        if re.search(r'Msg::(Stop|Quit)', update_content):
            checks["new_submit_allowed"] = True

    # Check for workspace rollback mechanism
    workspace_content = check_file_content("crates/runie-tools/src/workspace.rs")
    if workspace_content:
        if "rollback" in workspace_content.lower() or "preserve" in workspace_content.lower():
            checks["workspace_preserved"] = True

    return checks


def main():
    print("Checking cancellation clean state implementation...\n")

    checks = check_cancellation_handling()

    passed = 0
    total = len(checks)

    for check_name, result in checks.items():
        status = "PASS" if result else "FAIL"
        print(f"{status}: {check_name}")
        if result:
            passed += 1

    print(f"\n{passed}/{total} checks passed")

    # Need at least 4/5 for pass
    if passed >= 4:
        print("RESULT: pass")
        sys.exit(0)
    else:
        print("RESULT: fail")
        sys.exit(1)


if __name__ == "__main__":
    main()
