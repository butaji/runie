# Resolve Merge Conflicts

## Status: done

## Category: Core Architecture

## Priority: P0

## Blocks
- fix-broken-references
- fix-build-lint
- appstate-decomposition
- deduplicate-panel-types
- split-update-mod
- deduplicate-login-flow
- snapshot-dead-code
- clean-dead-modules

## Problem

Git history shows:
- `81becfa7 Merge win branch (resolved task conflicts)`
- `a1f4ae18 Merge review branch (resolved conflicts keeping agent-impl)`

The merge of the review branch left unresolved conflict markers in two files.

## Scope

Resolve conflict markers in:

1. **crates/runie-core/src/login_flow.rs** (1 conflict)
   - Lines 231-909: HEAD has tests, review is empty
   - Resolution: Keep HEAD (with tests)

2. **crates/runie-core/src/update/mod.rs** (8 conflicts)
   - Lines 1-5: HEAD has dialog imports
   - Lines 68-97: HEAD has providers_event handler
   - Lines 146-229: HEAD has providers_event, login_flow_event handlers
   - Lines 271-385: HEAD has login_flow methods
   - Lines 446-566: HEAD has login_flow_cancel and related methods
   - Lines 593-1603: HEAD has scroll_event, toggle_expand_all
   - Lines 1785-1925: HEAD has form_dialog_event, apply_form_action, form_build_submit

   Resolution: Keep HEAD (agent-impl) implementation throughout.

## Tests

### Layer 1: State/Logic
```bash
# After conflict resolution, verify project builds
cargo check -p runie-core
```

### Layer 2: Event Handling
```bash
# Run login flow tests
cargo test -p runie-core login_flow
```

## Acceptance Criteria

- [ ] No conflict markers (`<<<<<<<`, `=======`, `>>>>>>>`) in any .rs file
- [ ] `cargo check -p runie-core` passes
- [ ] `cargo test -p runie-core` passes
- [ ] All login flow tests pass
