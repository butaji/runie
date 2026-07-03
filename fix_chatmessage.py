#!/usr/bin/env python3
"""Fix ChatMessage migration: content field -> parts, .content -> .content()"""

import re
import sys

def process_file_content(content: str) -> str:
    """Transform content: patterns in ChatMessage struct literals and fix field accesses."""

    # Step 1: Only transform content: inside ChatMessage struct literals.
    # A ChatMessage struct has a role: field. Event::Response has content: but no role:.
    # Strategy: scan for ChatMessage struct openings, find matching closing brace,
    # transform content: within that range.

    lines = content.split('\n')
    result_lines = []
    i = 0

    while i < len(lines):
        line = lines[i]

        # Check if this line starts a ChatMessage struct literal
        # Pattern: push(ChatMessage { or new(ChatMessage { or similar
        chat_msg_start = re.search(r'\bChatMessage\s*\{', line)

        if chat_msg_start:
            # Found a ChatMessage struct. Process lines until matching }
            # Collect lines until matching brace
            struct_lines = [line]
            brace_count = line.count('{') - line.count('}')
            j = i + 1
            while j < len(lines) and brace_count > 0:
                struct_lines.append(lines[j])
                brace_count += lines[j].count('{') - lines[j].count('}')
                j += 1

            # Process this struct's lines
            processed_struct = transform_struct_lines(struct_lines)
            result_lines.extend(processed_struct)
            i = j
        else:
            result_lines.append(line)
            i += 1

    content = '\n'.join(result_lines)

    # Step 2: Fix .content field access (not in struct literals - those are done)
    content = fix_field_access(content)

    # Step 3: Add Part import where needed
    content = add_part_import(content)

    return content


def transform_struct_lines(lines: list) -> list:
    """Transform content: patterns in a list of struct literal lines."""

    # Patterns to apply (in order of specificity)
    transformations = []

    for idx, line in enumerate(lines):
        orig = line

        # Skip if already transformed
        if 'parts: vec![Part::Text' in line:
            continue

        # Pattern 1: content: "string".to_string()  →  parts: vec![Part::Text { content: "string".into() }]
        m = re.match(r'^(\s*)(content:)\s*("(?:[^"\\]|\\.)*")\.to_string\(\)(.*)$', line)
        if m:
            indent = m.group(1)
            string_lit = m.group(3)
            rest = m.group(4)  # trailing content after .to_string()
            line = indent + 'parts: vec![Part::Text { content: ' + string_lit + '.into() }]' + rest
            lines[idx] = line
            continue

        # Pattern 2: content: "string".into()  →  parts: vec![Part::Text { content: "string".into() }]
        m = re.match(r'^(\s*)(content:)\s*("(?:[^"\\]|\\.)*")\.into\(\)(.*)$', line)
        if m:
            indent = m.group(1)
            string_lit = m.group(3)
            rest = m.group(4)
            line = indent + 'parts: vec![Part::Text { content: ' + string_lit + '.into() }]' + rest
            lines[idx] = line
            continue

        # Pattern 3: content: var.into()  →  parts: vec![Part::Text { content: var }]
        m = re.match(r'^(\s*)(content:)\s*([a-zA-Z_][a-zA-Z0-9_]*)\.into\(\)(.*)$', line)
        if m:
            indent = m.group(1)
            var = m.group(3)
            rest = m.group(4)  # might start with ',' or '}' or whitespace
            line = indent + 'parts: vec![Part::Text { content: ' + var + ' }]' + rest
            lines[idx] = line
            continue

        # Pattern 4: content: format!(...)  →  parts: vec![Part::Text { content: format!(...) }]
        # This matches the entire format!(...) expression until , or }
        m = re.match(r'^(\s*)(content:)\s*(format_\S.*?)(,?\s*)$', line)
        if m:
            indent = m.group(1)
            fmt_call = m.group(3)
            rest = m.group(4)
            line = indent + 'parts: vec![Part::Text { content: ' + fmt_call + ' }]' + rest
            lines[idx] = line
            continue

        # Pattern 5: content: bare_identifier  (no .into(), just a bare variable)
        # Match: content: identifier, or content: identifier, or content: identifier}
        # The identifier must be at the end of the line (possibly followed by , or } and whitespace)
        m = re.match(r'^(\s*)(content:)\s*([a-zA-Z_][a-zA-Z0-9_]*)(\s*[,\}]?\s*)$', line)
        if m:
            indent = m.group(1)
            var = m.group(3)
            after = m.group(4).strip()  # might be ',' or '}' or ''
            # The replacement is: parts: vec![Part::Text { content: var }]
            # If original ended with ',' → replace with '],' (add ] before ,)
            # If original ended with '}' → replace with '}],' (preserve } then add ],)
            if after == ',':
                line = indent + 'parts: vec![Part::Text { content: ' + var + ' }],'
            elif after == '}':
                line = indent + 'parts: vec![Part::Text { content: ' + var + ' }],'
            else:
                line = indent + 'parts: vec![Part::Text { content: ' + var + ' }],'
            lines[idx] = line
            continue

        # Pattern 6: content: "string" (bare string literal, no method)
        m = re.match(r'^(\s*)(content:)\s*("(?:[^"\\]|\\.)*")(\s*[,\}].*)$', line)
        if m:
            indent = m.group(1)
            string_lit = m.group(3)
            rest = m.group(4)  # includes , or } after optional whitespace
            line = indent + 'parts: vec![Part::Text { content: ' + string_lit + '.into() }]' + rest
            lines[idx] = line
            continue

        # Pattern 7: content: long_expr (complex expression, not just identifier)
        # Match: content: expr, where expr has spaces or parens
        m = re.match(r'^(\s*)(content:)\s*(.+)(\s*,?)$', line)
        if m:
            indent = m.group(1)
            expr = m.group(3).strip()
            comma = m.group(4)
            # Only transform if it looks like an expression (has parens, etc)
            if '(' in expr or '<' in expr:
                # Strip trailing } if present (struct on same line)
                if expr.endswith('}'):
                    expr = expr[:-1].strip()
                    trailing_brace = '}'
                else:
                    trailing_brace = ''
                line = indent + 'parts: vec![Part::Text { content: ' + expr + ' }]' + trailing_brace + '],' + comma
                lines[idx] = line
                continue

    return lines


def fix_field_access(content: str) -> str:
    """Replace .content field access with .content() method call"""

    # Fix: msg.content.as_str()  →  msg.content().as_str()
    content = re.sub(r'(\w+)\.content\.as_str\(\)', r'\1.content().as_str()', content)

    # Fix: msg.content.contains(...)  →  msg.content().contains(...)
    content = re.sub(r'(\w+)\.content\.contains\(', r'\1.content().contains(', content)

    # Fix: assert_eq!(msg.content, ...)  →  assert_eq!(msg.content(), ...)
    content = re.sub(r'(\w+)\.content,', r'\1.content(),', content)

    # Fix standalone .content (not followed by ( or _ or .) after word character
    content = re.sub(r'(?<=[\w])(\.content)(?!\(|_)', '.content()', content)

    return content


def add_part_import(content: str) -> str:
    """Add Part import if Part:: is used but not imported"""
    if 'Part::' not in content:
        return content
    if re.search(r'use\s+[^;]*\bPart\b', content):
        return content

    # Try to add to existing runie_core imports
    if re.search(r'use\s+runie_core::(?:model::)?\{[^}]*\bChatMessage\b', content):
        content = re.sub(
            r'(use\s+runie_core::(?:model::)?\{[^}]*)\bChatMessage\b',
            r'\1ChatMessage, Part',
            content
        )
    elif re.search(r'use\s+runie_core::(?:model::)?\{', content):
        content = re.sub(
            r'(use\s+runie_core::(?:model::)?\{)',
            r'\1Part, ',
            content
        )
    elif 'runie_core::model::ChatMessage' in content or 'runie_core::ChatMessage' in content:
        if 'runie_core::event::' in content:
            content = re.sub(r'(use\s+runie_core::event::[^\n]+\n)', r'use runie_core::Part;\n\1', content)
        elif 'use runie_core::model' in content:
            content = re.sub(r'(use\s+runie_core::model::[^\n]+\n)', r'use runie_core::Part;\n\1', content)
        elif 'use runie_core::' in content:
            content = re.sub(r'(use\s+runie_core::[^\n]+\n)', r'use runie_core::Part;\n\1', content)

    return content


def process_file(filepath: str) -> bool:
    with open(filepath, 'r') as f:
        content = f.read()

    original = content
    content = process_file_content(content)

    if content != original:
        with open(filepath, 'w') as f:
            f.write(content)
        print(f"Fixed: {filepath}")
        return True
    return False


if __name__ == '__main__':
    for filepath in sys.argv[1:]:
        process_file(filepath)
