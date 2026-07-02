# Round 5 — Config, Sessions, Persistence & Integration Roadmap

## Findings

### 1. Custom config merging / env override duplication

- `crates/runie-core/src/config/layers.rs:18-63` — uses `figment` but then manually re-implements env overrides for `RUNIE_PROVIDER`, `RUNIE_MODEL`, `RUNIE_THEME`.
- `crates/runie-core/src/config/config_impl.rs:436-455` — `resolve_default_model` duplicates fallback logic.
- `crates/runie-core/src/model/state/session.rs:130-165` — `ConfigState::default` duplicates default resolution with `#[cfg(test)]` branches.

### 2. Non-atomic config/session writes

- `crates/runie-core/src/config/config_impl.rs:321-339` — writes TOML under `fs2` lock but not atomically.
- `crates/runie-core/src/session/persistence/header.rs:39-47` — reads entire session file, prepends header, overwrites.
- `crates/runie-core/src/session/store.rs:75-88` — append triggers full-file header rewrite.

Use `atomicwrites` or `tempfile::NamedTempFile::persist` for atomic replacement.

### 3. Session JSONL parsing and replay bypass event pipeline

- `crates/runie-core/src/session/store.rs:233-288` — manual header detection by JSON shape.
- `crates/runie-core/src/session/replay.rs:22-59` — directly mutates `session_mut()` instead of applying events.

Replay should emit `Event`s and apply them through `AppState::update`. Use an explicit header delimiter.

### 4. State initialization order race

- `crates/runie-tui/src/main.rs:90-95` — initial snapshot may render before `EnvDetected`/`ConfigLoaded` arrive.

Ensure the first snapshot is published after bootstrap facts are applied, or render a loading state.

## Recommended changes

1. Remove manual env overrides from `layers.rs`; rely on Figment `Env`.
2. Provide a single `Config::resolve_default_model()` used everywhere.
3. Use atomic writes for config and session files.
4. Adopt snapshot + append-only JSONL journal instead of full-file rewrites.
5. Replay sessions by emitting `Event`s through `AppState::update`.
6. Fix initial snapshot race in TUI bootstrap.

## Integration roadmap (Pareto order)

1. **Event protocol simplification** (Round 1) — biggest SSOT win, unblocks durable/session cleanup.
2. **Provider HTTP/retry centralization** (Round 2) — removes duplicated fragile code.
3. **Tool registry + MCP decision** (Round 3) — determines whether to adopt `rmcp` broadly.
4. **TUI event routing cleanup** (Round 4) — reduces direct state writes and custom widgets.
5. **Config/session atomic JSONL** (Round 5) — completes the persistence story.

## Task mapping

| Finding | Task file | Status |
|---------|-----------|--------|
| Remove manual env overrides | `tasks/remove-manual-env-overrides-from-config-layers.md` | **new** |
| Single default-model resolver | `tasks/single-config-resolve-default-model.md` | **new** |
| Atomic config/session writes | `tasks/use-atomic-writes-for-config-and-session-files.md` | **new** |
| Snapshot + append-only JSONL journal | `tasks/adopt-snapshot-journal-jsonl-pattern.md` | **new** |
| Replay sessions via events | `tasks/replay-sessions-via-events-through-appstate.md` | **new** |
| Fix initial snapshot race | `tasks/fix-initial-tui-snapshot-race-after-bootstrap.md` | **new** |
| Execute second-pass roadmap | `tasks/execute-second-pass-architecture-review-roadmap.md` | **new** |
