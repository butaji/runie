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

def find_message_list():
    """Find the message_list render file in various locations."""
    candidates = [
        # From workspace root (when running in repo)
        Path("crates/runie-tui/src/components/message_list/render.rs"),
        Path("message_list.rs"),
        # In sandbox
        Path("message_list.rs"),
        # Relative to script location
        Path(__file__).parent.parent.parent / "crates/runie-tui/src/components/message_list/render.rs",
    ]

    for candidate in candidates:
        if candidate.exists():
            return candidate

    # Try globbing in current directory
    for f in Path(".").glob("**/message_list/render.rs"):
        return f

    return None


def check_source():
    checks = {
        "has_empty_check": False,
        "has_greeting": False,
        "has_shortcuts_hint": False,
        "has_cta": False,
    }

    src = find_message_list()
    if not src:
        print("WARNING: Could not find message_list/render.rs")
        return checks

    content = src.read_text()

    # Check for empty guard at start of render_ref
    # Look for: if self.items.is_empty() or if items.is_empty()
    if "is_empty()" in content:
        checks["has_empty_check"] = True

    # Check for greeting/welcome text in the empty state
    greeting_patterns = [
        "No messages", "Start typing", "Welcome", "hello",
        "Start a conversation", "empty_state", "Empty state",
        "No messages yet", "Type your first"
    ]
    if any(p.lower() in content.lower() for p in greeting_patterns):
        checks["has_greeting"] = True

    # Check for keyboard shortcut hints
    hint_patterns = ["Enter", "shortcut", "hint", "^k", "^b", "scroll", "commands"]
    if any(p.lower() in content.lower() for p in hint_patterns):
        checks["has_shortcuts_hint"] = True

    # Check for CTA (call to action)
    cta_patterns = ["Press Enter", "Start", "begin", "Type", "Send a message", "Send"]
    if any(p.lower() in content.lower() for p in cta_patterns):
        checks["has_cta"] = True

    return checks


def main():
    print("Checking MessageList empty state implementation...")
    print()

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
