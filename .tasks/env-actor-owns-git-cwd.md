# env-actor-owns-git-cwd

## Status: DONE

## Changes Made

### 1. Added `IoMsg::DetectEnv` message to IoActor
- **File**: `crates/runie-core/src/actors/io/messages.rs`
- Added `DetectEnv` variant to `IoMsg` enum
- Added `detect_env()` method to `IoActorHandle`

### 2. Added `detect_env` handler to IoActor
- **File**: `crates/runie-core/src/actors/io/actor.rs`
- Added `IoMsg::DetectEnv` case to the `handle` method
- Spawns blocking IO task to detect git info and cwd name
- Emits `Event::EnvDetected` fact on completion

### 3. Added `Event::EnvDetected` variant
- **File**: `crates/runie-core/src/event/variants.rs`
- Added `EnvDetected { git_info, cwd_name }` to the `Event` enum

### 4. Added serde derives to GitInfo
- **File**: `crates/runie-core/src/snapshot.rs`
- Added `Serialize` and `Deserialize` derives to `GitInfo` struct
- Required for `Event::EnvDetected` serialization

### 5. Updated event kind categorization
- **File**: `crates/runie-core/src/event/kind/mod.rs`
- Added `EnvDetected` to `is_io_fact()` function

### 6. Updated dispatch to handle `EnvDetected`
- **File**: `crates/runie-core/src/update/dispatch.rs`
- Added `Event::EnvDetected` handler in `handle_io_events()`
- Applies git_info and cwd_name to state via mutable accessors
- Refactored `categorize()` to reduce complexity using helper functions

### 7. Updated app_init to send intent
- **File**: `crates/runie-tui/src/app_init.rs`
- Removed direct blocking IO call for git detection
- Sends `IoMsg::DetectEnv` through IoActor handle
- The result is applied through the normal event dispatch path

### 8. Updated intent/fact detection
- **File**: `crates/runie-core/src/event/intent_impl.rs`
- Added `EnvDetected` to `is_fact_variant()` function

### 9. Updated dispatch tests
- **File**: `crates/runie-core/src/event/variants_tests/dispatch.rs`
- Added `EnvDetected` and `PermissionRequestDismissed` to exhaustive match

## Validation

- ✅ `cargo check --workspace` passes
- ✅ `cargo test --workspace` passes (all tests green)

## Lint Compliance

| File | Lines | Limit | Complexity | Limit |
|------|-------|-------|------------|-------|
| dispatch.rs | 94 | ≤500 | 9 | ≤10 |
| intent_impl.rs | 38 | ≤40 | - | - |

All files compile without lint violations.

## Architecture Notes

The git/cwd detection flow is now:
1. `app_init.rs` sends `IoMsg::DetectEnv` to `IoActor`
2. `IoActor::detect_env()` spawns blocking IO task
3. `IoActor` emits `Event::EnvDetected` fact
4. `dispatch.rs::handle_io_events()` applies the fact to state
5. State accessors (`git_info_mut()`, `cwd_name_mut()`) provide mutable access

This maintains the event-driven architecture where state changes flow through facts, not direct mutations.

## Task Updated

- `tasks/index.json` - status changed from `todo` to `done`

## Commit

```
3552ef96 env-actor-owns-git-cwd: move git/cwd detection to IoActor
```
