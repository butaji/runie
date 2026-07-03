#!/bin/bash
# Regenerates tasks/index.json from task markdown files
# Usage: ./regenerate-task-index.sh

set -e

TASKS_DIR="/Users/admin/Code/GitHub/runie-dev/tasks"
INDEX_FILE="$TASKS_DIR/index.json"

cd "$TASKS_DIR"

# Use Python for proper JSON escaping
python3 << 'PYTHON_SCRIPT'
import os
import re
import json

TASKS_DIR = "/Users/admin/Code/GitHub/runie-dev/tasks"
INDEX_FILE = os.path.join(TASKS_DIR, "index.json")

def extract_status(lines):
    """Extract status from task file lines."""
    # Join all lines for easier pattern matching
    content = "\n".join(lines)
    
    # Pattern 1: **Status**: value (bold inline, case 1)
    match = re.search(r'\*\*Status\*\*:\s*([^\n]+)', content)
    if match:
        status = match.group(1).strip().lower()
        status = re.sub(r'\s*\[\[.*\]\]\s*', '', status)
        status = re.sub(r'\s*—.*$', '', status)
        status = status.strip()
        if status in ("done", "partial", "todo", "wontfix", "blocked"):
            return status
    
    # Pattern 2: ## Status\n\n`done` (heading with code span)
    match = re.search(r'## Status\s*\n\s*`([^`]+)`', content)
    if match:
        status = match.group(1).strip().lower()
        if status in ("done", "partial", "todo", "wontfix", "blocked"):
            return status
    
    # Pattern 3: **Status**: value [[done]]
    match = re.search(r'\*\*Status\*\*:\s*([^\n]+)', content)
    if match:
        # Look for status in brackets
        full_match = match.group(1)
        bracket_match = re.search(r'\[\[([^\]]+)\]\]', full_match)
        if bracket_match:
            status = bracket_match.group(1).strip().lower()
            if status in ("done", "partial", "todo", "wontfix", "blocked"):
                return status
    
    # Pattern 4: "done" in Status section (after ## Status heading)
    status_section_match = re.search(r'## Status\s*\n+(.*?)(?=\n## |\Z)', content, re.DOTALL)
    if status_section_match:
        section = status_section_match.group(1).strip().lower()
        # Check for "done", "partial", "todo", etc. in the section
        for status in ["done", "partial", "todo", "wontfix", "blocked"]:
            if status in section:
                return status
    
    return "todo"

tasks = []
for f in sorted(os.listdir(TASKS_DIR)):
    if not f.endswith(".md"):
        continue
    if f in ("index.json", "TEMPLATE.md"):
        continue
    if ".archive" in f:
        continue
    
    filepath = os.path.join(TASKS_DIR, f)
    
    with open(filepath, "r") as fh:
        lines = fh.readlines()
    
    # Get title from first line
    title = lines[0].strip().lstrip("# ").strip() if lines else f
    
    # Extract status
    status = extract_status(lines)
    
    # Escape JSON special characters in title
    title = title.replace("\\", "\\\\").replace('"', '\\"').replace("\n", " ")
    
    tasks.append({
        "file": f,
        "title": title,
        "status": status
    })

with open(INDEX_FILE, "w") as fh:
    json.dump(tasks, fh, indent=2)

print(f"Regenerated {INDEX_FILE} with {len(tasks)} tasks")

# Print summary
from collections import Counter
status_counts = Counter(t["status"] for t in tasks)
for status, count in sorted(status_counts.items()):
    print(f"  {status}: {count}")

PYTHON_SCRIPT
