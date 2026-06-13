# Remove Orphan Modules and Duplicate AppState Implementation

**Status**: todo
**Milestone**: R3
**Category**: Core Architecture
**Priority**: P0

## Description

The live tree contains files that are written but never compiled, plus a
stale duplicate of `AppState` methods. These confuse reviewers, break
exhaustiveness assumptions, and can drift out of sync with the real
implementation.

Confirmed orphan / unreferenced files:

- `crates/runie-core/src/model/app_state.rs` — 423-line duplicate of
  `AppState` impl methods; not declared as a module in `model.rs` or
  `lib.rs`. It uses `Vec` caches while the live `model.rs` uses `Arc`,
  so it is actively inconsistent.
- `crates/runie-core/src/context.rs` — no `mod context` declaration.
- `crates/runie-core/src/scopecache.rs` — no `mod scopecache` declaration.
- `crates/runie-core/src/slash_command.rs` — no `mod slash_command`
  declaration.
- `crates/runie-core/src/config_reload/tests.rs` — never included
  (`config_reload.rs` has its own inline `#[cfg(test)] mod tests { ... }`).
- `crates/runie-tui/src/glyphs.rs` — no `mod glyphs` declaration.
- `crates/runie-tui/src/layout.rs` — no `mod layout` declaration.
- `crates/runie-tui/src/messages.rs` — no `mod messages` declaration.
- `crates/runie-term/src/keymap/convert.rs` — no `mod convert` declaration.
- `crates/runie-term/src/keymap/mapping.rs` — no `mod mapping` declaration.

The workspace root `build.rs` is also dead (the workspace is not a crate
with a build script) and should be removed.

## Acceptance Criteria

- [ ] Every file above is either deleted or wired into the build via a
  `mod` declaration.
- [ ] `model/app_state.rs` is deleted (not wired); `model.rs` remains the
  single source of truth for `AppState` methods.
- [ ] Root `build.rs` is deleted; `crates/runie-core/build.rs` remains the
  active lint build script.
- [ ] `cargo build --workspace` succeeds.
- [ ] `cargo test --workspace` succeeds.
- [ ] `cargo clippy --workspace` produces no new warnings.

## Tests

### Layer 1 — State/Logic
- [ ] `cargo build --workspace` succeeds after deletions.
- [ ] `cargo test --workspace` succeeds.
- [ ] `cargo clippy --workspace` has no new warnings.

### Layer 2 — Event Handling
- [ ] No event dispatch behavior changes (orphan modules were not
  reachable).

### Layer 3 — Rendering
- [ ] `cargo test -p runie-tui --lib` passes.

### Layer 4 — Smoke
- [ ] `./dev.sh` starts and exits cleanly.

## Notes

**Pre-deletion verification:**

```bash
for f in crates/runie-core/src/model/app_state.rs \
         crates/runie-core/src/context.rs \
         crates/runie-core/src/scopecache.rs \
         crates/runie-core/src/slash_command.rs \
         crates/runie-core/src/config_reload/tests.rs \
         crates/runie-tui/src/glyphs.rs \
         crates/runie-tui/src/layout.rs \
         crates/runie-tui/src/messages.rs \
         crates/runie-term/src/keymap/convert.rs \
         crates/runie-term/src/keymap/mapping.rs; do
  echo "=== $f ==="
  grep -R "mod ${f##*/}\b\|pub mod ${f##*/}\b" "$(dirname "$f")" || true
done
```

Expected: no `mod` declarations for the orphan files.

**Out of scope:**
- Refactoring the surviving code (handled in other tasks).
- Splitting oversized files.

## Verification

```bash
cargo build --workspace
cargo test --workspace
cargo clippy --workspace
ls build.rs  # should not exist
```
