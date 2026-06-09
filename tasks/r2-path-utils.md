# Path Utils / CWD Resolution

**Status**: todo
**Milestone**: R2
**Category**: Tools

## Description

Full working directory resolution for all tool paths. Relative paths are resolved against the current working directory, not the binary's location.

## Architecture

```rust
pub fn resolve_path(raw: &str) -> PathBuf {
    let path = Path::new(raw);
    if path.is_absolute() {
        path.to_path_buf()
    } else {
        std::env::current_dir().unwrap_or_default().join(path)
    }
}
```

## Acceptance Criteria

- [ ] All tools resolve relative paths against cwd
- [ ] Absolute paths passed through unchanged
- [ ] `~` expanded to home directory
- [ ] Path canonicalization removes `..` and `.`

## Tests

### Layer 1
- [ ] `resolve_relative_joins_cwd` — "src/main.rs" → cwd/src/main.rs
- [ ] `resolve_absolute_unchanged` — "/tmp/foo" unchanged
- [ ] `resolve_tilde_expands` — "~/.runie" → /home/user/.runie
