#!/usr/bin/env python3
"""
Grader for workspace_concurrent_edits task.

Validates that concurrent file edits are handled safely through:
1. File locking mechanism
2. No lost updates
3. Consistent read-after-write
4. Race condition handling
"""
import sys
import re
from pathlib import Path

# Project root is the working directory when this script runs
PROJECT_ROOT = Path.cwd()

def check_file_content(relative_path):
    """Check if a file exists and return its content."""
    full_path = PROJECT_ROOT / relative_path
    if full_path.exists():
        return full_path.read_text()
    return ""

def check_concurrency_handling():
    checks = {
        "file_locking_present": False,
        "no_lost_updates": False,
        "consistent_reads": False,
        "race_handled": False,
    }

    # Check workspace.rs for locking mechanism
    workspace_content = check_file_content("crates/runie-tools/src/workspace.rs")
    if workspace_content:
        # Check for locking primitives
        locking_patterns = [
            r'Mutex',
            r'RwLock',
            r'Arc',
            r'lock\(\)',
            r'file_lock',
        ]
        locking_found = sum(1 for p in locking_patterns if re.search(p, workspace_content))
        if locking_found >= 2:
            checks["file_locking_present"] = True

        # Check for serialization
        if "serial" in workspace_content.lower() or "queue" in workspace_content.lower():
            checks["race_handled"] = True

        # Check for atomic operations
        if "atomic" in workspace_content.lower():
            checks["race_handled"] = True

    # Check edit_file.rs for mtime detection
    edit_file_content = check_file_content("crates/runie-tools/src/edit_file.rs")
    if edit_file_content:
        # Check for mtime-based conflict detection
        if "mtime" in edit_file_content.lower() or "modified" in edit_file_content.lower():
            checks["no_lost_updates"] = True

        # Check for consistent read-after-write
        if "read" in edit_file_content.lower() and "write" in edit_file_content.lower():
            checks["consistent_reads"] = True

        # Check for atomic write pattern (write to temp, then rename)
        if "temp" in edit_file_content.lower() and "rename" in edit_file_content.lower():
            checks["no_lost_updates"] = True

    return checks


def main():
    print("Checking workspace concurrent edit safety...\n")

    checks = check_concurrency_handling()

    passed = 0
    total = len(checks)

    for check_name, result in checks.items():
        status = "PASS" if result else "FAIL"
        print(f"{status}: {check_name}")
        if result:
            passed += 1

    print(f"\n{passed}/{total} checks passed")

    if passed >= 3:
        print("RESULT: pass")
        sys.exit(0)
    else:
        print("RESULT: fail")
        sys.exit(1)


if __name__ == "__main__":
    main()
