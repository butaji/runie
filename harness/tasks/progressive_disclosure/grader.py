#!/usr/bin/env python3
"""
Grader for progressive_disclosure task.

Validates that advanced options follow progressive disclosure:
1. Primary options are always visible
2. Advanced options are hidden by default
3. Advanced options can be revealed
4. Toggle mechanism exists
"""
import sys
import re
from pathlib import Path

PROJECT_ROOT = Path.cwd()

def check_file_content(relative_path):
    """Check if a file exists and return its content."""
    full_path = PROJECT_ROOT / relative_path
    if full_path.exists():
        return full_path.read_text()
    return ""

def check_progressive_disclosure():
    checks = {
        "primary_options_visible": False,
        "advanced_options_hidden_by_default": False,
        "advanced_revealed_on_interaction": False,
        "show_advanced_toggle_exists": False,
    }

    # Check permission modal for progressive disclosure
    modal_content = check_file_content("crates/runie-tui/src/components/permission_modal.rs")
    if modal_content:
        # Primary options (Confirm/Cancel) should be prominent
        if "Confirm" in modal_content and "Cancel" in modal_content:
            checks["primary_options_visible"] = True

        # Check for advanced options that are dimmed/hidden
        advanced_patterns = [
            r'AddModifier.*DIM',  # Dim modifier for secondary options
            r'DIM.*add_modifier',  # DIM style
            r'always.*allow.*hint',  # Hint text
            r'skip.*this.*hint',  # Skip hint
        ]
        advanced_found = sum(1 for p in advanced_patterns if re.search(p, modal_content, re.IGNORECASE))
        if advanced_found >= 1:
            checks["advanced_options_hidden_by_default"] = True

        # Check for keyboard shortcuts that reveal advanced
        reveal_patterns = [
            r'\[a\].*always',  # [a] for always allow
            r'\[s\].*skip',  # [s] for skip
            r'Char.*a.*Always',  # Key handler for 'a'
            r'Char.*s.*Skip',  # Key handler for 's'
        ]
        reveal_found = sum(1 for p in reveal_patterns if re.search(p, modal_content, re.IGNORECASE))
        if reveal_found >= 2:
            checks["advanced_revealed_on_interaction"] = True

    # Check state.rs for show_advanced toggle
    state_content = check_file_content("crates/runie-tui/src/tui/state.rs")
    if state_content:
        if "show_advanced" in state_content or "showAdvanced" in state_content:
            checks["show_advanced_toggle_exists"] = True

    # Check command palette for filter-based disclosure
    palette_content = check_file_content("crates/runie-tui/src/components/command_palette/mod.rs")
    if palette_content:
        # Fuzzy filter reveals options as you type
        if "filter" in palette_content.lower() and "fuzzy" in palette_content.lower():
            checks["advanced_revealed_on_interaction"] = True

    return checks

def main():
    print("Checking progressive disclosure patterns...\n")

    checks = check_progressive_disclosure()

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
