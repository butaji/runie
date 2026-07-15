# Per-test tmux sockets

## Objective

Run each test's tmux session on a unique socket path so that tests do not
serialize on the default tmux server. This removes the main source of
contention that currently forces `--test-threads=2`.

## Why this matters

The default tmux server is a global process. Multiple cargo test threads
compete for it, causing "capture-pane failed" flakiness and preventing higher
parallelism. Per-test sockets isolate sessions and let cargo run tests at
native parallelism.

## Status

**Implemented.**

## What was implemented

### `src/app_test.rs`

1. Added `socket_dir()` — returns `/tmp/runie-tests-<pid>/socket`, creating it
   with mode 0700 on first use. This is the known cleanup location.
2. Added `tmux_cmd_with_socket(socket: Option<&Path>) -> Command` — builds a
   `tmux` command with `TMUX_TMPDIR=/tmp` and optionally passes `-S <socket>`.
3. Added `socket_path: PathBuf` field to `TmuxSession`.
4. `TmuxSession::start()` generates a unique socket path per attempt and passes
   `-S <socket>` to `new-session` and all subsequent tmux commands.
5. Updated `TmuxSession::Drop`, `send_keys`, `capture_pane`, `close`,
   `pane_pid`, and the internal retry-cleanup calls to use the per-socket
   tmux command.
6. Updated `MockSessionCache::Drop` to extract `socket_path` from the cached
   `AppTest` session and pass `-S <socket>` to `kill-session`.
7. Updated `AppTest::kill_session` and `AppTest::Drop` to pass the socket path.

### `justfile`

Added `rm -rf /tmp/runie-tests-$(id -u)` at the start of `test` and `test-1`
recipes. This removes leaked sockets and temp directories from previous runs
before the tmux session cleanup, solving the `std::process::exit` static-dtor
leak problem.

## Socket path convention

```
/tmp/runie-tests-<pid>/socket/runie-tests-<pid>-<ts>-<n>.sock
```

- `<pid>` — process ID of the test binary, so cleanup only removes this process's sockets.
- `<ts>` — nanosecond timestamp for uniqueness across parallel tests.
- `<n>` — monotonically increasing counter.

## Cleanup strategy

Since `cargo test` exits via `std::process::exit` (skipping static destructors),
leaked sockets and temp directories are cleaned by the justfile before the next
run:

```make
rm -rf /tmp/runie-tests-$(id -u)
```

This removes all sockets and temp dirs created by this user, regardless of
whether the previous test binary exited cleanly.

## Remaining work

1. **Benchmark at higher thread counts.** Confirm that per-test sockets reduce
   contention enough to run at `--test-threads=4` or higher reliably.
2. **Update default thread count in justfile.** Once benchmarked, set the
   default `test` recipe to the optimal thread count.

## Acceptance checklist

- [x] Every tmux command issued by the harness uses a per-test `-S` socket.
- [x] Socket directory is cleaned before each test run (justfile).
- [ ] Suite passes reliably at `--test-threads=4`.
- [ ] Default `just test` thread count is updated based on the benchmark.
- [ ] `TEST_STATUS.md` Known Issues section no longer lists tmux parallelism as
      a hard limitation.

## Dependencies

- `improve_test_execution_speed` (done)
- `unify_replay_tui_harness` (done)
