#!/usr/bin/env python3
"""Grader for network_retry task.

Verifies that tools implement retry logic for transient network errors.
"""
import sys
from pathlib import Path

def find_retry_implementation():
    """Find retry-related code in tools."""
    candidates = [
        Path("crates/runie-tools/src/retry.rs"),
        Path("crates/runie-tools/src/bash.rs"),
        Path("crates/runie-tools/src/lib.rs"),
        Path(__file__).parent.parent.parent / "crates/runie-tools/src/retry.rs",
    ]
    
    for candidate in candidates:
        if candidate.exists():
            return candidate
    
    for f in Path(".").glob("**/tools/**/*.rs"):
        return f
    
    return None


def check_source():
    checks = {
        "has_retry_wrapper": False,
        "has_exponential_backoff": False,
        "detects_transient_errors": False,
        "has_max_retries": False,
    }
    
    src = find_retry_implementation()
    if not src:
        print("WARNING: Could not find retry implementation")
        return checks
    
    content = src.read_text()
    
    # Check for retry wrapper
    retry_patterns = [
        "retry_with_backoff",
        "fn retry",
        "pub async fn retry",
    ]
    if any(p in content for p in retry_patterns):
        checks["has_retry_wrapper"] = True
    
    # Check for exponential backoff
    backoff_patterns = [
        "exponential",
        "backoff",
        "saturating_pow",
        "saturating_mul",
    ]
    if any(p in content for p in backoff_patterns):
        checks["has_exponential_backoff"] = True
    
    # Check for transient error detection
    transient_patterns = [
        "is_transient",
        "transient_error",
        "transient_patterns",
        "connection refused",
    ]
    if any(p in content.lower() for p in transient_patterns):
        checks["detects_transient_errors"] = True
    
    # Check for max retries
    max_retry_patterns = [
        "max_retries",
        "max_attempts",
        "attempt >=",
        "attempt >=",
    ]
    if any(p in content for p in max_retry_patterns):
        checks["has_max_retries"] = True
    
    return checks


def main():
    print("Checking network retry implementation...")
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
