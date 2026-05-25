#!/usr/bin/env python3
"""
Grader for graceful_degradation task.

Validates that the system degrades gracefully when components fail:
1. Main content works without sidebar
2. Error recovery allows continuation
3. No panic on component failure
4. Fallback rendering exists
"""
import sys
import re
from pathlib import Path

PROJECT_ROOT = Path.cwd()

def check_file_content(relative_path):
    """Check if a file exists and return its content."""
    full_path = PROJECT_ROOT / relative_path
    if full_path.exists():
        return full_path.read_text()
    return ""

def check_graceful_degradation():
    checks = {
        "main_content_works_without_sidebar": False,
        "error_recovery_allows_continue": False,
        "no_panic_on_component_failure": False,
        "fallback_rendering_exists": False,
    }

    # Check tui.rs for conditional sidebar rendering
    tui_content = check_file_content("crates/runie-tui/src/tui.rs")
    if tui_content:
        # Sidebar should be optional
        if "show_sidebar" in tui_content:
            checks["main_content_works_without_sidebar"] = True

        # Fallback when sidebar unavailable
        fallback_patterns = [
            r'if.*show_sidebar',
            r'show_sidebar.*&&',
            r'!show_sidebar',
        ]
        fallback_found = sum(1 for p in fallback_patterns if re.search(p, tui_content, re.IGNORECASE))
        if fallback_found >= 1:
            checks["fallback_rendering_exists"] = True

    # Check agent.rs for error recovery
    agent_content = check_file_content("crates/runie-tui/src/tui/update/agent.rs")
    if agent_content:
        # Error should not panic
        if "recoverable" in agent_content or "Error" in agent_content:
            # Error handling exists
            checks["error_recovery_allows_continue"] = True

    # Check loop_engine.rs for panic handling
    loop_content = check_file_content("crates/runie-agent/src/loop_engine.rs")
    if loop_content:
        # Panic recovery pattern
        panic_patterns = [
            r'catch_unwind',
            r'panic.*hook',
            r'result.*unwrap_or',
            r'result.*map_err',
        ]
        panic_found = sum(1 for p in panic_patterns if re.search(p, loop_content, re.IGNORECASE))
        if panic_found >= 1:
            checks["no_panic_on_component_failure"] = True

    # Check tui.rs for panic hook
    tui_content = check_file_content("crates/runie-tui/src/tui.rs")
    if tui_content:
        if "panic" in tui_content.lower() and "hook" in tui_content.lower():
            checks["no_panic_on_component_failure"] = True

    return checks

def main():
    print("Checking graceful degradation patterns...\n")

    checks = check_graceful_degradation()

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
