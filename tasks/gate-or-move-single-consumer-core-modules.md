# Gate or move single-consumer core modules

**Status**: partial
**Milestone**: R4
**Category**: Configuration
**Priority**: P2

**Depends on**: split-runie-core-into-domain-and-io-crates
**Blocks**: none

## Description

`runie-core` carries modules that only one downstream consumer (always the TUI) actually uses. Every binary pays to compile them:

| Module | LOC | Only consumer | Status |
|--------|-----|---------------|--------|
| `ui/` (view DSL) | 1,040 | `runie-tui` | Not addressed - complex dependencies |
| `dialog/` (form DSL) | ~1,300 | `runie-tui` | Partial: copy in runie-tui, original stays in runie-core |
| `login_flow/` | ~1,100 | `runie-tui` | Not moved - depends on AppState types |
| `providers_dialog.rs` | ~200 | `runie-tui` | Not moved - depends on runie-core types |
| `themes.rs` | 43 | `runie-tui` | ✅ Moved: constants merged into `runie-tui/src/theme/` |
| `markdown/` (render helpers) | 551 | `runie-tui` | ✅ Done: markdown.rs → markdown_render.rs |
| `ipc.rs` | 128 | `runie-server` | ✅ Removed: not used anywhere |

## What was done

1. **themes.rs**: Moved `BUILTIN_THEMES` constant from `runie-core/src/themes.rs` to `runie-tui/src/theme/loader.rs`. The original file still exists in runie-core for backward compatibility.

2. **ipc.rs**: Removed from runie-core. The module was not used anywhere except its own test.

3. **dialog/**: Added a copy to `runie-tui/src/dialog/` with imports updated to use `runie_core::` paths. The original module still exists in runie-core for backward compatibility.

## What was NOT done (deferred)

- Full move of `dialog/` would require moving all dependent modules (`update/dialog/`, `commands/dsl/`, `login_flow/`, `provider/dialog.rs`)
- These modules depend on `AppState` and other runie-core types, creating circular dependency issues
- A cleaner solution would be to move ALL TUI-specific event handlers together

## Acceptance Criteria

- [ ] `dialog/` DSL moved to `runie-tui` (or a `runie-ui` crate); `runie-domain` no longer contains form/panel rendering types. — **Deferred**
- [ ] `login_flow/` moved to `runie-tui`; only pure `LoginFlowState` types (if any domain logic remains) stay in domain. — **Deferred**
- [ ] `providers_dialog.rs` moved to `runie-tui`. — **Deferred**
- [x] `themes.rs` moved to `runie-tui` (joins `theme/` dir). ✅
- [x] `markdown/` pure parsing stays in `runie-domain`; render helpers move to `runie-tui`. ✅
- [x] `ipc.rs` removed from runie-core (not used). ✅
- [ ] `view/` (renamed from `ui/`) stays in domain if it's a pure view-model, or moves to TUI if it's render-coupled. — **Deferred**
- [ ] `cargo tree -p runie-cli` shows no TUI-only deps. — **Deferred**
- [x] `cargo test --workspace` succeeds.
- [x] `cargo check --workspace` succeeds with no new warnings.

## Tests

### Layer 1 — State/Logic
- [x] `dialog_dsl_builds_form_after_move` — dialog DSL form construction tests pass from the new TUI location.
- [x] `markdown_parser_still_parses_after_split` — pure markdown parsing tests pass from `runie-domain`.

### Layer 2 — Event Handling
- [ ] `login_flow_events_dispatch_after_move` — login flow event handlers work from the new TUI location. — **Deferred**

### Layer 3 — Rendering
- [x] `dialog_renders_in_test_backend_after_move` — TestBackend dialog render tests pass from TUI.

### Layer 4 — Smoke / Crash
- [ ] `smoke_print_binary_no_dialog_deps` — `cargo tree -p runie-cli` shows no dialog/login_flow/themes edges. — **Deferred**
- [ ] `smoke_tui_login_flow_intact` — TUI login wizard still drives the full flow after the move. — **Deferred**

## Files touched

- `crates/runie-tui/src/theme/loader.rs` — added BUILTIN_THEMES constant
- `crates/runie-core/src/themes.rs` — kept for backward compat (re-exports constant)
- `crates/runie-core/src/ipc.rs` — deleted (unused)
- `crates/runie-tui/src/dialog/` — new directory with dialog module copy

## Notes

The full module move was blocked by circular dependency issues. The TUI-specific event handlers (update/dialog/, commands/dsl/, login_flow/) depend on AppState and other runie-core types. A cleaner solution would be to move these together as a group.

For now:
- `runie-tui` has its own `dialog` module that uses `runie_core::` imports
- `runie-core` still has the original `dialog` module for backward compatibility
- The facade pattern allows both to coexist
