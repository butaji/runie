#!/usr/bin/env python3
"""
SWE-bench-style grader for: param_struct task.

Pass conditions:
  1. ServiceConfig struct is defined (pub struct ServiceConfig)
  2. Struct has fields: host, port, timeout_ms, max_connections, debug_mode
  3. Struct derives Debug and Clone
  4. init_service takes exactly one parameter (config: ServiceConfig)
  5. init_service returns Result<T, E>
"""

import sys
import re
from pathlib import Path

TASK_DIR = Path(__file__).parent
SRC_FILE = TASK_DIR / "src" / "lib.rs"

REQUIRED_FIELDS = {"host", "port", "timeout_ms", "max_connections", "debug_mode"}


def load_source() -> str:
    if not SRC_FILE.exists():
        return ""
    return SRC_FILE.read_text()


def test_has_config_struct(source: str) -> bool:
    return bool(re.search(r'pub struct ServiceConfig\b', source))


def extract_struct_body(source: str) -> str | None:
    m = re.search(
        r'pub struct ServiceConfig\b(.*?)}(?:\s*$|\n(?:pub |#|\Z))',
        source,
        re.DOTALL,
    )
    if m:
        return m.group(0)
    # Try simpler extraction
    lines = source.splitlines()
    in_struct = False
    lines_out = []
    for line in lines:
        if 'pub struct ServiceConfig' in line:
            in_struct = True
        if in_struct:
            lines_out.append(line)
            if line.strip().startswith('}') and not line.strip().endswith('{'):
                break
    return '\n'.join(lines_out) if lines_out else None


def test_struct_has_required_fields(struct_body: str | None) -> bool:
    if struct_body is None:
        return False
    # Remove derive line for field checking
    body_no_derive = re.sub(r'#\[derive\([^\]]+\)\]', '', struct_body)
    found = set()
    for field in REQUIRED_FIELDS:
        if field in body_no_derive:
            found.add(field)
    return found == REQUIRED_FIELDS


def test_struct_derives_debug_clone(source: str) -> bool:
    m = re.search(r'pub struct ServiceConfig\b', source)
    if not m:
        return False
    # Look for derive line immediately before or with blank lines
    start = m.start()
    chunk = source[max(0, start - 200):start]
    derive_m = re.search(r'#\[derive\([^\)]+\)\]', chunk)
    if not derive_m:
        return False
    derives = derive_m.group(0)
    return 'Debug' in derives and 'Clone' in derives


def test_function_takes_single_param(source: str) -> bool:
    # init_service(config: ServiceConfig) — one param, no others
    pattern = r'pub fn init_service\s*\(\s*config:\s*ServiceConfig\s*\)'
    return bool(re.search(pattern, source))


def test_function_returns_result(source: str) -> bool:
    # init_service(...) -> Result<...>
    pattern = r'pub fn init_service[^)]+\)\s*->\s*Result<'
    return bool(re.search(pattern, source))


def grade() -> tuple[bool, str]:
    source = load_source()
    if not source:
        return False, "FAIL: src/lib.rs not found"

    struct_body = extract_struct_body(source)

    checks = [
        (test_has_config_struct(source), "has ServiceConfig struct"),
        (test_struct_has_required_fields(struct_body), f"struct has all {len(REQUIRED_FIELDS)} required fields"),
        (test_struct_derives_debug_clone(source), "struct derives Debug and Clone"),
        (test_function_takes_single_param(source), "init_service takes single ServiceConfig param"),
        (test_function_returns_result(source), "init_service returns Result"),
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
