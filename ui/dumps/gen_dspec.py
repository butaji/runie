#!/usr/bin/env python3
"""
Dspec generator: reads a grok ref dump and produces a dspec JSON.

Usage: python3 gen_dspec.py <ref.txt> [output.dspec.json]

Approach:
- Parse the ref into lines
- Detect the screen type (chat, welcome, session_list, etc.)
- Extract structural elements (top bar, user msg, assistant content, input box, shortcuts)
- Emit a dspec JSON that matches
"""
import sys
import json
import re
import os

GLYPH = '\ue0a0'

def strip_ansi(s):
    """Remove ANSI escape codes."""
    return re.sub(r'\x1b\[[0-9;]*m', '', s)

def width(s):
    """Display width of string (count chars, not bytes)."""
    return len(s)

def parse_ref(path):
    with open(path) as f:
        raw = f.read()
    # Strip ANSI codes
    raw = strip_ansi(raw)
    lines = raw.splitlines()
    return lines

def detect_pattern(lines, filename):
    non_blank = [l for l in lines if l.strip()]
    if not non_blank:
        return "empty"
    if "┌─" in non_blank[0] or "┌" in non_blank[0]:
        return "session_list"
    if "Commands" in raw if 'raw' in dir() else "\n".join(lines):
        return "command_palette"
    if "Thinking" in "\n".join(lines) or "plan" in filename.lower():
        return "plan_modal"
    if "extension" in filename.lower():
        return "extensions_modal"
    if "New worktree" in "\n".join(lines):
        return "welcome"
    return "chat"

def gen_dspec_chat(lines, width=80, timestamp="9:45 PM"):
    """Generate dspec for a chat screen."""
    non_blank_idx = [i for i, l in enumerate(lines) if l.strip()]
    if not non_blank_idx:
        return None

    children = []
    last_y = -1
    for i, line in enumerate(lines):
        if not line.strip():
            children.append({"type": "blank"})
            continue

        # Top bar: starts with "  " and has branch glyph
        if line.startswith("  ") and GLYPH in line and "feat" in line:
            # Extract content after the branch glyph
            parts = line.split(GLYPH, 1)
            if len(parts) == 2:
                after = parts[1].strip()
                # after is like "feat/grok-redesign ~/Code/GitHub/runie"
                # Split into branch and path
                tokens = after.split(" ", 1)
                branch = tokens[0] if tokens else "feat/grok-redesign"
                path = tokens[1] if len(tokens) > 1 else "~/Code/GitHub/runie"
                # Check for chip at the end
                chip = ""
                if "│" in line and line.count("│") >= 2:
                    # Extract chip between the last two │
                    parts2 = line.rsplit("│", 2)
                    if len(parts2) >= 3:
                        chip = "│" + parts2[1] + "│"
                if chip:
                    children.append({
                        "type": "row", "fill_trailing": True,
                        "children": [
                            {"type": "text", "content": f"  {GLYPH} {branch}", "style": "accent"},
                            {"type": "text", "content": " "},
                            {"type": "text", "content": path, "style": "secondary"},
                            {"type": "fill"},
                            {"type": "text", "content": chip, "style": "dim"}
                        ]
                    })
                else:
                    children.append({
                        "type": "row",
                        "children": [
                            {"type": "text", "content": f"  {GLYPH} {branch}", "style": "accent"},
                            {"type": "text", "content": " "},
                            {"type": "text", "content": path, "style": "secondary"}
                        ]
                    })
                continue

        # User msg: starts with "     ❯"
        if line.startswith("     ❯") or line.startswith("      ❯"):
            # Extract text after ❯
            text = line[6:].lstrip() if line.startswith("     ❯") else line[7:].lstrip()
            # Remove timestamp at the end
            ts = timestamp
            ts_match = re.search(r'\d+:\d+ [AP]M', text)
            if ts_match:
                ts = ts_match.group()
                text = text[:ts_match.start()].rstrip()
            if text:
                children.append({
                    "type": "pad", "top": 0, "right": 2, "bottom": 0, "left": 0,
                    "child": {
                        "type": "row", "fill_trailing": True,
                        "children": [
                            {"type": "text", "content": "     ❯", "style": "secondary"},
                            {"type": "text", "content": " "},
                            {"type": "text", "content": text, "style": "primary"},
                            {"type": "fill"},
                            {"type": "text", "content": ts, "style": "dim"}
                        ]
                    }
                })
            continue

        # Assistant bullet: starts with "     •"
        if line.startswith("     •"):
            text = line[5:].lstrip()
            children.append({
                "type": "pad", "top": 0, "right": 2, "bottom": 0, "left": 0,
                "child": {
                    "type": "row",
                    "children": [
                        {"type": "text", "content": f"     • {text}", "style": "primary"}
                    ]
                }
            })
            continue

        # Thought: "     ◆ Thought for X.Xs"
        if line.startswith("     ◆"):
            children.append({
                "type": "pad", "top": 0, "right": 2, "bottom": 0, "left": 0,
                "child": {
                    "type": "row",
                    "children": [
                        {"type": "text", "content": line.strip(), "style": "muted"}
                    ]
                }
            })
            continue

        # Input box border
        if line.startswith("  ╭") or line.startswith("  ╰"):
            children.append({
                "type": "row",
                "children": [
                    {"type": "text", "content": line}
                ]
            })
            continue

        # Input box content: "  │ ... │"
        if line.startswith("  │") and line.rstrip().endswith("│"):
            children.append({
                "type": "row",
                "children": [
                    {"type": "text", "content": line}
                ]
            })
            continue

        # Shortcuts
        if "Enter:send" in line or "Shift+Tab" in line or "Esc:back" in line:
            children.append({
                "type": "row",
                "children": [
                    {"type": "text", "content": line, "style": "dim"}
                ]
            })
            continue

        # Separator
        if line.strip().startswith("─"):
            children.append({
                "type": "row",
                "children": [
                    {"type": "text", "content": line}
                ]
            })
            continue

        # Default: text
        children.append({"type": "text", "content": line})

    return {
        "width": width,
        "height": 24,
        "mock_timestamp": timestamp,
        "root": {"type": "col", "children": children}
    }

def main():
    if len(sys.argv) < 2:
        print("Usage: gen_dspec.py <ref.txt> [output.dspec.json]")
        sys.exit(1)
    ref_path = sys.argv[1]
    out_path = sys.argv[2] if len(sys.argv) > 2 else ref_path.replace("grok", "scenarios").replace(".txt", ".dspec.json")

    lines = parse_ref(ref_path)
    pattern = detect_pattern(lines, os.path.basename(ref_path))

    # Detect width from the widest non-blank line
    w = max((width(l) for l in lines if l.strip()), default=78)

    if pattern == "chat":
        # Detect timestamp from user msg
        ts = "9:45 PM"
        for l in lines:
            m = re.search(r'(\d+:\d+ [AP]M)', l)
            if m:
                ts = m.group(1)
                break
        spec = gen_dspec_chat(lines, width=w, timestamp=ts)
    else:
        # Fallback: just emit each line as text
        children = []
        for l in lines:
            if not l.strip():
                children.append({"type": "blank"})
            else:
                children.append({"type": "text", "content": l})
        spec = {
            "width": w,
            "height": 24,
            "root": {"type": "col", "children": children}
        }

    os.makedirs(os.path.dirname(out_path), exist_ok=True)
    with open(out_path, "w") as f:
        json.dump(spec, f, indent=2, ensure_ascii=False)
    print(f"Generated {out_path} (pattern={pattern}, width={w})")

if __name__ == "__main__":
    main()
