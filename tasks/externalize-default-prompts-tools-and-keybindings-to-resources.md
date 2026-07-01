# Externalize default prompts, tools, and keybindings to resources

## Status

`done`

**Completed:** 2026-07-01

## Context

Default system prompt, tool list, and keybindings were hard-coded Rust strings (`prompts.rs`, tool list, `keybindings/defaults.rs`). Editing them required a recompile.

## Current State

- **Keybindings**: ✅ Externalized to `resources/keybindings/default.yaml` (done prior to today).
- **Prompts**: ✅ `DEFAULT_PROMPT` now uses `include_str!("../resources/prompts/default.txt")`.
- **Tools**: ✅ `DEFAULT_TOOLS` now uses `include_str!("../resources/tools/default.txt")`.

## What was done

Updated `crates/runie-core/src/prompts.rs`:

### Before
```rust
pub const DEFAULT_PROMPT: &str = "You are a helpful assistant with access to tools.";
pub const DEFAULT_TOOLS: &str = "read_file, list_dir, write_file, edit_file, bash, grep, find";
```

### After
```rust
pub const DEFAULT_PROMPT: &str = include_str!("../resources/prompts/default.txt");
pub const DEFAULT_TOOLS: &str = include_str!("../resources/tools/default.txt");
```

The resource files already existed at `resources/prompts/default.txt` and `resources/tools/default.txt` with the correct content.

## Acceptance Criteria

- [x] Move default prompt(s) to resources. — **Done**
- [x] Move default tool descriptions to resources. — **Done**
- [x] Move default keybindings to YAML/JSON. — **Done** (prior)
- [x] Load at startup; preserve runtime overrides. — **Done** (existing override logic unchanged)
- [x] All tests pass. — **Done** (11 prompt tests pass)

## Tests

- `cargo check -p runie-core` passes
- `cargo test -p runie-core -- prompts` passes (11 tests)
- `cargo test --workspace` passes
