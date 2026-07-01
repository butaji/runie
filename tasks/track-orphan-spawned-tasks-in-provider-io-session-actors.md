# Track orphan spawned tasks in provider, IO, and session actors

## Status

`todo`

## Description

`RactorProviderActor`, `RactorIoActor`, and `RactorSessionActor` spawn `tokio` tasks and discard the handles. These tasks are orphaned and cannot be cancelled or awaited on shutdown.

Target locations:
- `crates/runie-core/src/actors/provider/ractor_provider.rs:264-275`
- `crates/runie-core/src/actors/io/ractor_io.rs:157-228`
- `crates/runie-core/src/actors/session/session_handlers.rs:230`

## Acceptance criteria

- Every `tokio::spawn`/`spawn_blocking` stores its `JoinHandle` in actor state or a `JoinSet`.
- Replies are sent only after the spawned task completes.
- Actor shutdown awaits or cancels outstanding handles.

## Tests

### Layer 1 — State/Logic
- Actor state tracks spawned tasks correctly.

### Layer 2 — Event Handling
- Events are still emitted after spawned tasks complete.

### Layer 4 — Provider Replay / Mock-Tool E2E
- A multi-fixture replay run completes without leaked tasks.
