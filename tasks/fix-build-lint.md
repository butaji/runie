# Fix build.rs Lint Allow-List and Test File Detection

**Status**: todo
**Milestone**: MVP
**Category**: Core Architecture
**Priority**: P0
**Depends on**: resolve-merge-conflicts

## Description

The workspace lint at `build.rs` enforces 500-line file, 40-line function,
and 10-complexity caps, but:

1. The `ALLOWED_FILES_OVER` list is now only 2 entries (`update/mod.rs`,
   `model.rs`) and does not include the 6+ other files that legitimately
   exceed 500 lines (test files, vendor tables, providers).
2. The `ALLOWED_FUNCS_OVER` list is empty, so `update/mod.rs:131
   scroll_event` (110 lines) and 70+ other oversize functions will fail
   the lint as soon as the merge conflicts are resolved.
3. Test files are not excluded from the file-length check, so 7 test
   files > 500 lines will fail.
4. The lint path scanner (`walkdir`) has a leftover debug write to
   `/tmp/build_debug.txt` (no, wait — that's the *previous* version; the
   current `build.rs` doesn't have it. Confirmed during the review).

This task is purely the build.rs allow-list and exclusion rules — not the
actual code refactors that would shrink the offending files (those are
separate tasks).

## Acceptance Criteria

- [ ] `build.rs` skips any file under `tests/`, `benches/`, or ending in `_test.rs` / `_tests.rs` for the file-length check (test files are sized by different criteria)
- [ ] `ALLOWED_FILES_OVER` is updated to include all currently oversized *source* files. Required entries (at minimum):
  - `crates/runie-core/src/login_flow.rs` (909 lines)
  - `crates/runie-core/src/config_reload.rs` (571 lines)
  - `crates/runie-tui/src/tui/update/agent/events.rs` (verb conjugation tables if applicable)
  - Any new wired-orphan files that exceed 500 lines
- [ ] `ALLOWED_FUNCS_OVER` is updated to include all currently oversize functions. Required entries (at minimum):
  - `crates/runie-core/src/update/mod.rs:131` — `fn scroll_event` (110 lines)
  - `crates/runie-core/src/commands/handlers/session.rs:11` — `pub fn register` (185 lines)
  - `crates/runie-core/src/commands/handlers/system.rs:7` — `pub fn register` (112 lines)
  - `crates/runie-core/src/model_catalog.rs:54` — `pub fn model_catalog` (169 lines)
  - `crates/runie-core/src/keybindings.rs:15` — `pub fn default_keybindings` (60 lines)
  - `crates/runie-core/src/keybindings.rs:140` — `pub fn event_from_name` (45 lines)
  - `crates/runie-core/src/keybindings.rs:193` — `pub fn validate_key_combo` (71 lines)
  - `crates/runie-core/src/update/settings_dialog.rs:40` — `pub fn build_setting_items` (93 lines)
- [ ] `cargo build` with `RUNIE_SKIP_BUILD_CHECKS` unset runs the lint and reports zero violations
- [ ] The lint also catches new violations: temporarily add a 600-line function to a non-allowlisted file and confirm the build fails with a clear message
- [ ] `RUNIE_SKIP_BUILD_CHECKS=1` continues to bypass the lint for emergency builds (existing escape hatch)

## Tests

### Layer 1 — State/Logic
- [ ] `cargo build` succeeds with the updated allow-list
- [ ] `RUNIE_SKIP_BUILD_CHECKS=1 cargo build` succeeds even with intentionally oversize files
- [ ] A unit test in a new `crates/runie-core/src/lint_tests.rs` (or similar) verifies the allow-list is non-empty and points at files that exist (catches allow-list drift when files are deleted)

### Layer 4 — Smoke
- [ ] `./dev.sh` (the documented dev script) runs end-to-end and triggers the lint

## Notes

**Why allow-list rather than refactor everything now:** this task is the
*minimum* change required to make the build pass after the merge
conflicts are resolved. The actual refactors that would shrink
`update/mod.rs` to < 500 lines and `register()` to < 40 lines are
*separate* tasks (`split-update-mod` and the `commands/handlers/*`
table-driven refactor — see `keybindings-table-driven` for a similar
pattern).

**Test file exclusion rule:**
```rust
if path_str.contains("/tests/") || path_str.contains("/benches/") {
    continue;
}
```

**The lint also has no `pub(crate) fn` and no `async fn` detection in the
function-start matcher.** `async fn` declarations won't be detected as
function starts. This is a known limitation, not a blocker.

**Out of scope:**
- Splitting `update/mod.rs` (see `split-update-mod`)
- Converting `register()` to table-driven form (separate refactor)
- Adding a `clippy::too_many_arguments` check to the lint (those are currently silenced with `#[allow]` at call sites)
- Detecting `pub(crate) fn` as a function start

**Verification:**
```bash
cargo build 2>&1 | tail -5
# Should show no "RUNIE LINT VIOLATIONS" block

# And the inverse — confirm the lint actually fires:
echo "pub fn x() { $(yes '{}' | head -100 | tr -d '\n') }" > /tmp/lint_test.rs
cp /tmp/lint_test.rs crates/runie-core/src/_lint_test.rs
RUNIE_SKIP_BUILD_CHECKS=1 cargo build
rm crates/runie-core/src/_lint_test.rs
cargo build 2>&1 | grep "RUNIE LINT" && echo "OK: lint fires" || echo "FAIL: lint silent"
```
