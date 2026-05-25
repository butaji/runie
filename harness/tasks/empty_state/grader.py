#!/usr/bin/env python3
"""Grader for empty_state_test.

Verifies that MessageList::render_ref handles empty state with:
1. Empty check guard at start of render
2. Greeting/guidance text
3. Keyboard shortcut hints
4. CTA to start conversation
"""
import sys
from pathlib import Path

def check_source():
    # Check for the source file in the workspace
    src = Path("src/message_list.rs")
    if not src.exists():
        # Try finding in workspace
        src = Path("message_list.rs")

    if not src.exists():
        return False, "message_list.rs not found"

    content = src.read_text()

    checks = {
        "has_empty_check": False,
        "has_greeting": False,
        "has_shortcuts_hint": False,
        "has_cta": False,
    }

    # Check for empty guard: items.is_empty() or items.is_empty()
    if "is_empty()" in content and ("for item in" in content or "iter()" in content):
        checks["has_empty_check"] = True

    # Check for greeting text
    greeting_patterns = ["No messages", "Start typing", "Welcome", "hello", "Start a conversation"]
    if any(p.lower() in content.lower() for p in greeting_patterns):
        checks["has_greeting"] = True

    # Check for keyboard shortcut hints
    hint_patterns = ["Enter", "shortcut", "hint", "^k", "^b", "scroll", "commands"]
    if any(p.lower() in content.lower() for p in hint_patterns):
        checks["has_shortcuts_hint"] = True

    # Check for CTA
    cta_patterns = ["Press Enter", "Start", "begin", "Type", "Send a message"]
    if any(p.lower() in content.lower() for p in cta_patterns):
        checks["has_cta"] = True

    return checks


def main():
    checks = check_source()

    passed = 0
    total = len(checks)

    for check_name, result in checks.items():
        status = "PASS" if result else "FAIL"
        print(f"{status}: {check_name}")
        if result:
            passed += 1

    print(f"\n{passed}/{total} checks passed")

    if passed == total:
        print("RESULT: pass")
        sys.exit(0)
    else:
        print("RESULT: fail")
        sys.exit(1)


if __name__ == "__main__":
    main()
