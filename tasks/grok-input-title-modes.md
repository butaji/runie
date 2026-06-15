# Mode Suffix in Input Title

**Status**: todo
**Milestone**: R4
**Category**: TUI / Rendering
**Priority**: P0

**Depends on**: r4-solo-team-mode-toggle
**Blocks**: (none)

## Description

Runie's input box title currently shows `provider/model`. Add execution-mode
suffixes so the user always knows which mode is active, matching Grok's
`grok-build · plan` style but limited to Runie's existing/planned modes.

## Acceptance Criteria

- [ ] Input title format becomes `provider/model · mode1 · mode2 ...`.
- [ ] `Solo` or `Team` suffix shown when execution mode is set.
- [ ] `read-only` suffix shown when `read_only` is true.
- [ ] No suffix for the default Solo/read-write state to reduce noise.
- [ ] Existing input box rendering tests still pass.

## Tests

### Layer 1 — State / Logic

```rust
#[test]
fn input_title_includes_team_mode() {
    let mut state = AppState::default();
    state.session.execution_mode = ExecutionMode::Team;
    let snap = state.snapshot();
    assert!(snap.input_title.contains("Team"));
}

#[test]
fn input_title_includes_read_only() {
    let mut state = AppState::default();
    state.read_only = true;
    let snap = state.snapshot();
    assert!(snap.input_title.contains("read-only"));
}

#[test]
fn default_input_title_has_no_mode_suffix() {
    let state = AppState::default();
    let snap = state.snapshot();
    assert!(!snap.input_title.contains("Solo"));
    assert!(!snap.input_title.contains("read-only"));
}
```

### Layer 3 — Rendering

```rust
#[test]
fn input_box_renders_team_suffix() {
    // TestBackend assertion: title bar contains "Team".
}
```

## Files touched

- `crates/runie-core/src/snapshot.rs` (add `input_title` field)
- `crates/runie-core/src/model/cache.rs` (build input_title)
- `crates/runie-tui/src/ui/input.rs`

## Out of scope

- Plan mode / always-approve suffixes (modes that don't exist yet).
