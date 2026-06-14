# Extract Effect Handlers from `runie-term` Main Loop

**Status**: done
**Milestone**: R3
**Category**: Core Architecture
**Priority**: P1

## Description

`crates/runie-term/src/main.rs` was refactored to move side-effect handling into
typed effect actors/tasks. All 6 effect types are now in `crates/runie-term/src/effects/`.

## Resolution

Implemented as `crates/runie-term/src/effects/` with the following modules:
- `editor.rs` — `OpenExternalEditor` effect
- `clipboard.rs` — `CopyToClipboard` / `CopyLastResponse` effect (OSC 52)
- `share.rs` — `ShareSession` HTTP effect
- `suspend.rs` — `Suspend` terminal restore effect
- `login.rs` — `LoginFlowSubmitKey` async validation
- `subagent.rs` — `SpawnAgent` effect

Each effect is triggered by sending a command to its task/actor; the result is
delivered back as a `CoreEvent`.

Archived in tasks/archive/.
