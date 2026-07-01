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

1. **Unit tests** — Actor state correctly tracks spawned tasks via `JoinHandle` or `JoinSet`.
2. **E2E tests** — A multi-fixture replay run completes without leaked tasks.
3. **Live run tests** — Run an IO-heavy turn in tmux and verify clean shutdown with no orphaned processes.

## Tests

### Unit tests
- Actor state tracks spawned tasks correctly.

### E2E tests
- Events are still emitted after spawned tasks complete.

### Live run tests
- Run bash/file-tool operations in tmux, then shut down and confirm no lingering IO tasks.
