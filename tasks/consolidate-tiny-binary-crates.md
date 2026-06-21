# Consolidate tiny binary crates into one

**Status**: todo
**Milestone**: R4
**Category**: Architecture / Actors
**Priority**: P2

**Depends on**: none
**Blocks**: none

## Description

Three sub-300-line binary crates exist as separate workspace members:

| Crate | LOC | Mode |
|-------|-----|------|
| `runie-print` | 97 | Non-interactive print mode |
| `runie-json` | 274 | Non-interactive JSON mode |
| `runie-server` | 283 | RPC server mode |

Each has its own `Cargo.toml`, workspace member entry, and duplicate setup. `consolidate-binary-setup` (done) deduplicated the shared setup code but kept 3 crates. YAGNI: fold `runie-print` and `runie-json` into `runie-tui` as subcommands (`runie print "..."`, `runie json "..."`), or into a single `runie-bin` crate with a mode flag. Keep `runie-server` separate only if RPC isolation is truly needed (evaluate: does it share deps that bloat the TUI?).

## Acceptance Criteria

- [ ] Audit complete: confirm `runie-print` and `runie-json` share all deps with `runie-tui` (no isolation benefit from separate crates).
- [ ] `runie-print` and `runie-json` crates deleted from workspace.
- [ ] Print/JSON modes available as `runie-tui` subcommands or a `runie-bin` crate with mode arg.
- [ ] `bin/runie-print` and `bin/runie-json` install targets updated (or removed if using subcommands).
- [ ] `Cargo.toml` workspace members reduced by 2.
- [ ] `cargo test --workspace` succeeds.
- [ ] `cargo check --workspace` succeeds with no new warnings.
- [ ] `README.md` "Modes" table updated to reflect new entry points.

## Tests

### Layer 1 — State/Logic
- [ ] `print_mode_outputs_same_as_before` — `runie print "find unused imports"` produces identical output to the old `runie-print` binary.
- [ ] `json_mode_outputs_valid_json` — `runie json "..."` produces the same JSON shape.

### Layer 2 — Event Handling
- [ ] N/A.

### Layer 3 — Rendering
- [ ] N/A — print/json modes are non-interactive.

### Layer 4 — Smoke / Crash
- [ ] `smoke_print_subcommand` — `./target/release/runie print "hello"` exits 0 with expected output.
- [ ] `smoke_json_subcommand` — `./target/release/runie json "hello"` exits 0 with valid JSON.

## Files touched

- `crates/runie-print/` → delete (fold into `runie-tui` or `runie-bin`)
- `crates/runie-json/` → delete (fold into `runie-tui` or `runie-bin`)
- `crates/runie-tui/src/main.rs` — add subcommand dispatch (or new `crates/runie-bin/src/main.rs`)
- `Cargo.toml` — remove 2 workspace members
- `Cargo.lock` — regenerated
- `README.md` — update Modes table
- `bin/` install scripts (if any reference deleted crates)

## Notes

Depends on `consolidate-binary-setup` (done) which already deduplicated shared setup. If `runie-server` is also folded, verify it doesn't pull RPC deps (`tokio` features, serialization) into the TUI binary that would bloat it. Rejected alternative: keep 3 crates "for separation" — rejected because separation without isolation benefit is YAGNI; the 3 `Cargo.toml`s + workspace entries are pure overhead at <300 LOC each.
