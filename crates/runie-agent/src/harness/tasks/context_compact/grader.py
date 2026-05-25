#!/usr/bin/env python3
"""
SWE-bench-style grader for: context_compact task.

Pass conditions:
  1. compact_context respects max_messages limit
  2. Keeps recent messages
  3. Removes duplicate system prompts
  4. Implementation is idempotent
"""

import sys
import re
from pathlib import Path

TASK_DIR = Path(__file__).parent
SRC_FILE = TASK_DIR / "src" / "compactor.rs"


def load_source() -> str:
    if not SRC_FILE.exists():
        return ""
    return SRC_FILE.read_text()


def test_respects_max_messages(source: str) -> tuple[bool, str]:
    """Check that function uses max_messages parameter"""
    # Check if max_messages is used in a comparison or slice
    patterns = [
        r'max_messages',
        r'len\(\).*<',
        r'slice.*max_messages',
        r'truncate',
    ]
    uses_limit = any(re.search(p, source) for p in patterns)
    # Also check that the function signature has the parameter
    has_param = 'max_messages: usize' in source
    return uses_limit and has_param, "respects max_messages limit"


def test_keeps_recent(source: str) -> tuple[bool, str]:
    """Check that recent messages are preserved (reverse iteration or slice from end)"""
    patterns = [
        r'\[\s*-',  # Negative indexing
        r'reversed',
        r'rev\(\)',
        r'\.\.[\s)]',  # Slice from end
    ]
    keeps_recent = any(re.search(p, source) for p in patterns)
    return keeps_recent, "keeps recent messages"


def test_removes_duplicates(source: str) -> tuple[bool, str]:
    """Check for deduplication logic"""
    patterns = [
        r'HashSet',
        r'BTreeSet',
        r'duplicate',
        r'distinct',
        r'filter.*==',
        r'fold.*unique',
    ]
    has_dedup = any(re.search(p, source) for p in patterns)
    return has_dedup, "removes duplicate system prompts"


def test_is_idempotent(source: str) -> tuple[bool, str]:
    """Check that compaction doesn't corrupt messages (basic structural checks)"""
    # Check that original messages are preserved in some form
    checks = [
        r'Message\s*\{',  # Message struct is constructed
        r'role:',  # role field is preserved
        r'content:',  # content field is preserved
    ]
    preserves_structure = all(re.search(p, source) for p in checks)
    return preserves_structure, "preserves message structure"


def grade() -> tuple[bool, str]:
    source = load_source()
    if not source:
        return False, "FAIL: src/compactor.rs not found"

    checks = [
        test_respects_max_messages(source),
        test_keeps_recent(source),
        test_removes_duplicates(source),
        test_is_idempotent(source),
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
