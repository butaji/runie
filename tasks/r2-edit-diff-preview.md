# Edit Diff Preview

**Status**: done
**Milestone**: R2
**Category**: Tools

## Description

Show a diff preview before applying an edit. The agent can review the proposed change before it is written to disk.

## Architecture

```rust
pub struct EditPreview {
    pub path: PathBuf,
    pub original: String,
    pub proposed: String,
    pub diff: String,
}

// In tool execution flow
pub fn preview_edit(path: &Path, old: &str, new: &str) -> Result<EditPreview> {
    let original = std::fs::read_to_string(path)?;
    let proposed = original.replace(old, new);
    let diff = generate_diff(&original, &proposed);
    Ok(EditPreview { path: path.to_owned(), original, proposed, diff })
}
```

The preview is shown in the chat as a collapsible diff block. The user can approve or reject.

## Acceptance Criteria

- [x] `edit_file` tool generates preview before applying
- [x] Preview shows unified diff with line numbers
- [x] User can approve (apply) or reject (cancel)
- [x] Preview rendered with green/red highlighting
- [x] Works with the file mutation queue

## Tests

### Layer 1
- [x] `preview_generates_diff` — diff output is valid unified diff
- [x] `preview_shows_line_numbers` — hunks have line numbers
