# Delete the 41,477 Lines of Dead Code in `runie-tui`

**Status**: done
**Milestone**: R1
**Category**: TUI / Rendering
**Priority**: P0

## Description

`crates/runie-tui/src/lib.rs` declares only **8 modules**: `diff`,
`markdown`, `message`, `popups`, `status_bar`, `syntax`, `theme`, `ui`
(the only modules wired into the live build). All other
directories exist on disk but are **not declared**, which means
their code never compiles as part of the workspace.

**Dead directories (not declared in `lib.rs`):**

| Directory | Lines | What was it |
|---|---|---|
| `crates/runie-tui/src/tui/` | 13,753 | Alternative TUI architecture (events, state, update, render, tests, tests_hotkeys, view_models) |
| `crates/runie-tui/src/components/` | 12,054 | 23 component subdirs (model_picker, plan_modal, settings_modal, message_list, onboarding, command_palette, etc.) |
| `crates/runie-tui/src/tui.rs` | 354 | Top-level TUI struct (354 lines, **broken imports** — references deleted `runie_agent::events` and `runie_agent::loop_engine`) |
| `crates/runie-tui/src/pipe/` | 1,279 | Render pipe (referenced by `tui.rs` and `components/`) |
| `crates/runie-tui/src/paint/` | 499 | Paint helpers |
| `crates/runie-tui/src/plugins/` | 283 | Plugin examples |
| `crates/runie-tui/src/style/` | ~200 | Style helpers |
| `crates/runie-tui/src/replay/` | ~150 | Session replay |
| `crates/runie-tui/src/actors/` | ~100 | Actor pattern (abandoned) |
| `crates/runie-tui/src/theme/themes/` | 408 | Multiple theme definitions |
| **Total** | **~41,477** | |

**Why this is the biggest reduction win in the codebase:**

The `runie-tui` crate is 50,455 lines total. **84% of it is dead code** that never compiles. The 8 declared modules total ~6,000 lines. The remaining ~44,000 lines is unreachable.

**The `tui.rs` file is specifically broken:**

```rust
// crates/runie-tui/src/tui.rs:19-21
use runie_agent::events::AgentEvent;
use runie_agent::PermissionDecision;
use runie_agent::loop_engine::PermissionState;
```

Both `runie_agent::events` and `runie_agent::loop_engine` were
deleted in commits `87052015` and `03d8aba2`. If you try to wire
`mod tui;` into `lib.rs`, the build fails with "module `events`
not found in `runie_agent`" and "module `loop_engine` not found in
`runie_agent`".

**Many test files in the dead directories have 0 `#[test]`
annotations:**

```
tui/tests/grok_parity_tests.rs          930 lines  0 tests
tui/tests/grok_element_tests.rs         713 lines  0 tests
tui/tests/session_management_tests.rs   742 lines  0 tests
tui/tests/palette_execution_tests.rs   ~150 lines  0 tests
tui/tests/palette_close_tests.rs       ~150 lines  0 tests
tui/tests/test_harness.rs               ~200 lines  0 tests
components/message_list/tests.rs        ~300 lines  0 tests
components/home_screen/mod_test.rs      ~200 lines  0 tests
components/home_screen/render_test.rs   ~200 lines  0 tests
```

That's 3,785+ lines of test scaffolding with **zero actual
assertions**. Likely artifact of one-shot test generation that
was never finished.

## Acceptance Criteria

- [x] `crates/runie-tui/src/lib.rs` is the only file that imports
  from the deleted directories (verify with `git grep`)
- [x] `git rm -r crates/runie-tui/src/tui.rs`
- [x] `git rm -r crates/runie-tui/src/tui/`
- [x] `git rm -r crates/runie-tui/src/components/`
- [x] `git rm -r crates/runie-tui/src/pipe/`
- [x] `git rm -r crates/runie-tui/src/paint/`
- [x] `git rm -r crates/runie-tui/src/plugins/`
- [x] `git rm -r crates/runie-tui/src/replay/`
- [x] `git rm -r crates/runie-tui/src/actors/`
- [x] `git rm -r crates/runie-tui/src/style/`
- [x] `git rm -r crates/runie-tui/src/theme/themes/`
- [x] `cargo build -p runie-tui` succeeds
- [x] `cargo test -p runie-tui` succeeds (with the test count
  reduced by ~400-500 since most tests were in the dead
  directories)
- [x] The `tests/` directory (declared in `lib.rs`) is preserved
  with all its 159 tests intact
- [x] `cargo test --workspace` succeeds

## Tests

### Layer 1 — State/Logic
- [x] `cargo build -p runie-tui` succeeds after the deletion
- [x] `cargo test -p runie-tui` succeeds; test count should drop
  from 669 to ~159 (the count of tests in the active `tests/`
  directory)
- [x] `git grep -rn 'crate::tui' crates/runie-tui/src/`
  returns zero hits (no references to deleted modules)
- [x] `git grep -rn 'crate::components\|crate::pipe\|crate::paint' crates/runie-tui/src/`
  returns zero hits

### Layer 4 — Smoke
- [x] `cargo run -p runie-term --bin runie` starts the TUI without
  panicking (the live render path uses `runie_tui::ui::draw_snapshot`,
  which is in the surviving `ui.rs`)

## Notes

**This task is a strict subset of `consolidate-tui-tests`.** The
existing `consolidate-tui-tests` task is framed as "merge two test
hierarchies" but the actual delta is: delete the 41k lines of
dead code (the `tui/`, `components/`, `pipe/`, etc. directories)
which contains both the unused test hierarchies AND a bunch of
non-test code that was never wired up.

If the `consolidate-tui-tests` task is the chosen path, this task
is redundant. They are equivalent in scope. Either rename
`consolidate-tui-tests` to "delete the dead `runie-tui` code" or
keep both and note the relationship.

**Why is this P0 and not P1?** This is the single largest
reduction opportunity in the codebase (~41k lines = 43% of
the entire workspace). The reduction is mechanical (delete
directories, no refactoring needed). No design decisions
required. The only risk is "what if some dead code is actually
useful?" — but the verification step (`git grep` for live
references) rules that out.

**Out of scope:**
- Restoring any of the deleted code (separate revival task; would
  require creating `runie-agent::events` and `runie-agent::loop_engine`
  again, plus migrating the live TUI to use them)
- The `tests/` directory (declared in `lib.rs`) — this is the
  ACTIVE test directory, keep it
- `crates/runie-tui/src/{diff,markdown,message,popups,status_bar,
  syntax,theme,ui}.rs` — these are the 8 declared modules
- Splitting `tui.rs` into smaller files (it's deleted as a unit)

**Verification:**
```bash
# After deletion
ls crates/runie-tui/src/tui/ 2>/dev/null && echo "FAIL" || echo "OK"
ls crates/runie-tui/src/components/ 2>/dev/null && echo "FAIL" || echo "OK"
ls crates/runie-tui/src/pipe/ 2>/dev/null && echo "FAIL" || echo "OK"
test -f crates/runie-tui/src/tui.rs && echo "FAIL" || echo "OK"

# Build clean
cargo build -p runie-tui
cargo test -p runie-tui
cargo test --workspace
```
