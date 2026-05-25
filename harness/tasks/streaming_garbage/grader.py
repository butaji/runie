#!/usr/bin/env python3
"""Grader for streaming_garbage task.

Verifies that streaming implementation handles invalid UTF-8 gracefully.
"""
import sys
from pathlib import Path

def check_stream_validation():
    checks = {
        "validates_utf8": False,
        "has_stream_error_type": False,
        "skips_invalid_chunks": False,
        "no_panic_on_garbage": False,
    }

    # Find the runie-ai crate
    base_paths = [
        Path(__file__).parent.parent.parent,
        Path("."),
    ]

    found_files = []
    for base in base_paths:
        ai_dir = base / "crates/runie-ai/src"
        if ai_dir.exists():
            for f in ai_dir.glob("*.rs"):
                found_files.append(f)

    # Also check the agent loop for streaming handling
    loop_file = None
    for base in base_paths:
        candidate = base / "crates/runie-agent/src/loop_engine.rs"
        if candidate.exists():
            loop_file = candidate
            found_files.append(candidate)

    found_content = ""
    for f in found_files:
        content = f.read_text()
        # Look for streaming-related code
        if "stream" in content.lower() or "chunk" in content.lower():
            found_content += content + "\n"

    if found_content:
        # Check for UTF-8 validation
        utf8_patterns = [
            "from_utf8",
            "from_utf8_lossy",
            "is_utf8",
            "valid_utf8",
        ]
        if any(p in found_content for p in utf8_patterns):
            checks["validates_utf8"] = True

        # Check for error type
        error_patterns = [
            "StreamError",
            "StreamResult",
        ]
        if any(p in found_content for p in error_patterns):
            checks["has_stream_error_type"] = True

        # Check for skip or filter logic
        skip_patterns = [
            "skip",
            "filter",
            "filter_map",
        ]
        if any(p in found_content for p in skip_patterns):
            checks["skips_invalid_chunks"] = True

        # Check that from_utf8 is used with error handling (not unwrap)
        if "from_utf8" in found_content:
            # Good if it uses map_err or similar
            if "map_err" in found_content or "ok()" in found_content or "unwrap_or" in found_content:
                checks["no_panic_on_garbage"] = True
            # Also OK if using from_utf8_lossy
            if "from_utf8_lossy" in found_content:
                checks["no_panic_on_garbage"] = True

    return checks


def main():
    print("Checking streaming garbage handling implementation...")
    print()

    checks = check_stream_validation()

    passed = 0
    total = len(checks)

    for check_name, result in checks.items():
        status = "PASS" if result else "FAIL"
        print(f"{status}: {check_name}")
        if result:
            passed += 1

    print(f"\n{passed}/{total} checks passed")

    if passed >= 2:
        print("RESULT: pass")
        sys.exit(0)
    else:
        print("RESULT: fail")
        sys.exit(1)


if __name__ == "__main__":
    main()
