# Path Completion

**Status**: done
**Milestone**: R2
**Category**: Input & Commands

## Description

Tab-complete file and directory paths in the input field.

## Architecture

```rust
pub fn complete_path(partial: &str, cwd: &Path) -> Vec<PathCompletion> {
    let base = if partial.is_empty() { cwd } else { cwd.join(partial) };
    let parent = base.parent().unwrap_or(cwd);
    let prefix = base.file_name().map(|s| s.to_string_lossy()).unwrap_or_default();
    
    std::fs::read_dir(parent)
        .unwrap_or_default()
        .filter_map(|e| e.ok())
        .filter(|e| {
            let name = e.file_name().to_string_lossy();
            name.starts_with(&prefix) && (!name.starts_with('.') || prefix.starts_with('.'))
        })
        .map(|e| PathCompletion {
            path: e.path().to_string_lossy().to_string(),
            is_dir: e.file_type().map(|t| t.is_dir()).unwrap_or(false),
        })
        .collect()
}

pub struct PathCompletion {
    pub path: String,
    pub is_dir: bool,
}
```

## Acceptance Criteria

- [x] Tab triggers path completion popup
- [x] Shows files and directories matching prefix
- [x] Directories marked with `/`
- [x] Arrow keys navigate, Enter selects
- [x] Works after `@` and bare paths
- [x] Hidden files excluded unless prefix starts with `.`

## Tests

### Layer 1
- [x] `complete_empty_returns_cwd` — tab with empty shows cwd
- [x] `complete_filters_prefix` — "src/" filters children
- [x] `complete_excludes_hidden` — dotfiles hidden

### Layer 2
- [x] `tab_triggers_completion` — Tab key opens popup
