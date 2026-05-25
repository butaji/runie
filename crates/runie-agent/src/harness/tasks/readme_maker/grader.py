#!/usr/bin/env python3
"""
SWE-bench-style grader for: readme_maker task.

Pass conditions:
  1. README.md exists and has a title (# ...)
  2. Has an Installation section (## Installation or similar)
  3. Has a Usage section with code block (```)
  4. Has a License section
  5. Under 30 lines total
"""

import sys
import re
from pathlib import Path

TASK_DIR = Path(__file__).parent
README = TASK_DIR / "README.md"


def load_readme() -> str:
    if not README.exists():
        return ""
    return README.read_text()


def test_has_title(text: str) -> bool:
    return bool(re.search(r'^#\s+\w', text, re.MULTILINE))


def test_has_installation(text: str) -> bool:
    text_lower = text.lower()
    return 'installation' in text_lower or 'install' in text_lower


def test_has_usage(text: str) -> bool:
    return '```' in text  # Has a code block


def test_has_license(text: str) -> bool:
    text_lower = text.lower()
    return 'license' in text_lower


def test_under_30_lines(text: str) -> bool:
    lines = [l for l in text.splitlines() if l.strip()]
    return len(lines) <= 30


def grade() -> tuple[bool, str]:
    text = load_readme()
    if not text:
        return False, "FAIL: README.md not found"

    checks = [
        (test_has_title(text), "has title (# heading)"),
        (test_has_installation(text), "has installation section"),
        (test_has_usage(text), "has usage code block"),
        (test_has_license(text), "has license section"),
        (test_under_30_lines(text), "under 30 non-empty lines"),
    ]

    passed = sum(1 for ok, _ in checks if ok)
    details = "\n".join(f"{'PASS' if ok else 'FAIL'}: {desc}" for ok, desc in checks)

    if all(ok for ok, _ in checks):
        return True, f"PASS: all {len(checks)} checks passed\n{details}"
    else:
        return False, f"FAIL: {passed}/{len(checks)} checks passed\n{details}"


if __name__ == "__main__":
    ok, msg = grade()
    print(msg)
    sys.exit(0 if ok else 1)
