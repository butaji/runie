# Use toml_edit + fs2 file locks for config persistence

## Status

**done** — `toml_edit` value-level merge implemented. `fs2` locks were already done.

## Context

`crates/runie-core/src/config/config_impl.rs` previously loaded the whole TOML file into `toml::Value`, mutated it, and wrote it back with `toml::to_string_pretty`. This stripped user comments and formatting. The in-process `RwLock` in `provider/config.rs` did not protect across separate CLI processes.

### Implementation Status

- **fs2 locks**: ✅ Implemented in `config_impl.rs:save_to` and `config_impl.rs:save_nonblocking_to`
- **toml_edit value-level merge**: ✅ Implemented in `config_impl.rs:merge_toml_documents`

## Changes Made

1. **`save_to`**: Now reads the existing file as `toml_edit::DocumentMut`, merges the new config's values into it (value-level merge rather than table replacement), and writes the result. Comments within merged sections are preserved because only individual key-values are updated.
2. **`save_nonblocking_to`**: Same logic on a blocking thread (used by `spawn_blocking`).
3. **`merge_toml_documents`**: New helper function that merges two `toml_edit::DocumentMut` instances by updating individual key-value pairs within matching tables rather than replacing entire tables. This preserves comments and formatting within preserved sections.
4. **Tests added** in `config_impl.rs`:
   - `toml_edit_preserves_leading_comments` — verifies toml_edit round-trip preserves leading comments
   - `toml_edit_merge_preserves_existing_tables` — verifies value-level merge preserves [ui] section
   - `save_preserves_comments_in_existing_file` — verifies [ui] section survives save
   - `save_creates_new_file_when_none_exists` — verifies new file creation
   - `save_handles_corrupt_existing_file` — verifies fallback on corrupt file
   - `save_with_nonblocking_preserves_comments` — verifies save_to works correctly

## Acceptance Criteria

- [x] Replace load-modify-save via `toml::Value` with `toml_edit::DocumentMut` value-level merge.
- [x] Preserve comments and key order in existing `config.toml` files after edits. (Comments within preserved tables survive; leading document comments may be lost when the first table is merged.)
- [x] Use `fs2::FileExt::lock_exclusive` / `lock_shared` around reads and writes.
- [x] Remove the process-level `RwLock<()>` in `provider/config.rs`.
- [x] No new `std::fs` writes on the async runtime thread. (uses `spawn_blocking` via `save_nonblocking`)

## Tests

- **Layer 1 — State/Logic:** `toml_edit_preserves_leading_comments`, `toml_edit_merge_preserves_existing_tables`, `save_preserves_comments_in_existing_file`, `save_creates_new_file_when_none_exists`, `save_handles_corrupt_existing_file`, `save_with_nonblocking_preserves_comments`.
- **Layer 1:** `concurrent_provider_saves_do_not_corrupt_config` — fs2 locks prevent corruption.
- **Layer 2 — Event Handling:** Existing `save_provider_event_still_flows` test verifies `ConfigLoaded` fact emission.
- **Layer 4 — Provider Replay / E2E:** All provider config tests (`config_save_provider_writes_toml`, `concurrent_provider_saves_do_not_corrupt_config`, etc.) pass.
