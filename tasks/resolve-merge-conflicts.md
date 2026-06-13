# Resolve Merge Conflict Markers in Source

**Status**: done
**Completed in**: 77a605c3 ("Fix broken event routing in update dispatcher")
**Milestone**: MVP
**Category**: Core Architecture
**Priority**: P0

## Original Description

Four source files contained unresolved git merge conflict markers from the
`agent-impl` / `review` / `dev` triple-merge. `cargo build` failed on the
first one. The fix routed session events to `control_event`, settings
events to `dialog_toggle_event`, and removed duplicate `AtFilePicker`
match arms.

## Resolution Notes (audit trail)

The merge was resolved by commit `77a605c3` ("Fix broken event routing
in update dispatcher"), which **chose the agent-impl branch** as the
canonical version (consistent with the merge commits `a1f4ae18` and
`81becfa7`). The dispatcher in `update/mod.rs` was rewritten as a
single large `match event` (rather than the `EventCategory`-based
approach that the `review` branch had). The `Event::category()` and
`Event::is_login()` methods that the `review` branch introduced are
**still present in `event.rs:344-380`** and are called by the
match-based dispatcher (e.g. `update/mod.rs:56` calls
`event.is_login()`).

## Verification

```bash
# All conflict markers resolved
! git grep -nE '^(<{7}|={7}|>{7})' -- 'crates/**/*.rs'

# The four files are now single-version
git log --oneline -- crates/runie-core/src/update/mod.rs | head -3
git log --oneline -- crates/runie-core/src/model.rs | head -3
git log --oneline -- crates/runie-core/src/login_flow.rs | head -3
git log --oneline -- crates/runie-core/src/config_reload.rs | head -3
```

## Status

✅ Done. Build is unblocked. Subsequent tasks can proceed.

## Followups

The resolution preserved some quality issues that are addressed by
later tasks:

- `update/mod.rs` is 1901 lines (above the 1000-line `build.rs` lint
  cap) — see `split-update-mod`
- The login flow methods are still all in `mod.rs` (lines 232-595) —
  see `extract-login-flow`
- The `Event` enum has `LoginFlowValidate` variant that the dispatcher
  matches but `login_flow_event` ignores with `_ => {}` — see
  `extract-login-flow` and `clean-dead-modules`
