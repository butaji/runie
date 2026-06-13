# Extract Effect Handlers from `runie-term` Main Loop

**Status**: todo
**Milestone**: R3
**Category**: Core Architecture
**Priority**: P1

## Description

`crates/runie-term/src/main.rs` contains a single `event_loop` that mixes
pure state updates with blocking/async side effects:

- `OpenExternalEditor` spawns a blocking editor task.
- `CopyToClipboard` / `CopyLastResponse` write OSC 52 sequences.
- `ShareSession` performs an HTTP request.
- `Suspend` manipulates terminal state and sends `SIGTSTP`.
- `LoginFlowSubmitKey` triggers an async API-key validation task.
- `SpawnAgent` runs a subagent in a blocking task.

This violates the project’s actor/state-update separation and makes the
loop impossible to unit test. Side effects should be handled by small,
typed effect actors/tasks that receive commands and emit `CoreEvent`s
back into the main loop.

## Acceptance Criteria

- [ ] A new `effects` module (or separate actors) in `runie-term` owns
  external-editor, clipboard, share, suspend, login validation, and
  subagent effects.
- [ ] Each effect is triggered by sending a command to its task/actor;
  the result is delivered back as a `CoreEvent`.
- [ ] `event_loop` no longer contains inline branches for these effects
  beyond dispatching the command and processing the returned event.
- [ ] `cargo build --workspace` succeeds.
- [ ] `cargo test --workspace` succeeds.
- [ ] `cargo clippy --workspace` produces no new warnings.

## Tests

### Layer 1 — State/Logic
- [ ] `effect_command_serializes` — each effect command can be created
  and inspected.
- [ ] `clipboard_effect_noop_when_unsupported` — if capabilities report
  no clipboard, no OSC sequence is emitted.

### Layer 2 — Event Handling
- [ ] `external_editor_done_event_replays` — the effect task sends
  `ExternalEditorDone { content }` after the editor exits.
- [ ] `share_event_returns_system_message` — share success/failure emits
  the correct `SystemMessage` event.
- [ ] `login_validation_emits_models_fetched_or_failed` — validation
  emits `LoginFlowModelsFetched` or `LoginFlowValidationFailed`.

### Layer 3 — Rendering
- [ ] No rendering changes.

### Layer 4 — Smoke
- [ ] `tmux_login_logout_test.sh` passes.
- [ ] `./dev.sh` runs end-to-end.

## Notes

**Suggested module layout:**

```
crates/runie-term/src/effects/
├── mod.rs       # Effect command enum and dispatch
├── editor.rs    # OpenExternalEditor
├── clipboard.rs # CopyToClipboard / CopyLastResponse
├── share.rs     # ShareSession HTTP call
├── suspend.rs   # SIGTSTP + terminal restore
├── login.rs     # LoginFlowSubmitKey validation
└── subagent.rs  # SpawnAgent
```

**Out of scope:**
- Changing the public `runie_core::Event` enum.
- Refactoring the render actor (it is already separate).

## Verification

```bash
cargo build --workspace
cargo test --workspace
cargo clippy --workspace
./dev.sh
```
