#!/usr/bin/env python3
"""Lint script enforcing code quality rules."""
import os
import sys
import re
from pathlib import Path

MAX_FILE_LINES = 500
MAX_FUNC_LINES = 40
MAX_COMPLEXITY = 10

def count_code_lines(content: str) -> int:
    """Count non-blank, non-comment lines."""
    lines = content.split('\n')
    count = 0
    for line in lines:
        stripped = line.strip()
        if stripped and not stripped.startswith('//') and not stripped.startswith('#'):
            count += 1
    return count

def check_function_complexity(func_body: list) -> tuple[int, int]:
    """Returns (lines, complexity) for a function."""
    lines = len(func_body)
    complexity = 1
    
    body_text = '\n'.join(func_body)
    
    # Count complexity-increasing patterns
    complexity += len(re.findall(r'\bif\b', body_text))
    complexity += len(re.findall(r'\bwhile\b', body_text))
    complexity += len(re.findall(r'\bfor\b', body_text))
    complexity += len(re.findall(r'\bmatch\b', body_text))
    complexity += len(re.findall(r'\belse\b', body_text))
    complexity += body_text.count(' && ')
    complexity += body_text.count(' || ')
    complexity += body_text.count('?')  # try operator
    
    return lines, complexity

def check_file(path: Path) -> list[str]:
    """Check a single file for lint errors."""
    errors = []
    
    try:
        content = path.read_text()
    except:
        return errors
    
    # Check file line count
    code_lines = count_code_lines(content)
    if code_lines > MAX_FILE_LINES:
        errors.append(f"{path}: {code_lines} lines (max {MAX_FILE_LINES})")
    
    # Check functions
    lines = content.split('\n')
    in_func = False
    func_start = 0
    func_body = []
    brace_depth = 0
    
    for i, line in enumerate(lines, 1):
        stripped = line.strip()
        
        # Detect function start
        if not in_func and re.match(r'^(pub\s+)?(async\s+)?fn\s+\w+', stripped):
            in_func = True
            func_start = i
            func_body = [line]
            brace_depth = stripped.count('{') - stripped.count('}')
            continue
        
        if in_func:
            func_body.append(line)
            brace_depth += stripped.count('{') - stripped.count('}')
            
            if brace_depth <= 0 and '}' in stripped:
                # Function ended
                func_lines, complexity = check_function_complexity(func_body)
                
                if func_lines > MAX_FUNC_LINES:
                    errors.append(f"{path}:{func_start}: Function has {func_lines} lines (max {MAX_FUNC_LINES})")
                
                if complexity > MAX_COMPLEXITY:
                    errors.append(f"{path}:{func_start}: Complexity {complexity} (max {MAX_COMPLEXITY})")
                
                in_func = False
                func_body = []
    
    return errors

def main():
    errors = []
    
    # Check all crates
    for crate_dir in Path('crates').iterdir():
        if not crate_dir.is_dir():
            continue
        
        src_dir = crate_dir / 'src'
        if not src_dir.exists():
            continue
        
        print(f"Checking {crate_dir.name}...")
        
        for src_file in src_dir.rglob('*.rs'):
            # Skip generated files
            if 'target' in str(src_file):
                continue
            
            file_errors = check_file(src_file)
            errors.extend(file_errors)
            
            for err in file_errors:
                print(f"  ERROR: {err}")
    
    if errors:
        print(f"\n{len(errors)} lint error(s) found")
        sys.exit(1)
    
    print("Lint passed!")

if __name__ == '__main__':
    main()
