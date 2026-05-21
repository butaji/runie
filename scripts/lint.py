#!/usr/bin/env python3
"""Code quality linter for tidy codebase.

Enforces:
- Max file lines: 500
- Max function lines: 40  
- Max function complexity: 10 (branches + loops + matches)
"""

import sys
import re
from pathlib import Path

MAX_FILE_LINES = 500
MAX_FUNCTION_LINES = 40
MAX_COMPLEXITY = 10

def count_function_lines(content: str, start_line: int, end_line: int) -> int:
    """Count non-blank, non-comment lines in a function."""
    lines = content.split('\n')[start_line:end_line]
    count = 0
    in_multiline_comment = False
    
    for line in lines:
        stripped = line.strip()
        
        # Skip blank lines
        if not stripped:
            continue
            
        # Handle multiline comments
        if '/*' in stripped and '*/' not in stripped:
            in_multiline_comment = True
            continue
        if in_multiline_comment:
            if '*/' in stripped:
                in_multiline_comment = False
            continue
            
        # Skip single-line comments
        if stripped.startswith('//') or stripped.startswith('*'):
            continue
            
        count += 1
    
    return count

def calculate_complexity(content: str, start_line: int, end_line: int) -> int:
    """Calculate cyclomatic complexity approximation.
    
    Counts: if, else, match, for, while, loop, ?, &&, ||, catch
    """
    lines = content.split('\n')[start_line:end_line]
    complexity = 1  # Base complexity
    
    for line in lines:
        stripped = line.strip()
        if not stripped or stripped.startswith('//') or stripped.startswith('*'):
            continue
            
        # Count branches
        complexity += len(re.findall(r'\bif\b|\belse\s*\{|\bmatch\b|\bfor\b|\bwhile\b|\bloop\b', stripped))
        # Count early returns (Result/Option handling)
        complexity += stripped.count('?')
        # Count logical operators
        complexity += stripped.count('&&') + stripped.count('||')
        # Count catch/unwrap_or
        complexity += len(re.findall(r'\bcatch\b|unwrap_or|unwrap_or_else', stripped))
    
    return complexity

def analyze_file(filepath: Path) -> list:
    """Analyze a Rust file for violations."""
    violations = []
    content = filepath.read_text()
    lines = content.split('\n')
    
    # Check file length
    if len(lines) > MAX_FILE_LINES:
        violations.append(
            f"  FILE TOO LONG: {len(lines)} lines (max {MAX_FILE_LINES})"
        )
    
    # Find functions and their line counts
    # Match: fn name(...) { or fn name(...) -> Type {
    # or async fn name(...) {
    func_pattern = re.compile(r'^(\s*)(pub\s+)?(async\s+)?fn\s+\w+.*\{')
    
    in_function = False
    func_start = 0
    brace_depth = 0
    func_name = ""
    
    for i, line in enumerate(lines):
        stripped = line.strip()
        
        # Check for function start
        if not in_function:
            match = func_pattern.match(stripped)
            if match and not stripped.startswith('//'):
                # Extract function name
                name_match = re.search(r'fn\s+(\w+)', stripped)
                if name_match:
                    func_name = name_match.group(1)
                    in_function = True
                    func_start = i
                    brace_depth = 1
            continue
        
        # Track braces to find function end
        if in_function:
            # Skip strings and comments for brace counting (simplified)
            code_part = stripped.split('//')[0]
            
            # Count braces
            for char in code_part:
                if char == '{':
                    brace_depth += 1
                elif char == '}':
                    brace_depth -= 1
                    if brace_depth == 0:
                        # Function ended
                        func_lines = count_function_lines(content, func_start, i + 1)
                        func_complexity = calculate_complexity(content, func_start, i + 1)
                        
                        if func_lines > MAX_FUNCTION_LINES:
                            violations.append(
                                f"  FUNCTION TOO LONG: `{func_name}` = {func_lines} lines "
                                f"(max {MAX_FUNCTION_LINES}) at line {func_start + 1}"
                            )
                        
                        if func_complexity > MAX_COMPLEXITY:
                            violations.append(
                                f"  COMPLEXITY TOO HIGH: `{func_name}` = {func_complexity} "
                                f"(max {MAX_COMPLEXITY}) at line {func_start + 1}"
                            )
                        
                        in_function = False
                        brace_depth = 0
                        break
    
    return violations

def main():
    """Run linter on all Rust files in the project."""
    project_root = Path(__file__).parent.parent
    src_dirs = [
        project_root / "crates" / "runie-tui" / "src",
        project_root / "crates" / "runie-cli" / "src",
        project_root / "crates" / "runie-agent" / "src",
        project_root / "crates" / "runie-ai" / "src",
        project_root / "crates" / "runie-core" / "src",
    ]
    
    all_violations = []
    files_checked = 0
    
    for src_dir in src_dirs:
        if not src_dir.exists():
            continue
        
        for rust_file in src_dir.rglob("*.rs"):
            if rust_file.name == "lib.rs" or rust_file.name == "main.rs":
                # Skip module root files (often long)
                continue
                
            violations = analyze_file(rust_file)
            files_checked += 1
            
            if violations:
                rel_path = rust_file.relative_to(project_root)
                all_violations.append(f"\n{rel_path}:")
                all_violations.extend(violations)
    
    print(f"Checked {files_checked} Rust files")
    
    if all_violations:
        print("\n" + "=" * 60)
        print("VIOLATIONS FOUND:")
        print("=" * 60)
        for v in all_violations:
            print(v)
        print(f"\nTotal violations: {len([v for v in all_violations if not v.startswith(chr(10))])}")
        return 1
    else:
        print("\n✓ All files pass quality checks!")
        return 0

if __name__ == "__main__":
    sys.exit(main())
