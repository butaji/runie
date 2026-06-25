# Event Taxonomy for Actor State Sync

**Status**: done
**Milestone**: R4
**Category**: Architecture / Actors
**Priority**: P0

**Depends on**: none
**Blocks**: app-state-read-only-projection, input-actor-owns-input-state, session-actor-owns-session-state, view-actor-owns-view-state, completion-actor-owns-completion-state, turn-actor-owns-agent-turn-state, permission-actor-owns-approvals, notification-actor-owns-transient-messages, trust-actor-owns-trust-decisions, env-actor-owns-git-cwd, fff-indexer-owns-file-picker-results

## Description

Document and enforce the event taxonomy for state synchronization. Events are classified into three categories: **Intents** (requests), **Facts** (state changes), and **Control** (app lifecycle).

## Event Taxonomy

### Intents (requests to actors)
- Fire-and-forget requests
- Handlers emit intents → actors receive and process
- Examples: `SetTheme`, `TrustProject`, `SubmitInput`, `RunCompact`
- Location: `crates/runie-core/src/event/intent.rs`

### Facts (state changes from actors)
- Produced by actors after processing intents
- Projected into `AppState` by the UI layer
- Examples: `ConfigLoaded`, `SessionChanged`, `TurnProgress`
- Location: `crates/runie-core/src/event/variants.rs` (Event enum with Fact variants)

### Control (app lifecycle)
- System-level events
- Handled by the main event loop
- Examples: `Quit`, `Suspend`, `TerminalSize`

## Implementation Status

| Category | Location | Status |
|----------|---------|--------|
| Intent enum | `event/intent.rs` | ✅ Implemented |
| Event enum (with Fact/Control variants) | `event/variants.rs` | ✅ Implemented |
| EventKind classification | `event/kind/mod.rs` | ✅ Implemented |
| Intent → Actor routing | `update/dispatch.rs` | ✅ Implemented |

### Fact Types (Event Variants)

| Fact | Description | Producer |
|------|-------------|----------|
| `ConfigLoaded` | Config loaded from disk | ConfigActor |
| `TrustLoaded` | Trust decisions loaded | SessionActor |
| `TrustChanged` | Trust decision changed | SessionActor |
| `HistoryLoaded` | Input history loaded | SessionActor |
| `HistoryAppend` | Input history appended | SessionActor |
| `SessionLoaded` | Session loaded from disk | SessionActor |
| `SessionSaved` | Session saved to disk | SessionActor |
| `SessionDeleted` | Session deleted | SessionActor |
| `SessionImported` | Session imported | SessionActor |
| `SessionExported` | Session exported | SessionActor |
| `SessionList` | Session list retrieved | SessionActor |
| `BashOutput` | Bash command output | IoActor |
| `FilesWritten` | Files written | IoActor |
| `EnvDetected` | Environment detected | EnvActor |
| `FffSearchResult` | FFF search results | FffIndexerActor |
| `Thinking`, `ToolStart`, `ToolEnd`, etc. | Agent lifecycle events | AgentActor |
| `PermissionRequest`, `PermissionResponse` | Permission events | PermissionActor |
| `ValidationFailed`, `ModelsFetched` | Login flow events | ProviderActor |
| `MessageReplayed` | Replay event | SessionActor |

### Planned Fact Types (Future)

| Fact | Description | Producer |
|------|-------------|----------|
| `InputStateChanged` | InputActor state changes | InputActor (planned) |
| `ViewChanged` | ViewActor cache invalidated | ViewActor (planned) |
| `TurnStarted`, `TurnProgress`, `TurnEnded` | Turn lifecycle | TurnActor (planned) |
| `DialogStateChanged` | UiControlActor state | UiControlActor (planned) |

## Acceptance Criteria

- [x] Event taxonomy documented
- [x] `Intent` enum covers all actor requests
- [x] `Event` variants cover all state changes (Facts)
- [x] `EventKind` classifies each event correctly
- [x] Intent → Actor routing documented
- [x] Missing fact types documented with planned actors
- [x] `cargo test --workspace` passes

## Tests

### Layer 1 — State/Logic
- [x] `event_kind_classifies_intents_correctly` (in kind_tests.rs)
- [x] `event_kind_classifies_facts_correctly` (in kind_tests.rs)
- [x] `event_kind_classifies_control_correctly` (in kind_tests.rs)

### Layer 2 — Event Handling
- N/A (documentation task)

### Layer 3 — Rendering
- N/A

### Layer 4 — Provider Replay / Mock-Tool E2E
- N/A

## Files touched

- `crates/runie-core/src/event/intent.rs` — typed intent enum
- `crates/runie-core/src/event/variants.rs` — Event enum with all variants
- `crates/runie-core/src/event/kind/mod.rs` — EventKind classification
- `crates/runie-core/src/event/kind/kind_tests.rs` — classification tests
- `crates/runie-core/src/update/dispatch.rs` — event routing

## Notes

- This task documents the existing architecture
- Missing fact types will be implemented as actors are created (ViewActor, TurnActor, InputActor, UiControlActor)
- The event taxonomy enables a clean separation between UI handlers and actor state
