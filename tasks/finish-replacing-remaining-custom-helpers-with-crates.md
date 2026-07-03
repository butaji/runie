# Finish replacing remaining custom helpers with crates

## Status

`partial` — Most helpers already replaced; remaining modules are thin wrappers around crates.

## Context

Custom helpers that can be replaced with standard crates have been removed. The remaining modules are thin wrappers.

## What was replaced (done)

- `glob.rs` — deleted; `globset`/`ignore` used directly
- `fuzzy.rs` — deleted; `sublime_fuzzy` used directly
- `keybinding.rs`/`parse_key_combo` — moved to `#[cfg(test)]`

## What remains

### `display_width.rs` (45 lines)

Thin wrapper around `unicode_width`:
```rust
pub fn width(s: &str) -> u16 { unicode_width::UnicodeWidthStr::width(s) }
pub fn split_at_width(s: &str, max_width: u16) -> (&str, &str) { ... }
```

Used in: `layout.rs`, `tool/format.rs`, `popups/panel/form.rs`.

**Replacement:** Delete `display_width.rs`; callers use `unicode_width::UnicodeWidthStr::width` directly.

### `path.rs` (74 lines)

Thin wrapper around `shellexpand` + `path-absolutize`:
```rust
pub fn resolve_path_in(raw: &str, working_dir: impl AsRef<Path>) -> PathBuf {
    let expanded = shellexpand::tilde(raw).into_owned();
    let path = Path::new(&expanded);
    if path.is_absolute() { path.absolutize().unwrap() }
    else { working_dir.join(path).absolutize().unwrap() }
}
```

Used in: `runie-agent` tool modules (7 import sites).

**Replacement:** Delete `path.rs`; callers use `shellexpand` and `path_absolutize::Absolutize` directly.

## Remaining work

1. Delete `crates/runie-core/src/display_width.rs`
2. Replace all `crate::display_width::width` with `unicode_width::UnicodeWidthStr::width`
3. Replace all `crate::display_width::split_at_width` with direct implementation or `unicode_width`
4. Delete `crates/runie-core/src/path.rs`
5. Replace all `runie_core::path::resolve_path_in` with direct `shellexpand::tilde` + `path_absolutize`
6. Verify E2E tests pass

## Acceptance criteria

- [ ] Delete `display_width.rs` and update callers.
- [ ] Delete `path.rs` and update callers.
- [ ] Unit tests pass for affected modules.
- [ ] E2E tests pass.
- [ ] Live tmux verification of palette, file picker, and slash commands.

## Tests

- **Layer 1 — State/Logic:** N/A.
- **Layer 2 — Event Handling:** N/A.
- **Layer 3 — Rendering:** N/A.
- **Layer 4 — E2E:** Replay tests for path/glob/fuzzy/keybinding commands.
- **Live tmux tests:** Open command palette, file picker, and submit a slash command.
