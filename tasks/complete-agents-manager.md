# Complete the Agent Profile Manager

**Status**: todo
**Milestone**: R3
**Category**: Input & Commands
**Priority**: P1

## Description

`crates/runie-core/src/commands/agents_manager.rs` has a partial
implementation of the agent profile manager UI:

- `Event::OpenAgentsManager`, `AgentsManagerSave`, and
  `AgentsManagerDelete` are dispatched.
- `Event::AgentsManagerSetField { name, field, value }` is defined in
  `Event` but never handled.
- `AgentsManagerSave` does not use the edited form fields; it reloads
  the profile from disk and shows a transient.

As a result, users can open the manager and delete profiles, but cannot
edit fields or save changes from the form.

## Acceptance Criteria

- [ ] The agent-profile dialog form stores pending edits in
  `LoginFlowState` or a dedicated `AgentsManagerState`.
- [ ] `AgentsManagerSetField` updates the pending edit for the named
  profile and field.
- [ ] `AgentsManagerSave` persists the pending edits (via
  `agent_profiles::save_profile`) instead of reloading from disk.
- [ ] Validation rejects empty profile names and reports errors via a
  transient notification.
- [ ] `cargo build --workspace` succeeds.
- [ ] `cargo test --workspace` succeeds.

## Tests

### Layer 1 — State/Logic
- [ ] `set_field_updates_pending_edit` — setting a field changes the
  in-memory pending profile.
- [ ] `save_persists_pending_profile` — save writes the pending profile
  to disk and clears pending edits.
- [ ] `save_empty_name_fails` — empty name returns a warning/error
  result without writing.

### Layer 2 — Event Handling
- [ ] `agents_manager_set_field_event` — `AppState::update` routes
  `AgentsManagerSetField` to the manager and updates state.
- [ ] `agents_manager_save_event` — `AppState::update` routes
  `AgentsManagerSave` to persist the profile.

### Layer 3 — Rendering
- [ ] `agent_profile_form_renders_prefilled` — the form panel shows the
  existing profile values.

### Layer 4 — Smoke
- [ ] `./dev.sh` can open the agents manager, edit a field, save, and
  see the updated value after reopening.

## Notes

**Related files:**
- `crates/runie-core/src/agent_profiles.rs`
- `crates/runie-core/src/commands/handlers/agents.rs`
- `crates/runie-core/src/commands/agents_manager.rs`
- `crates/runie-core/src/event.rs`
- `crates/runie-core/src/update/mod.rs`

**Out of scope:**
- Adding new profile fields beyond what `AgentProfile` already supports.
- Import/export of profiles.

## Verification

```bash
cargo test -p runie-core --lib agents_manager
cargo test --workspace
cargo clippy --workspace
```
