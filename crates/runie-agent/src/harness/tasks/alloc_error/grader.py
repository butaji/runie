#!/usr/bin/env python3
"""
SWE-bench-style grader for: alloc_error task.

Pass conditions:
  1. function signature returns Result<T, E>
  2. No panic!() calls in the function body
  3. Error message includes the offending size value
"""

import sys
import re
import json
from pathlib import Path

TASK_DIR = Path(__file__).parent
SRC_FILE = TASK_DIR / "src" / "lib.rs"


def load_source() -> str:
    if not SRC_FILE.exists():
        return ""
    return SRC_FILE.read_text()


def extract_fn_body(source: str, fn_name: str) -> str | None:
    # Find the function and capture everything up to the next pub fn or end of file
    pattern = rf'pub fn {fn_name}\b.*?\n(.*?)(?=\npub fn |\Z)'
    m = re.search(pattern, source, re.DOTALL)
    if m:
        return m.group(0)
    return None


def test_result_return_type(fn_body: str) -> bool:
    """Check that function signature returns Result"""
    return bool(re.search(r'->\s*Result<', fn_body))


def test_no_panic(fn_body: str) -> bool:
    """Check that there are no panic!() calls in the function body"""
    # Extract just the body (after the -> line)
    body_lines = fn_body.split('\n')
    in_fn_body = False
    brace_count = 0
    body = []
    for line in body_lines:
        if '->' in line:
            in_fn_body = True
        if in_fn_body:
            body.append(line)
            brace_count += line.count('{') - line.count('}')
            if brace_count == 0 and '{' in '\n'.join(body):
                break
    body_str = '\n'.join(body)
    return 'panic!' not in body_str


def test_error_message_includes_size(fn_body: str) -> bool:
    """Check that error paths mention size or the size value"""
    # Find all Err(...) returns
    err_pattern = r'Err\([^)]+\)'
    errors = re.findall(err_pattern, fn_body)
    if not errors:
        return False
    for err in errors:
        err_text = err.lower()
        # Should mention size, usize, or a variable that would contain size
        if any(kw in err_text for kw in ['size', 'usize', '{size}']):
            return True
    return False


def grade() -> tuple[bool, str]:
    source = load_source()
    if not source:
        return False, "FAIL: src/lib.rs not found"

    fn_body = extract_fn_body(source, "allocate_buffer")
    if fn_body is None:
        return False, "FAIL: allocate_buffer function not found"

    checks = [
        (test_result_return_type(fn_body), "function returns Result"),
        (test_no_panic(fn_body), "no panic!() calls in function body"),
        (test_error_message_includes_size(fn_body), "error messages include size info"),
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
