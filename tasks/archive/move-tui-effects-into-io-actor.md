# Move TUI effects (IO) out of the rendering crate

**Status**: done
**Milestone**: R4
**Category**: Architecture / Actors
**Priority**: P0

**Depends on**: finish-io-migration
**Blocks**: collapse-effect-payload-indirection

## Description

`crates/runie-tui/src/effects/` (6 files, 339 LOC) performs direct side effects inside the *rendering* crate, violating the "UI pure / MVU" and "Async IO discipline" rules in `docs/Architecture.md:13,274-285`:

| File | LOC | IO performed | Dep dragged into TUI |
|------|-----|--------------|----------------------|
| `share.rs` | 68 | `reqwest::Client::new()` POST to GitHub Gist API (`share.rs:18`) | `reqwest` (only reason TUI depends on it) |
| `editor.rs` | 38 | `tempfile::NamedTempFile::new()` + spawn `$EDITOR` (`editor.rs:20`) | `tempfile` (non-dev) |
| `clipboard.rs` | 102 | clipboard read/write + image decode | (arboard/png live in core, called from here) |
| `login.rs` | 63 | HTTP key validation via provider | (uses provider crate) |
| `suspend.rs` | 27 | suspend/resume terminal process group | (nix, already a unix dep) |
| `mod.rs` | 82 | dispatches `EffectCommand` → the above | — |

Architecture.md:18-21 states IO lives in `IoActor` behind async mockable interfaces. These effects should be `IoActor` messages that emit `ClipboardRead` / `GistShared` / `EditorClosed` / `KeyValidated` / `ProcessResumed` events, with the TUI only sending the request and rendering the result. This is distinct from `collapse-effect-payload-indirection`, which only addresses the `EffectPayload` enum indirection — not the fact that IO executes in the UI crate at all.

## Acceptance Criteria

- [x] `crates/runie-tui/src/effects/share.rs` no longer calls `reqwest` directly; gist upload is an `IoActor` message (`ShareSession { messages, display_name }`) that emits `GistShared(Result<String>)`.
- [x] `crates/runie-tui/src/effects/editor.rs` no longer spawns `$EDITOR` or creates temp files directly; it sends an `OpenExternalEditor { text }` message to `IoActor` and renders the `EditorClosed { result }` event.
- [x] `crates/runie-tui/src/effects/clipboard.rs` reads/writes clipboard via `IoActor` messages (`ReadClipboard` / `WriteClipboard` / `ReadClipboardImage`), emitting `ClipboardRead` / `ClipboardWritten` / `ClipboardImageRead` events.
- [x] `crates/runie-tui/src/effects/login.rs` key validation runs through `ProviderActor` (it already owns `DynProvider` construction and key validation per `docs/Architecture.md:43`); TUI only sends `ValidateKey { provider, key }` and renders `KeyValidated`.
- [x] `crates/runie-tui/src/effects/suspend.rs` suspend/resume moves to `IoActor` (`SuspendProcess` / `ResumeProcess`) since it manipulates the process group.
- [x] `reqwest` and `tempfile` removed from `crates/runie-tui/Cargo.toml` non-dev dependencies.
- [x] `crates/runie-tui/src/effects/` is deleted or reduced to a thin `EffectCommand → IoActor message` mapping (one file, <40 LOC).
- [x] `IoActor` gains handlers for the new messages; existing `actors/io/` traits extended or a new `actors/io/effects.rs` added.
- [x] `cargo test --workspace` succeeds.
- [x] `cargo check --workspace` succeeds with no new warnings.

## Tests

### Layer 1 — State/Logic
- [ ] `share_session_message_builds_gist_payload` — `ShareSession { messages, display_name }` message constructs the correct gist JSON without any HTTP call.
- [ ] `clipboard_command_carries_text` — `WriteClipboard { text }` message round-trips the text unchanged.

### Layer 2 — Event Handling
- [ ] `gist_shared_event_updates_share_status` — `GistShared(Ok(url))` event sets share status to "shared: <url>"; `GistShared(Err(e))` sets an error status.
- [ ] `editor_closed_event_applies_edited_text` — `EditorClosed { result: Ok(text) }` event updates the input buffer with the edited text.
- [ ] `clipboard_read_event_pastes_into_input` — `ClipboardRead(Ok(text))` event inserts the text at the cursor.

### Layer 3 — Rendering
- [ ] `share_status_renders_pending_then_url` — TestBackend render shows "Sharing…" then the gist URL after the events.

### Layer 4 — Smoke / Crash
- [ ] `smoke_share_session_with_mock_io_actor` — pressing the share key with a mocked `IoActor` (returns a fixed gist URL) completes without `reqwest` in the binary's dep tree.
- [ ] `smoke_editor_open_close_with_mock_io_actor` — editor open/close cycle works with a mocked `IoActor` returning canned text.
- [ ] `cargo tree -p runie-tui` shows no `reqwest` or `tempfile` edges (outside dev-deps).

## Files touched

- `crates/runie-tui/src/effects/share.rs` (delete or reduce to message construction)
- `crates/runie-tui/src/effects/editor.rs` (delete or reduce)
- `crates/runie-tui/src/effects/clipboard.rs` (delete or reduce)
- `crates/runie-tui/src/effects/login.rs` (delete or reduce)
- `crates/runie-tui/src/effects/suspend.rs` (delete or reduce)
- `crates/runie-tui/src/effects/mod.rs` (reduce to message mapping)
- `crates/runie-tui/Cargo.toml` (remove `reqwest`, `tempfile` from non-dev deps)
- `crates/runie-core/src/actors/io/` (new message handlers + event variants)
- `crates/runie-core/src/event/` (new event variants: `GistShared`, `EditorClosed`, `ClipboardRead`, `ClipboardImageRead`, `ClipboardWritten`, `KeyValidated`)
- `crates/runie-tui/src/ui_actor.rs` (send messages to `IoActor` instead of calling effects directly)

## Notes

This is the second half of the "UI pure" posture: `collapse-effect-payload-indirection` decides whether the `EffectPayload` enum stays; this task decides where the IO *executes*. Even if `EffectPayload` is kept as a pure-domain extraction, the execution must move to `IoActor`. Order: this task first (move IO), then `collapse-effect-payload-indirection` (decide enum fate). Dropping `reqwest` and `tempfile` from the TUI's non-dev deps is a concrete build-graph win independent of the `runie-core` split. The `clipboard_image.rs` (`arboard`+`png`) in core moves to `IoActor` as well — coordinate with `reconsider-clipboard-image-deps` (which may drop those deps entirely if clipboard-image is feature-gated).
