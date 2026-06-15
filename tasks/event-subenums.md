# Split `Event` Enum (80+ Variants) into Focused Sub-Enums

**Status**: superseded
**Milestone**: R3
**Category**: Core Architecture
**Priority**: P2

**Superseded by**: `flatten-event-system`

## Description

`crates/runie-core/src/event.rs:309` defines an `Event` enum with
**80+ variants** (76 unit variants + 7 struct/tuple variants).
This contributes to the god-object pattern and forces the
dispatcher in `update/mod.rs:60-220` to be a 12-arm match each
touching 5-10 variants.

The variants fall naturally into 6 categories (the existing
`EventCategory` enum, currently dead, partitions them this way):

| Sub-enum | # variants | Examples |
|---|---|---|
| `InputEvent` | ~20 | `Input`, `Backspace`, `Newline`, `Submit`, `CursorLeft/Right/Start/End`, `DeleteWord/ToEnd/ToStart/KillChar`, `HistoryPrev/Next`, `Undo/Redo`, `CursorWordLeft/Right`, `Paste`, `PasteImage` |
| `AgentEvent` | ~10 | `AgentThinking`, `AgentThoughtDone`, `AgentToolStart/End`, `AgentResponse`, `AgentTurnComplete`, `AgentDone`, `AgentError` |
| `ScrollEvent` | 4 | `ScrollUp`, `ScrollDown`, `PageUp`, `PageDown` |
| `ControlEvent` | ~10 | `Quit`, `Reset`, `Abort`, `Suspend`, `OpenExternalEditor`, `ExternalEditorDone`, `ShareSession`, `SpawnAgent`, `Dequeue`, `FollowUp` |
| `ModelConfigEvent` | ~10 | `SwitchModel`, `SwitchTheme`, `CycleModelNext/Prev`, `CycleThinkingLevel`, `SetThinkingLevel`, `ToggleReadOnly`, `TrustProject`, `UntrustProject`, `ToggleScopedModelsDialog`, etc. |
| `DialogEvent` | ~10 | `ToggleCommandPalette`, `ToggleModelSelector`, `PaletteFilter`, `PaletteSelect`, `DialogBack`, etc. |
| `EditEvent` | ~10 | `PendingEdit`, `ApproveEdit`, `RejectEdit`, `RunSaveCommand`, `RunLoadCommand`, etc. |
| `SystemEvent` | ~5 | `SystemMessage`, `TransientMessage`, `TransientError`, `ClearTransient`, `ShowDiagnostics` |
| `LoginEvent` | ~10 | `LoginFlowStart`, `LoginFlowSelectProvider`, `LoginFlowSubmitKey`, `LoginFlowValidate`, `LoginFlowValidationDone`, `LoginFlowValidationFailed`, `LoginFlowModelsFetched`, `LoginFlowToggleModel`, `LoginFlowSave`, `LoginFlowCancel` |
| **Total** | **~80+** | |

## Acceptance Criteria

- [ ] A new `Event` enum exists that wraps focused sub-enums:
  ```rust
  pub enum Event {
      Input(InputEvent),
      Agent(AgentEvent),
      Scroll(ScrollEvent),
      Control(ControlEvent),
      ModelConfig(ModelConfigEvent),
      Dialog(DialogEvent),
      Edit(EditEvent),
      System(SystemEvent),
      Login(LoginEvent),
  }
  ```
- [ ] Each sub-enum is defined in its own file (e.g.
  `event/input.rs`, `event/agent.rs`, etc.) for ~200 lines total
  (vs 309 lines in one file)
- [ ] `From<SubEvent> for Event` impls make conversion cheap and
  ergonomic:
  ```rust
  let evt: Event = InputEvent::Submit.into();
  ```
- [ ] The dispatcher in `update/mod.rs::update()` becomes:
  ```rust
  pub fn update(&mut self, event: Event) {
      match event {
          Event::Input(e) => input_event(self, e),
          Event::Agent(e) => agent_event(self, e),
          // ...
      }
  }
  ```
- [ ] `Event::category()` and `Event::is_login()` methods are
  removed (subsumed by the match dispatch)
- [ ] The dead `EventCategory` enum (lines 310-322) is deleted
  (now subsumed)
- [ ] All call sites that construct `Event::Foo(...)` are updated
  to use the new typed variants
- [ ] `cargo build --workspace` succeeds
- [ ] `cargo test --workspace` succeeds (1,631 tests)

## Tests

### Layer 1 — State/Logic
- [ ] `cargo build --workspace` succeeds
- [ ] `cargo test --workspace` succeeds
- [ ] `cargo test -p runie-core --lib` passes (covers event
  dispatch via `update/mod.rs::update`)

### Layer 2 — Event Handling
- [ ] `cargo test -p runie-core --lib update::` passes (the
  dispatcher with sub-enums)

## Notes

**Why sub-enums:**
- **Pattern exhaustiveness**: each sub-enum is a closed set; the
  compiler enforces every variant is handled
- **Smaller match arms**: `match scroll_event` has 4 arms, not
  80
- **Better error messages**: `expected AgentEvent::ToolStart, got
  AgentEvent::ToolEnd` is clearer than `expected Event::X, got
  Event::Y` for a 80-variant enum
- **Type-driven routing**: `fn handle(e: AgentEvent)` is more
  self-documenting than `fn handle(e: Event) { match e { Agent* => ... } }`

**Why is this P2 not P0/P1:**
- The mechanical refactor touches 100+ call sites (every
  `Event::Foo` construction in tests and production code)
- The benefit is incremental (smaller match arms, better
  errors) — not a correctness or performance fix
- The dispatcher in `update/mod.rs` will still be 200+ lines
  unless combined with `split-update-mod`; doing them together
  is the natural sequence

**Migration strategy:**

1. **Phase 1**: Add the new sub-enums in `event/input.rs`,
   `event/agent.rs`, etc. Each is a simple type alias or
   re-export from the existing variants. No breaking changes.
2. **Phase 2**: Change `Event` to wrap sub-enums. The compiler
   errors on every call site. Fix them mechanically (add
   `.into()` everywhere).
3. **Phase 3**: Update `update/mod.rs::update` to use the
   sub-enum match. The 12-arm top-level match becomes a clean
   8-arm match.
4. **Phase 4**: Delete the now-dead `EventCategory` enum.

**Breaking change warning:** this is a public API change.
Every consumer of `Event` (including `runie-tui::ui::Snapshot`,
which serializes events for snapshots) needs to be updated.
Coordinate with `snapshot-dead-code.md` to keep the
serialization stable.

**Out of scope:**
- Splitting `keybindings::event_from_name` (covered by
  `keybindings-table-driven` task; once `Event` is split, the
  table will be a `match` over the sub-enums)
- Refactoring the dispatcher dispatchers (covered by
  `split-update-mod` task)
- The `CommandFlow` enum in `commands/dsl/flow.rs` (different
  enum, similar problem, separate task)

**Verification:**
```bash
cargo build --workspace
cargo test --workspace

# Event enum file is now smaller
wc -l crates/runie-core/src/event.rs
# Should be ~30 lines (just the Event enum wrapper)
```
