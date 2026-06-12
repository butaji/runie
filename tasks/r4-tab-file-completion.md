# Task: Tab-Based File/Folder Completion in Input Box

## Context
Users need shell-like tab completion for file and folder paths directly in the input box. When typing a partial path like `@Carg` or `./src/ma` and pressing Tab, the input should autocomplete to the first match (or cycle through matches on subsequent tabs).

## Design

### Trigger
Tab key in the input box when:
1. Input contains `@` followed by partial text → complete to matching file in cwd
2. Input contains `./` or `../` followed by partial text → complete relative path

### Behavior
- **Single match**: Complete to full path, cursor at end
- **Multiple matches**: Complete to common prefix, subsequent Tab cycles through matches
- **No match**: Flash input (same as invalid key)

### Completion Algorithm
1. Detect the "token" being completed (text after `@`, `./`, `../`)
2. Find all files/folders in the relevant directory matching the prefix
3. Return sorted matches
4. Replace token with selected match

## DSL for Completion Sources

```rust
/// A completion source provides candidates for tab-completion.
pub trait CompletionSource {
    /// Returns candidates matching the given prefix.
    fn complete(&self, prefix: &str) -> Vec<String>;
    /// Human-readable label for this source.
    fn label(&self) -> &'static str;
}

/// File completion: completes relative file/folder paths.
pub struct FileCompletion { base: PathBuf }
impl CompletionSource for FileCompletion { ... }
```

This DSL will be used for:
- `@` refs → files in cwd
- `./` paths → relative file paths
- Future: `#` → commit hashes, `:` → line numbers, etc.

## Acceptance Criteria

### Layer 1: State/Logic
- [ ] `tab_complete()` finds files matching prefix after `@` or `./`
- [ ] `tab_complete()` cycles through multiple matches on repeated Tab
- [ ] `tab_complete()` flashes on no match

### Layer 2: Event Handling
- [ ] Tab with no token to complete → flash
- [ ] Tab with `@par` → completes to first matching file
- [ ] Second Tab with same prefix → cycles to next match
- [ ] Tab with `./src/ma` → completes relative path

### Layer 3: Rendering
- [ ] No special rendering needed (inline text replacement)

### Tests
- [ ] `tab_completes_file_after_at`
- [ ] `tab_cycles_multiple_file_matches`
- [ ] `tab_completes_relative_path`
- [ ] `tab_flashes_when_no_match`
- [ ] `tab_completes_to_common_prefix`

## Dependencies
- `r3-at-file-picker-fix` (disables old at_suggestions popup)
