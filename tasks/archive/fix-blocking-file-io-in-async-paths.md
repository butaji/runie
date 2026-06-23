# Fix synchronous file IO running directly on async tasks

**Status**: done
**Milestone**: R3
**Category**: Architecture / Actors
**Priority**: P1

**Depends on**: none
**Blocks**: none

## Description

Several production code paths call `std::fs` directly from code that runs on the Tokio runtime, violating the async IO discipline in `docs/Architecture.md`.

Confirmed live paths:

- `@` file picker: `crates/runie-core/src/update/dialog/fff.rs:46` calls `file_refs::find_file_entries(".", limit)` from the update dispatcher.
- `open_at_file_picker` (`crates/runie-core/src/update/dialog/open.rs:157`) calls `query_fff_files`, which falls back to `find_file_entries`.
- `file_picker.rs` (`crates/runie-core/src/update/dialog/file_picker.rs:65`) also calls `query_fff_files`.
- Path tab completion: `crates/runie-core/src/update/path_complete.rs:13` calls `crate::path_complete::complete_path`, which calls `collect_completions` (`path_complete.rs:35`).
- Pending edit apply fallback: `crates/runie-core/src/update/tools.rs:155` calls `std::fs::write` directly.
- Session import/export fallbacks: `crates/runie-core/src/update/command.rs:103` and `:140`.
- Skill reload: `crates/runie-core/src/commands/dsl/handlers/system.rs:144` calls `crate::skills::load_all()` directly.
- `IoActor::write_files`: `crates/runie-core/src/actors/io/actor.rs:55` calls `write_files_sync` directly.

`hashline_edit.rs` already wraps `try_apply_hashline` in `block_in_place_if_runtime`, so it is **not** part of this task.

## Acceptance Criteria

- [ ] No live `std::fs` read/write/dir call remains on a code path reached from an async actor without offloading.
- [ ] `cargo test --workspace` passes.
- [ ] `cargo clippy --workspace` reports no new warnings.

## Tests

### Layer 1 — State/Logic
- [ ] Add/update pure tests for any new sync helper wrappers.

### Layer 2 — Event Handling
- [ ] Add/update tests that drive `AppState::update` with an edit approval and verify the expected system message is emitted without blocking the runtime.

### Layer 3 — Rendering
- N/A.

### Layer 4 — Provider Replay / E2E
- N/A.

## Files touched

- `crates/runie-core/src/file_refs.rs` (kept sync; callers wrap)
- `crates/runie-core/src/path_complete.rs` (kept sync; callers wrap)
- `crates/runie-core/src/update/tools.rs`
- `crates/runie-core/src/update/command.rs`
- `crates/runie-core/src/commands/dsl/handlers/system.rs`
- `crates/runie-core/src/actors/io/actor.rs`
- `crates/runie-core/src/update/dialog/fff.rs`, `open.rs`, `file_picker.rs`, `path_complete.rs`

## Implementation

Use `block_in_place_if_runtime` for call sites that must remain synchronous (update dispatcher, sync skill hooks). Use `spawn_blocking` for actor-internal work.

### 1. `file_refs` callers

Keep `file_refs` functions synchronous and fast. Wrap the call sites in the update dispatcher.

In `crates/runie-core/src/update/dialog/fff.rs:46`:

```rust
fn build_fff_fallback(limit: usize) -> Vec<FffFileEntry> {
    crate::async_io::block_in_place_if_runtime(|| {
        crate::file_refs::find_file_entries(".", limit)
    })
    .into_iter()
    .map(|e| FffFileEntry {
        name: e.name.clone(),
        path: e.name,
        is_dir: e.is_dir,
        score: 0.0,
        git_status: None,
    })
    .collect()
}
```

`open.rs:157` and `file_picker.rs:65` call `query_fff_files`, which now offloads internally.

### 2. `path_complete.rs`

Make `complete_path` offload `collect_completions`:

```rust
pub fn complete_path(partial: &str, cwd: &Path) -> Vec<PathCompletion> {
    let base: PathBuf = if partial.is_empty() {
        cwd.to_path_buf()
    } else {
        cwd.join(partial)
    };

    let (dir, prefix) = if partial.ends_with('/') || partial.is_empty() {
        (base, String::new())
    } else {
        let parent = base.parent().unwrap_or(cwd).to_path_buf();
        let prefix = base
            .file_name()
            .map(|s| s.to_string_lossy().to_string())
            .unwrap_or_default();
        (parent, prefix)
    };

    crate::async_io::block_in_place_if_runtime(|| collect_completions(&dir, &prefix))
}
```

In `crates/runie-core/src/update/path_complete.rs:13`, no change is needed because `complete_path` now offloads internally.

### 3. `update/tools.rs approve_edits`

Around line 155, replace direct `std::fs::write` with:

```rust
match crate::async_io::block_in_place_if_runtime(|| std::fs::write(&path, content)) {
    Ok(()) => applied += 1,
    Err(e) => errors.push(format!("{}: {}", preview.path.display(), e)),
}
```

### 4. `update/command.rs import/export fallbacks`

In `run_import_command` fallback (line 103):

```rust
let result = crate::async_io::block_in_place_if_runtime(|| {
    std::fs::read_to_string(&path_buf)
        .ok()
        .and_then(|json| serde_json::from_str::<Session>(&json).ok())
});
let result = result
    .map(|session| {
        let msg = format!("Session imported from '{}'", path);
        state.restore_session(&session);
        CommandResult::Message(msg)
    })
    .unwrap_or_else(|| {
        CommandResult::Message(format!("Could not import session from '{}'", path))
    });
dialog::process_command_result(state, result);
```

In `run_export_command` fallback (line 140):

```rust
let result = crate::async_io::block_in_place_if_runtime(|| std::fs::write(&path_buf, json));
let result = result
    .map(|_| CommandResult::Message(format!("Session exported to '{}'", path)))
    .unwrap_or_else(|e| CommandResult::Message(format!("Could not export: {}", e)));
dialog::process_command_result(state, result);
```

### 5. `commands/dsl/handlers/system.rs handle_reload`

Around line 144:

```rust
state.skills = crate::async_io::block_in_place_if_runtime(crate::skills::load_all);
```

### 6. `actors/io/actor.rs write_files`

Change `write_files` to offload:

```rust
async fn write_files(&self, edits: Vec<(PathBuf, String)>) {
    let (count, errors) = match tokio::task::spawn_blocking(move || write_files_sync(&edits)).await {
        Ok(res) => res,
        Err(e) => (0, vec![format!("write task failed: {e}")]),
    };
    self.emit(Event::FilesWritten { count, errors });
}
```

### Step 7: Run tests

```bash
cargo test --workspace
cargo clippy --workspace
```

### Step 8: Commit

```bash
git add crates/runie-core/src/update/dialog/fff.rs crates/runie-core/src/update/dialog/open.rs \
  crates/runie-core/src/update/dialog/file_picker.rs crates/runie-core/src/path_complete.rs \
  crates/runie-core/src/update/path_complete.rs crates/runie-core/src/update/tools.rs \
  crates/runie-core/src/update/command.rs crates/runie-core/src/commands/dsl/handlers/system.rs \
  crates/runie-core/src/actors/io/actor.rs tasks/fix-blocking-file-io-in-async-paths.md tasks/index.json
git commit -m "fix(core): move blocking file IO off async runtime"
```

## Notes

- `block_in_place_if_runtime` panics if called from within another `block_in_place`; ensure callers are not already on a blocking thread.
- Where possible, prefer native `tokio::fs` for new async functions instead of wrapping sync code.
- `skills::load_all` is already wrapped in `spawn_blocking` in `runie-tui/src/app_init.rs`; the `handle_reload` path is the missing one.
