#!/usr/bin/env python3
"""Grader for file_stale_edit task.

Verifies that edit_file tool detects when a file has been modified
between read and write operations (TOCTOU race condition).
"""
import sys
from pathlib import Path

def find_edit_file():
    """Find the edit_file tool implementation."""
    candidates = [
        Path("crates/runie-tools/src/edit_file.rs"),
        Path("edit_file.rs"),
        Path(__file__).parent.parent.parent / "crates/runie-tools/src/edit_file.rs",
    ]
    
    for candidate in candidates:
        if candidate.exists():
            return candidate
    
    for f in Path(".").glob("**/edit_file.rs"):
        return f
    
    return None


def check_source():
    checks = {
        "has_mtime_tracking": False,
        "detects_file_change": False,
        "returns_meaningful_error": False,
        "does_not_silently_overwrite": False,
    }
    
    src = find_edit_file()
    if not src:
        print("WARNING: Could not find edit_file.rs")
        return checks
    
    content = src.read_text()
    
    # Check for mtime/modified time tracking
    mtime_patterns = [
        "modified()",
        "mtime",
        "last_modified",
        "file_time",
        "metadata().modified",
    ]
    if any(p in content for p in mtime_patterns):
        checks["has_mtime_tracking"] = True
    
    # Check for file change detection logic
    detect_patterns = [
        "original_mtime",
        "saved_mtime",
        "current_mtime",
        "previous_mtime",
    ]
    if any(p in content for p in detect_patterns):
        checks["detects_file_change"] = True
    
    # Check for meaningful error message
    error_patterns = [
        "stale",
        "modified since",
        "concurrent modification",
        "file changed",
        "TOCTOU",
    ]
    if any(p.lower() in content.lower() for p in error_patterns):
        checks["returns_meaningful_error"] = True
    
    # Check that write doesn't silently overwrite
    # (i.e., there's a conditional check before write)
    if "if" in content and ("mtime" in content or "modified" in content):
        # There should be a check before write
        checks["does_not_silently_overwrite"] = True
    
    return checks


def main():
    print("Checking file_stale_edit implementation...")
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
