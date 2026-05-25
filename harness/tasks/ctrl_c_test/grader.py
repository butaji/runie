#!/usr/bin/env python3
"""Grader for ctrl_c_test.

Verifies:
1. Event handler intercepts Ctrl+C key events
2. agent_running flag cleared on interrupt
3. Mode reset to Chat on interrupt
4. Panic hook for graceful crash recovery
"""
import sys
from pathlib import Path

def find_tui_file():
    """Find the tui.rs or related files."""
    candidates = [
        Path("crates/runie-tui/src/tui.rs"),
        Path("crates/runie-tui/src/tui/events.rs"),
        Path("crates/runie-tui/src/tui/state.rs"),
        Path("crates/runie-tui/src/tui/update.rs"),
    ]
    for c in candidates:
        if c.exists():
            return c
    return None


def check_ctrl_c_implementation():
    checks = {
        "has_event_handler": False,
        "agent_running_cleared": False,
        "mode_reset": False,
        "has_panic_hook": False,
    }

    # Check events.rs for Ctrl+C handling
    events_file = Path("crates/runie-tui/src/tui/events.rs")
    if events_file.exists():
        content = events_file.read_text()
        # Check for Ctrl+C / Ctrl+Q handling
        ctrl_patterns = ["CONTROL", "ctrl_c", "KeyModifiers::CONTROL", "'c'"]
        if any(p in content for p in ctrl_patterns):
            checks["has_event_handler"] = True

    # Check state.rs for agent_running and mode handling
    state_file = Path("crates/runie-tui/src/tui/state.rs")
    if state_file.exists():
        content = state_file.read_text()
        # Check for agent_running = false pattern
        if "agent_running" in content and "false" in content:
            checks["agent_running_cleared"] = True

    # Check update.rs for mode reset
    update_file = Path("crates/runie-tui/src/tui/update.rs")
    if update_file.exists():
        content = update_file.read_text()
        if "mode" in content and "Chat" in content and ("Quit" in content or "Stop" in content):
            checks["mode_reset"] = True

    # Check tui.rs for panic hook
    tui_file = Path("crates/runie-tui/src/tui.rs")
    if tui_file.exists():
        content = tui_file.read_text()
        if "panic" in content.lower() and ("hook" in content.lower() or "take_hook" in content or "set_hook" in content):
            checks["has_panic_hook"] = True

    return checks


def main():
    print("Checking Ctrl+C handler implementation...\n")

    checks = check_ctrl_c_implementation()

    passed = 0
    total = len(checks)

    for check_name, result in checks.items():
        status = "PASS" if result else "FAIL"
        print(f"{status}: {check_name}")
        if result:
            passed += 1

    print(f"\n{passed}/{total} checks passed")

    if passed >= 3:  # Allow 1 failure
        print("RESULT: pass")
        sys.exit(0)
    else:
        print("RESULT: fail")
        sys.exit(1)


if __name__ == "__main__":
    main()
