# Extract Session Restore Helper

**Status**: todo
**Milestone**: R3
**Category**: Sessions
**Priority**: P1

**Depends on**: none
**Blocks**: none

## Description

Session state restoration code is duplicated 3-4 times across:
- `session/mod.rs:416-437` (handle_resume)
- `session/io.rs:42-66` (handle_load)  
- `session/io.rs:84-110` (handle_import)

All copies restore these fields identically (12 lines):
```rust
state.session.messages = session.messages;
state.config.current_provider = session.provider;
state.config.current_model = session.model;
state.config.theme_name = session.theme_name;
state.config.thinking_level = session.thinking_level;
state.config.read_only = session.read_only;
state.session.session_display_name = session.display_name.or(Some(session.name));
state.session.session_created_at = session.created_at;
state.session.session_updated_at = session.updated_at;
state.session.session_tree = session.session_tree;
state.configure_token_tracker();
state.messages_changed();
```

Extract to `AppState::restore_session(session: Session)`.

## Acceptance Criteria

- [ ] Add `AppState::restore_session(&mut self, session: Session)` method
- [ ] Replace all 3-4 call sites
- [ ] `cargo test --workspace` succeeds
- [ ] ~40 LOC reduced

## Tests

### Layer 1 — State/Logic
- [ ] Existing session restore tests pass

### Layer 2 — Event Handling
- [ ] Load/resume/import command tests pass

### Layer 3 — Rendering
- [ ] N/A

### Layer 4 — Smoke / Crash
- [ ] `scripts/smoke-tmux.sh` passes

## Files touched

- `crates/runie-core/src/app.rs` (add method)
- `crates/runie-core/src/commands/dsl/handlers/session/mod.rs`
- `crates/runie-core/src/commands/dsl/handlers/session/io.rs`

## Notes

This pairs well with unifying Session construction (save/export) which also duplicates session field access.
