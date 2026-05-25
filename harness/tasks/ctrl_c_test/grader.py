#!/usr/bin/env python3
"""Grader for ctrl_c_test.

Verifies:
1. Signal handler registration in Tui::new or similar
2. agent_running flag cleared on interrupt
3. Mode reset to Chat on interrupt
4. No conflict with existing panic hook
"""
import sys
import subprocess
from pathlib import Path

def check_signal_handler():
    """Check if signal handler is registered in tui.rs."""
    tui_file = Path("crates/runie-tui/src/tui.rs")
    if not tui_file.exists():
        # Try in harness workspace
        tui_file = Path("tui.rs")

    if not tui_file.exists():
        return None

    content = tui_file.read_text()

    checks = {
        "has_signal_handler": False,
        "agent_running_cleared": False,
        "mode_reset": False,
        "no_panic_hook_conflict": False,
    }

    # Check for signal handler: ctrl_c, signal, SIGINT, sig_hook
    signal_patterns = ["ctrl_c()", "signal::ctrl_c", "SIGINT", "signal::signal"]
    if any(p in content for p in signal_patterns):
        checks["has_signal_handler"] = True

    # Check for agent_running = false on interrupt
    if "agent_running" in content and ("= false" in content or "= !running" in content):
        checks["agent_running_cleared"] = True

    # Check for mode reset: mode = TuiMode::Chat or similar
    mode_reset_patterns = ["mode = TuiMode::Chat", "mode: Chat", "Chat,"]
    if any(p in content for p in mode_reset_patterns):
        checks["mode_reset"] = True

    # Check that panic hook and signal handler are separate
    if "panic_hook" in content and "ctrl_c" in content:
        checks["no_panic_hook_conflict"] = True
    elif "panic_hook" not in content:
        checks["no_panic_hook_conflict"] = True  # No conflict if no panic hook

    return checks


def check_compilation():
    """Verify the code compiles with cargo check."""
    result = subprocess.run(
        ["cargo", "check", "--all-targets"],
        capture_output=True,
        text=True,
        cwd=".",
        timeout=120
    )
    return result.returncode == 0


def main():
    print("Checking Ctrl+C handler implementation...\n")

    checks = check_signal_handler()

    if checks is None:
        print("FAIL: tui.rs not found in workspace")
        print("RESULT: fail")
        sys.exit(1)

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
