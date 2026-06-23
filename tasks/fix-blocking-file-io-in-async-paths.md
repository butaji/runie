# Fix synchronous file IO running directly on async tasks

**Status**: todo
**Milestone**: R3
**Category**: Architecture / Actors
**Priority**: P1

**Depends on**: none
**Blocks**: none

## Description

Several production code paths call `std::fs` directly from code that runs on the Tokio runtime, violating the async IO discipline in `docs/Architecture.md`. The affected paths are:

- `@` file picker / `complete_at_ref` (`file_refs.rs` functions called from update dispatcher).
- Path completion (`path_complete.rs`, `update/dialog/tab_complete.rs`).
- Pending edit apply fallback (`update/tools.rs approve_edits`).
- Session import/export fallback (`update/command.rs`).
- Skill reload (`commands/dsl/handlers/system.rs handle_reload`).
- Hashline edit validation/apply (`harness_skills/hashline_edit.rs`).
- `IoActor::write_files` (`actors/io/actor.rs`).

The fix is to move blocking calls off the async runtime using `tokio::fs`, `tokio::task::spawn_blocking`, or the existing `block_in_place_if_runtime` helper.

## Acceptance Criteria

- [ ] No `std::fs` read/write/dir call remains on a code path reached from an async actor without offloading.
- [ ] `cargo test --workspace` passes.
- [ ] `cargo clippy --workspace` reports no new warnings.

## Tests

### Layer 1 — State/Logic
- [ ] Add pure tests for any new sync helper wrappers (e.g., `write_files_sync` behavior unchanged).

### Layer 2 — Event Handling
- [ ] Add/update tests that drive `AppState::update` with an edit approval and verify `FilesWritten` is emitted without blocking the runtime (use `tokio::time::timeout` around the update).

### Layer 3 — Rendering
- N/A.

### Layer 4 — Provider Replay / E2E
- N/A.

## Files touched

- `crates/runie-core/src/file_refs.rs`
- `crates/runie-core/src/path_complete.rs`
- `crates/runie-core/src/update/tools.rs`
- `crates/runie-core/src/update/command.rs`
- `crates/runie-core/src/commands/dsl/handlers/system.rs`
- `crates/runie-core/src/harness_skills/hashline_edit.rs`
- `crates/runie-core/src/actors/io/actor.rs`
- `crates/runie-core/src/async_io.rs` (if a helper variant is needed)

## Implementation

Use `block_in_place_if_runtime` for call sites that must remain synchronous (update dispatcher, sync skill hooks). Use `spawn_blocking` for actor-internal work.

### 1. `file_refs.rs` callers

Keep `file_refs` functions synchronous and fast. Wrap the call sites in the update dispatcher:

In `crates/runie-core/src/update/dialog/fff.rs:46`:

```rust
let entries = crate::async_io::block_in_place_if_runtime(|| {
    crate::file_refs::find_file_entries(".", limit)
});
```

In `crates/runie-core/src/update/dialog/open.rs:157` and `file_picker.rs:65`:

```rust
let entries = crate::async_io::block_in_place_if_runtime(|| {
    super::fff::query_fff_files(query, 50)
});
```

(Alternative: make `query_fff_files` itself wrap `find_file_entries` with `block_in_place_if_runtime` so callers stay simple.)

### 2. `path_complete.rs` and `tab_complete.rs`

In `crates/runie-core/src/path_complete.rs:31`, wrap `collect_completions`:

```rust
crate::async_io::block_in_place_if_runtime(|| collect_completions(&parent, &prefix))
```

In `crates/runie-core/src/update/dialog/tab_complete.rs:109`, wrap the `read_dir` call similarly or call `path_complete::complete_path` which now offloads internally.

### 3. `update/tools.rs approve_edits`

Around line 155, replace direct `std::fs::write` with:

```rust
match crate::async_io::block_in_place_if_runtime(|| std::fs::write(&path, content)) {
    Ok(()) => applied += 1,
    Err(e) => errors.push(format!("{}: {}", preview.path.display(), e)),
}
```

(The `IoActor` branch at line 146 already offloads; this fallback is the only direct IO.)

### 4. `update/command.rs import/export fallbacks`

In `run_import_command` fallback (line 103):

```rust
let result = crate::async_io::block_in_place_if_runtime(|| {
    std::fs::read_to_string(&path_buf)
        .ok()
        .and_then(|json| serde_json::from_str::<Session>(&json).ok())
})
.map(|session| { ... })
.unwrap_or_else(|| CommandResult::Message(format!("Could not import session from '{}'", path)));
```

In `run_export_command` fallback (line 140):

```rust
let result = crate::async_io::block_in_place_if_runtime(|| {
    std::fs::write(&path_buf, json)
});
```

### 5. `commands/dsl/handlers/system.rs handle_reload`

Around line 144:

```rust
state.skills = crate::async_io::block_in_place_if_runtime(crate::skills::load_all);
```

### 6. `harness_skills/hashline_edit.rs`

Wrap the file reads/writes in `validate_hashes` and `apply_edits`:

```rust
let content = crate::async_io::block_in_place_if_runtime(|| {
    std::fs::read_to_string(path)
})?;
```

```rust
crate::async_io::block_in_place_if_runtime(|| std::fs::write(path, &new_content))
    .map_err(|e| format!("Error writing {}: {}", path.display(), e))?;
```

### 7. `actors/io/actor.rs write_files`

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

### Step 8: Run tests

```bash
cargo test --workspace
cargo clippy --workspace
```

### Step 9: Commit

```bash
git add crates/runie-core/src/file_refs.rs crates/runie-core/src/path_complete.rs \
  crates/runie-core/src/update/tools.rs crates/runie-core/src/update/command.rs \
  crates/runie-core/src/commands/dsl/handlers/system.rs \
  crates/runie-core/src/harness_skills/hashline_edit.rs \
  crates/runie-core/src/actors/io/actor.rs tasks/fix-blocking-file-io-in-async-paths.md tasks/index.json
git commit -m "fix(core): move blocking file IO off async runtime"
```

## Notes

- `block_in_place_if_runtime` panics if called from within another `block_in_place`; ensure callers are not already on a blocking thread.
- Where possible, prefer native `tokio::fs` for new async functions instead of wrapping sync code.
- `skills::load_all` is already wrapped in `spawn_blocking` in `runie-tui/src/app_init.rs`; the `handle_reload` path is the missing one.
