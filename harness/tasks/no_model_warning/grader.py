#!/usr/bin/env python3
"""Grader for no_model_warning task.

Verifies that when no model is configured (current_model is None),
the status bar displays a visible warning to prevent user confusion.
"""
import sys
from pathlib import Path

def find_file(name, subdirs=None):
    """Find a file in various potential locations."""
    candidates = [
        Path(f"crates/runie-tui/src/{subdir or ''}{name}") if subdir else Path(f"crates/runie-tui/src/{name}")
        for subdir in [subdirs] if subdirs
    ]
    candidates.extend([
        Path(name),
        Path("crates/runie-tui/src/tui/render.rs"),
        Path("crates/runie-tui/src/tui/view_models.rs"),
        Path("crates/runie-tui/src/tui/state.rs"),
        Path("crates/runie-tui/src/tui/update/misc.rs"),
    ])

    for candidate in candidates:
        if candidate.exists():
            return candidate
    return None


def check_status_bar_warning():
    checks = {
        "has_no_model_warning": False,
        "warning_uses_warning_color": False,
        "warning_is_visible": False,
        "submit_blocked_or_warning_shown": False,
    }

    # Check render.rs for no-model handling
    render_file = Path(__file__).parent.parent.parent / "crates/runie-tui/src/tui/render.rs"
    if not render_file.exists():
        render_file = Path("crates/runie-tui/src/tui/render.rs")

    if render_file.exists():
        content = render_file.read_text()

        # Check for warning text when no model
        warning_patterns = [
            "No model",
            "no model",
            "configure",
            "warning",
        ]
        if any(p.lower() in content.lower() for p in warning_patterns):
            # Check it's related to current_model
            if "current_model" in content:
                checks["has_no_model_warning"] = True

        # Check for warning color usage
        if "warning" in content.lower() and ("fg(" in content or "Color::" in content):
            checks["warning_uses_warning_color"] = True

        # Check that warning is conditionally shown for None
        if "is_none()" in content and "current_model" in content:
            checks["warning_is_visible"] = True

    # Check state.rs or view_models for current_model definition
    for f in [Path("crates/runie-tui/src/tui/state.rs"),
              Path("crates/runie-tui/src/tui/view_models.rs")]:
        if f.exists():
            content = f.read_text()
            if "current_model" in content:
                checks["submit_blocked_or_warning_shown"] = True
                break

    # Check update/misc.rs for submit handling with no model
    misc_file = Path(__file__).parent.parent.parent / "crates/runie-tui/src/tui/update/misc.rs"
    if misc_file.exists():
        content = misc_file.read_text()
        if "current_model" in content:
            checks["submit_blocked_or_warning_shown"] = True

    return checks


def main():
    print("Checking no-model warning implementation...")
    print()

    checks = check_status_bar_warning()

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
