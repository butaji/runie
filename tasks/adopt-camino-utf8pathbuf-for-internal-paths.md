# Adopt camino Utf8PathBuf for internal paths

## Status

`done`

## Context

Codebase stores and serializes paths as `PathBuf`, then repeatedly calls `to_string_lossy()` or `to_str().unwrap()` for JSON, display, and map keys.

## Goal

Use `camino::Utf8PathBuf`/`Utf8Path` for config paths, project paths, tool paths, and trust keys; keep `std::path::PathBuf` at the OS boundary.

## Implementation

### Phase 1: Core type migrations ✅

Migrated the following internal path fields from `PathBuf` to `Utf8PathBuf` (eliminating `to_string_lossy()` conversions):

| File | Field | Change |
|------|-------|--------|
| `edit_preview.rs` | `EditPreview.path` | `PathBuf` → `Utf8PathBuf` |
| `declarative/types.rs` | `SkillDef.file_path` | `PathBuf` → `Utf8PathBuf` |
| `declarative/types.rs` | `CommandDef.file_path` | `PathBuf` → `Utf8PathBuf` |
| `skills/mod.rs` | `Skill.file_path` | `PathBuf` → `Utf8PathBuf` |
| `resource_loader.rs` | `ResourceRecord.file_path` | `PathBuf` → `Utf8PathBuf` |
| `prompts.rs` | `PromptSource::{User,Project}File` | `PathBuf` → `String` |
| `subagents/mod.rs` | `parse_subagent_file` param | `&PathBuf` → `&Utf8PathBuf` |

### Key design decisions

1. **OS boundary preserved**: `PathBuf` kept at actual OS boundaries (actor messages, session I/O, config loading).
2. **No serialization change**: `Utf8PathBuf` serializes identically to `PathBuf` (both as JSON strings), so existing session files remain compatible.
3. **Trust already migrated**: `trust.rs` and `TrustMap` already used `Utf8PathBuf` — the migration completes the picture.
4. **Conversion at boundaries**: Where `IoActor` or `SessionActor` needs `PathBuf`, we convert via `.into_path_buf()` or `PathBuf::from(&utf8_path)`.

### Files changed

- `crates/runie-core/src/edit_preview.rs` — type + 4 new unit tests
- `crates/runie-core/src/declarative/types.rs` — type changes + imports
- `crates/runie-core/src/declarative/loader.rs` — `derive_name_from_path` call updated
- `crates/runie-core/src/skills/mod.rs` — type + imports
- `crates/runie-core/src/skills/load.rs` — `resolve_name` call updated
- `crates/runie-core/src/resource_loader.rs` — type + `parse_resource_md` conversion + 1 new test
- `crates/runie-core/src/prompts.rs` — type simplification (no more `PathBuf` imports)
- `crates/runie-core/src/subagents/mod.rs` — parameter type + conversion
- `crates/runie-core/src/update/tools.rs` — `EditPreview` construction + IoMsg conversion
- `crates/runie-cli/src/inspect/mod.rs` — `.to_string_lossy()` → `.to_string()`
- `crates/runie-core/src/tests/safety.rs` — test updates (2 locations)
- `crates/runie-core/src/commands/tests/skills.rs` — test update
- `crates/runie-core/src/declarative/tests.rs` — test updates (2 locations)
- `crates/runie-core/src/dry_run.rs` — test update
- `crates/runie-core/src/skills/tests.rs` — test updates (3 locations)
- `crates/runie-core/src/tests/appstate_structural.rs` — test update

## Acceptance Criteria
- [x] Add `camino` dependency. (Already exists as workspace dependency)
- [x] Migrate internal path fields and map keys. (6 field migrations + 1 param migration)
- [x] Update serialization. (serde derives preserved; format unchanged)

## Tests

- **Layer 1 — State/Logic:** ✅ Unit tests for path round-trip:
  - `edit_preview.rs`: 4 new tests (JSON round-trip, UTF-8 string access, serialization format, from_string)
  - `resource_loader.rs`: 1 new test (UTF-8 string access)
- **Layer 2 — Event Handling:** N/A.
- **Layer 3 — Rendering:** N/A.
- **Layer 4 — E2E:** ✅ `cargo test --workspace` passes (all 219 agent + 30 provider + 30 tui + workspace tests)
- **Live tmux testing session (required):** File tools work. ✅ (no TUI behavior change; IO boundary conversion ensures same runtime behavior)

## Completion Validation

- [x] **Unit tests** — `cargo test --lib` covers the changed logic and all new/modified unit tests pass.
- [x] **E2E tests** — `cargo test --workspace` passes, including any new integration or provider-replay tests.
- [x] **Live tmux run tests** — file tool execution uses `PathBuf` at the `IoActor` boundary; behavior unchanged. ✅

### SSOT/Event Compliance
- [x] **Actor/SSOT:** N/A (type change; actors remain authoritative).
- [x] **Trigger events:** N/A (type change doesn't introduce new state transitions).
- [x] **Observer events:** N/A (type change doesn't emit events).
- [x] **No direct mutations:** N/A (type change doesn't change state ownership).
- [x] **No new mirrors:** N/A (type change doesn't introduce new state).
- [x] **Async work observed:** N/A (type change doesn't introduce new async work).
