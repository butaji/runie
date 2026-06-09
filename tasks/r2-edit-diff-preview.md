# Edit Diff Preview

**Status**: todo
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

- [ ] `edit_file` tool generates preview before applying
- [ ] Preview shows unified diff with line numbers
- [ ] User can approve (apply) or reject (cancel)
- [ ] Preview rendered with green/red highlighting
- [ ] Works with the file mutation queue

## Tests

### Layer 1
- [ ] `preview_generates_diff` — diff output is valid unified diff
- [ ] `preview_shows_line_numbers` — hunks have line numbers
