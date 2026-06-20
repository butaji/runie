# Gate or move single-consumer core modules

**Status**: todo
**Milestone**: R4
**Category**: Configuration
**Priority**: P2

**Depends on**: split-runie-core-into-domain-and-io-crates
**Blocks**: none

## Description

`runie-core` carries modules that only one downstream consumer (always the TUI) actually uses. Every binary pays to compile them:

| Module | LOC | Only consumer | Why it's in core |
|--------|-----|---------------|------------------|
| `ui/` (view DSL) | 1,040 | `runie-tui` | Renamed to `view/` by `rename-core-ui-to-view`; still in domain crate |
| `dialog/` (form DSL) | ~1,300 | `runie-tui` | TUI forms |
| `login_flow/` | ~1,100 | `runie-tui` | TUI login wizard |
| `providers_dialog.rs` | ~200 | `runie-tui` | TUI provider picker |
| `themes.rs` | 43 | `runie-tui` | TUI theme tokens |
| `markdown/` (render helpers) | 551 | `runie-tui` | TUI markdown render (domain parsing is pure; render helpers are not) |
| `ipc.rs` | 128 | `runie-server` (via `runie-cli`) | server only |

The `runie-print` / `runie-json` binaries never touch any of these. After `split-runie-core-into-domain-and-io-crates` establishes the domain/io boundary, move each TUI-only module into `runie-tui` (or a new `runie-ui` crate if the TUI gets too large), and move `ipc.rs` into the `runie-cli` server module. Where a module has a pure core (e.g. `markdown/` parsing vs. rendering), keep the pure part in domain and move only the render adapter.

## Acceptance Criteria

- [ ] `dialog/` DSL moved to `runie-tui` (or a `runie-ui` crate); `runie-domain` no longer contains form/panel rendering types.
- [ ] `login_flow/` moved to `runie-tui`; only pure `LoginFlowState` types (if any domain logic remains) stay in domain.
- [ ] `providers_dialog.rs` moved to `runie-tui`.
- [ ] `themes.rs` moved to `runie-tui` (joins `theme/` dir; see `unify-duplicate-module-names-core-tui`).
- [ ] `markdown/` pure parsing stays in `runie-domain`; render helpers move to `runie-tui` (see `unify-duplicate-module-names-core-tui`).
- [ ] `ipc.rs` moved into `runie-cli` server module (coordinate with `fold-protocol-into-core`).
- [ ] `view/` (renamed from `ui/`) stays in domain if it's a pure view-model, or moves to TUI if it's render-coupled — decide per `rename-core-ui-to-view` outcome.
- [ ] `cargo tree -p runie-cli` (print/json modes) shows no `pulldown-cmark`/`textwrap`/`unicode-width` edges unless the mode actually needs markdown.
- [ ] `cargo test --workspace` succeeds.
- [ ] `cargo check --workspace` succeeds with no new warnings.

## Tests

### Layer 1 — State/Logic
- [ ] `dialog_dsl_builds_form_after_move` — dialog DSL form construction tests pass from the new TUI location.
- [ ] `markdown_parser_still_parses_after_split` — pure markdown parsing tests pass from `runie-domain`.

### Layer 2 — Event Handling
- [ ] `login_flow_events_dispatch_after_move` — login flow event handlers work from the new TUI location.

### Layer 3 — Rendering
- [ ] `dialog_renders_in_test_backend_after_move` — TestBackend dialog render tests pass from TUI.

### Layer 4 — Smoke / Crash
- [ ] `smoke_print_binary_no_dialog_deps` — `cargo tree -p runie-cli` (print mode) shows no dialog/login_flow/themes edges.
- [ ] `smoke_tui_login_flow_intact` — TUI login wizard still drives the full flow after the move.

## Files touched

- `crates/runie-domain/src/dialog/` → `crates/runie-tui/src/dialog/`
- `crates/runie-domain/src/login_flow/` → `crates/runie-tui/src/login_flow/`
- `crates/runie-domain/src/providers_dialog.rs` → `crates/runie-tui/src/providers_dialog.rs`
- `crates/runie-domain/src/themes.rs` → `crates/runie-tui/src/theme/` (merge with existing)
- `crates/runie-domain/src/markdown/` (split: pure parsing stays, render helpers move)
- `crates/runie-domain/src/ipc.rs` → `crates/runie-cli/src/server/ipc.rs`
- All `Cargo.toml` edges referencing moved modules

## Notes

This task is the cleanup pass after `split-runie-core-into-domain-and-io-crates`. It depends on the split establishing where the domain/io boundary is. The decision rule: if a module has no pure domain logic and only the TUI calls it, it does not belong in the domain crate. `view/` is the judgment call — `rename-core-ui-to-view` decides whether it's a pure view-model (stays in domain) or render-coupled (moves to TUI). Coordinate with `unify-duplicate-module-names-core-tui` for the `themes`/`theme` and `markdown` duplicate-naming resolution.
