# Fix build.rs Lint: Test File Detection and Archive Exclusion

**Status**: done
**Milestone**: MVP
**Category**: Core Architecture
**Priority**: P0
**Depends on**: resolve-merge-conflicts

## Description

The workspace lint at `build.rs` (166 lines) was updated in commit
`402943c5` to:
- Raise thresholds: `MAX_FILE_LINES` 500→1000, `MAX_FUNCTION_LINES`
  40→80, `MAX_COMPLEXITY` 10→15
- Populate `ALLOWED_FILES_OVER` with ~30 entries (covering test files
  and binaries)
- Add `RUNIE_SKIP_BUILD_CHECKS` early return (line 60-63) for
  emergency builds
- Add a comment "Tuned to the structural shape of the code"

**Two real bugs remain:**

1. **`walkdir` does not skip `crates/_archive/`.** Lines 53-66 of
   `build.rs` recurse into every subdirectory under `crates/`. The
   archived files at `crates/_archive/runie-ext/src/mcp.rs` (476
   lines) and `crates/_archive/wireframe_layout.rs` etc. are linted
   even though they're not part of the build. If a future commit
   adds an oversized file to `_archive/`, the build will fail for a
   file that no one can fix without going to `_archive/`.

2. **The lint does not differentiate test files from source files.**
   Test files are held to the same 1000-line cap as production code.
   This means a 900-line integration test (e.g.
   `crates/runie-term/tests/e2e_legacy.rs`, 1215 lines) is allowed
   only because someone added it to `ALLOWED_FILES_OVER`. The
   allow-list will keep growing as new test files are added.

3. **`ALLOWED_FUNCS_OVER` is empty.** The lint detects long functions
   (line 80+) and would currently flag every function in
   `commands/handlers/session.rs` (the `register` function is 185
   lines). But because `ALLOWED_FUNCS_OVER` is empty, the lint would
   flag them — but the lint appears to be **silently failing** in
   that case. Or the build isn't being run.

   Looking at `build.rs:111-119`:
   ```rust
   if fn_len > MAX_FUNCTION_LINES {
       errors.push(format!(...));
   }
   ```
   This pushes the error but the function continues. Let me check
   whether the function-length check is actually being applied.

4. **Complexity check is the same as before** — counts `if `,
   `match `, `while `, `for ` substrings, which produces false
   positives (e.g. `if_chain_match` in a string is counted as both
   `if` and `match`). The threshold of 15 is also high enough to
   never fire on real code.

## Acceptance Criteria

- [ ] `build.rs::walkdir` skips any path containing `/_archive/`
  (added as a single check, not by name)
- [ ] `build.rs::walkdir` skips any path containing `/target/`
  (already present, verify)
- [ ] `build.rs::walkdir` skips any path containing `/tests/` for
  the file-length check (tests have different size budgets; allow
  1500 lines for test files)
- [ ] `build.rs::walkdir` does **not** skip test files for the
  function-length and complexity checks (test files can have
  oversize `#[test] fn`s but they should still be flagged)
- [ ] The build.rs `errors` accumulator is **actually checked** at
  the end (verify the function continues correctly and the
  process exits with `std::process::exit(1)` on non-empty errors)
- [ ] The `ALLOWED_FUNCS_OVER` allow-list is **populated** with
  every currently-oversize function (per the function scan below)
- [ ] The complexity check is **fixed** to count `if`/`match`/
  `while`/`for` as actual keywords (preceded by space, followed by
  space) rather than as substrings — or replaced with a proper
  parser
- [ ] A new test in `crates/runie-core/src/tests/lint_drift.rs`
  asserts that every entry in `ALLOWED_FILES_OVER` and
  `ALLOWED_FUNCS_OVER` points at a file/function that actually
  exists in the current tree (catches allow-list drift on
  file/function renames)

## Tests

### Layer 1 — State/Logic
- [ ] `cargo build` succeeds (the lint passes with the new
  exclusions)
- [ ] The newly-added `lint_drift` test passes:
  `cargo test -p runie-core --lib tests::lint_drift`
- [ ] `RUNIE_SKIP_BUILD_CHECKS=1 cargo build` still succeeds (escape
  hatch preserved)
- [ ] Adding a 1500-line test file to `crates/runie-core/src/tests/`
  does NOT fail the lint
- [ ] Adding a 1500-line source file to `crates/runie-core/src/`
  (not in the allow-list) DOES fail the lint with a clear message
- [ ] Adding a 1500-line file to `crates/_archive/somewhere/foo.rs`
  does NOT fail the lint (the archive is excluded)

### Layer 4 — Smoke
- [ ] `./dev.sh` runs end-to-end and triggers the lint

## Notes

**The current `build.rs` has reasonable thresholds but bad
mechanics.** Raising the limits from 500/40/10 to 1000/80/15 is a
sledgehammer — it lets oversize files exist but doesn't fix them.
A better design:
- Keep MAX_FILE_LINES at 500 for source files, allow 2000 for test
  files (test files can be long because they have many small
  `#[test] fn`s)
- Keep MAX_FUNCTION_LINES at 40 for source, allow 200 for tests
  (some integration tests are inherently large)
- Keep MAX_COMPLEXITY at 10 (the substring-counting bug is the real
  problem)

**Function scan** — every function that the build.rs would
currently flag (per the awk-based scan I ran during the review):

```
crates/runie-core/src/commands/handlers/session.rs:11   pub fn register  (185 lines)
crates/runie-core/src/commands/handlers/system.rs:7    pub fn register  (112 lines)
crates/runie-core/src/model_catalog.rs:54             pub fn model_catalog  (169 lines)
crates/runie-core/src/config_reload.rs:128            pub fn spawn_config_watcher  (85 lines)
crates/runie-core/src/keybindings.rs:15               pub fn default_keybindings  (60 lines)
crates/runie-core/src/keybindings.rs:140              pub fn event_from_name  (45 lines)
crates/runie-core/src/keybindings.rs:193              pub fn validate_key_combo  (71 lines)
crates/runie-core/src/update/settings_dialog.rs:40    pub fn build_setting_items  (93 lines)
crates/runie-agent/src/bin/reply_to_scenario.rs:34    fn main  (116 lines)
crates/runie-agent/src/bin/reply_to_scenario.rs:155   fn provider_event_to_agent_event  (68 lines)
crates/runie-agent/src/subagent.rs:29                 pub fn run_subagent  (55 lines)
crates/runie-tui/src/pipe/render/modes.rs:55          pub fn render_home_screen_mode  (48 lines)
crates/runie-tui/src/pipe/render/modes.rs:105         pub fn render_normal_mode  (65 lines)
crates/runie-tui/src/pipe/render/overlays.rs:13       pub fn render_overlays  (48 lines)
crates/runie-tui/src/pipe/render/overlays.rs:210      fn render_history_search  (71 lines)
crates/runie-tui/src/paint/mod.rs:120                 pub fn text  (43 lines)
```

The first three (`register`, `register`, `model_catalog`) are the
real problems. The rest are either tolerable or will be fixed by
other refactor tasks (e.g. `keybindings-table-driven` will shrink
`event_from_name` to < 10 lines; `split-update-mod` will shrink
`update/mod.rs` to < 500 lines).

**The current `build.rs` is not failing in CI** because
`RUNIE_SKIP_BUILD_CHECKS=1` is presumably set in the dev environment,
OR the lint exits 0 on the first error. Verify which before making
changes.

**Out of scope:**
- Replacing the build.rs complexity check with a proper Rust parser
  (use `syn` or `cargo-clippy`'s internals)
- Adding new lint rules (e.g. `clippy::too_many_arguments`)
- Splitting the 185-line `register` function in
  `commands/handlers/session.rs` (separate refactor task)
- The function-length drift test could be a build-time macro instead
  of a unit test (less runtime overhead) — separate task

**Verification:**
```bash
# Lint should pass with the current code
cargo build 2>&1 | tail -5

# The archive should be excluded
echo "pub fn x() { $(yes '{}' | head -1500 | tr -d '\n') }" \
  > crates/_archive/_lint_test.rs
cargo build 2>&1 | grep "RUNIE LINT" && echo "FAIL: archive is linted" \
  || echo "OK: archive is excluded"
rm crates/_archive/_lint_test.rs

# Test files should have relaxed limits
echo "#[test] fn x() { $(yes '{}' | head -2000 | tr -d '\n') }" \
  > crates/runie-core/src/tests/_lint_test.rs
cargo build 2>&1 | grep "RUNIE LINT" && echo "FAIL: tests are over-limit" \
  || echo "OK: tests have relaxed limits"
rm crates/runie-core/src/tests/_lint_test.rs
```
