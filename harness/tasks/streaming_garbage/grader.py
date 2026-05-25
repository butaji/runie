#!/usr/bin/env python3
"""Grader for streaming_garbage task.

Verifies that streaming implementation handles invalid UTF-8 gracefully.
"""
import sys
from pathlib import Path

def check_stream_validation():
    checks = {
        "validates_utf8": False,
        "has_error_handling": False,
        "skips_invalid_chunks": False,
        "no_panic_on_garbage": False,
    }

    # Find streaming-related code in runie-agent
    agent_dir = Path("crates/runie-agent/src")
    if agent_dir.exists():
        for f in agent_dir.glob("*.rs"):
            content = f.read_text()
            if "stream" in content.lower() or "chunk" in content.lower():
                # Check for UTF-8 validation
                utf8_patterns = ["from_utf8", "from_utf8_lossy", "is_utf8", "valid_utf8"]
                if any(p in content for p in utf8_patterns):
                    checks["validates_utf8"] = True

                # Check for error handling
                error_patterns = ["map_err", "ok()", "unwrap_or", "Result", "Error"]
                if any(p in content for p in error_patterns):
                    checks["has_error_handling"] = True

                # Check for skip/filter logic
                skip_patterns = ["skip", "filter", "filter_map"]
                if any(p in content for p in skip_patterns):
                    checks["skips_invalid_chunks"] = True

    # Check runie-ai for streaming
    ai_dir = Path("crates/runie-ai/src")
    if ai_dir.exists():
        for f in ai_dir.glob("*.rs"):
            content = f.read_text()
            if "stream" in content.lower():
                if "from_utf8" in content or "from_utf8_lossy" in content:
                    checks["validates_utf8"] = True
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
