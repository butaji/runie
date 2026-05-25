#!/usr/bin/env python3
"""
SWE-bench-style grader for: error_recovery task.

Pass conditions:
  1. fetch_data has a retry loop
  2. Has exponential backoff
  3. Maximum 3 retries
  4. Returns Result type
"""

import sys
import re
from pathlib import Path

TASK_DIR = Path(__file__).parent
SRC_FILE = TASK_DIR / "src" / "api.rs"


def load_source() -> str:
    if not SRC_FILE.exists():
        return ""
    return SRC_FILE.read_text()


def test_has_retry_loop(source: str) -> tuple[bool, str]:
    """Check for retry loop (for/while with counter or iterator)"""
    patterns = [
        r'for\s+\w+\s+in\s+\d+\.\.',
        r'let\s+mut\s+\w+\s*=\s*\d+.*while',
        r'for\s+\w+\s+in\s+0\.\.\3',
    ]
    has_loop = any(re.search(p, source) for p in patterns)
    return has_loop, "has retry loop"


def test_has_backoff(source: str) -> tuple[bool, str]:
    """Check for exponential backoff (multiply by 2, or sleep with duration)"""
    patterns = [
        r'\*\s*2',
        r'duration.*\*\s*2',
        r'sleep.*\*',
        r'pow\(',
    ]
    has_backoff = any(re.search(p, source) for p in patterns)
    return has_backoff, "has exponential backoff"


def test_max_retries_3(source: str) -> tuple[bool, str]:
    """Check that max retries is 3"""
    patterns = [
        r'<\s*3',
        r'!=\s*3',
        r'<=\s*3',
        r'\.\.\3',
    ]
    has_limit = any(re.search(p, source) for p in patterns)
    return has_limit, "max retries is 3"


def test_returns_result(source: str) -> tuple[bool, str]:
    """Check that function returns Result"""
    pattern = r'fn\s+fetch_data.*->\s*Result<'
    return bool(re.search(pattern, source)), "returns Result<T, E>"


def grade() -> tuple[bool, str]:
    source = load_source()
    if not source:
        return False, "FAIL: src/api.rs not found"

    checks = [
        test_has_retry_loop(source),
        test_has_backoff(source),
        test_max_retries_3(source),
        test_returns_result(source),
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
