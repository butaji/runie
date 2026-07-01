# Round 2 — Replace Custom Code with Crates / Libraries

## Findings

### 1. `EventBus<E>` is a thin tokio broadcast wrapper

The custom wrapper adds no backpressure/acknowledgement. Consider using `tokio::sync::broadcast` directly with explicit acknowledgement where lossless delivery matters.

### 2. Event taxonomy is hand-generated

`taxonomy.json` + four generated files are maintained by `scripts/generate-event-taxonomy.sh`. `strum` (`Display`, `IntoStaticStr`, `EnumIter`, `VariantNames`) can derive name tables and remove the codegen script.

### 3. `DurableCoreEvent` vs `Event` dual enums

`crates/runie-core/src/event/durable.rs` maintains a parallel enum with hand-written `TryFrom` in both directions. A single canonical `Event` enum with `#[serde(skip)]` transient fields would eliminate the conversion table.

### 4. `TurnState` + `AgentState` duplication

Same as Round 1 — single authoritative struct plus typed projection.

### 5. `SpeedWindow` is a custom ring buffer

`crates/runie-core/src/actors/turn/state.rs` (inferred) contains a hand-rolled window. The `ringbuffer` crate provides this.

### 6. `StreamingBuffer` / `InputState` are custom

`runie-tui` still carries custom input state and streaming-buffer classification. `tui-input`, `tui-textarea`, or `ropey` can replace them while keeping visual output frozen.

### 7. Config layering is custom

`crates/runie-core/src/config/config_impl.rs` + `file_helpers.rs` implement layered config merging. `figment` (`Toml::file`, `Env::prefixed`, `Serialized` defaults) is the standard replacement.

### 8. Permission policy chain is over-engineered

The policy chain (`DefaultToolApprove`, `GitTrackedWriteApprove`, `FileAccessAsk`, `PermissionSetPolicy`) can be simplified to a single ruleset evaluated with a plain `match` or, if middleware ordering is truly needed, `tower` layers.

### 9. `ProviderFactory` uses `Pin<Box<dyn Future>>`

`crates/runie-core/src/provider/provider_trait.rs` and factory implementations return pinned boxed futures. `async_trait` or a provider enum removes the boilerplate.

### 10. Session persistence is JSONL + dual SQLite

`crates/runie-core/src/session/sqlite_store.rs` exists but JSONL still coexists. Standardize on `sqlx` + migrations; keep JSONL only as an event-log export format.

### 11. Custom helpers still remain

Fuzzy/path/glob/keybinding/text helpers can be further replaced with `nucleo-matcher`, `globset`, `shellexpand`, `crokey`, `shell-words`, `textwrap`.

## Recommended changes

1. Adopt `strum` for event/intent/name tables and delete `taxonomy.json` generation.
2. Collapse `DurableCoreEvent` into `Event` with `#[serde(skip)]`.
3. Replace `SpeedWindow` with `ringbuffer`.
4. Replace custom input/streaming buffer with `tui-input`/`tui-textarea`/`ropey`.
5. Replace config layering with `figment`.
6. Simplify permission evaluation to a ruleset match.
7. Replace `ProviderFactory` pinned-box futures with `async_trait` or an enum.
8. Migrate session runtime state to `sqlx` + migrations; JSONL becomes export-only.
9. Finish replacing remaining custom helpers with crates.

## Task mapping

| Finding | Task file | Status |
|---------|-----------|--------|
| Event taxonomy → `strum` | `tasks/finish-strum-migration-for-remaining-enums.md`, `tasks/generate-event-taxonomy-or-delete-generated-files.md` | existing `todo` |
| Collapse durable/event dual enums | `tasks/collapse-durablecoreevent-into-event-enum.md` | **new** |
| `TurnState`/`AgentState` unification | `tasks/merge-agentstate-into-turnstate-projection.md` | existing `todo` |
| `SpeedWindow` → `ringbuffer` | `tasks/replace-speedwindow-with-ringbuffer-crate.md` | **new** |
| Input/streaming → `tui-input`/`tui-textarea` | `tasks/finish-replacing-custom-tui-widgets.md`, `tasks/replace-streaming-buffer-classifier-with-pulldown-cmark.md` | existing `todo` |
| Config layering → `figment` | `tasks/replace-layered-config-merge-with-figment.md` | existing `todo` |
| Simplify permission policy chain | `tasks/simplify-permission-policy-chain-to-ruleset-match.md` | **new** |
| `ProviderFactory` → `async_trait`/enum | `tasks/replace-providerfactory-pinbox-with-async-trait-or-enum.md` | **new** |
| Session persistence → `sqlx` | `tasks/migrate-session-persistence-to-sqlx-with-migrations.md` | **new** |
| Finish helper-crate replacements | `tasks/finish-replacing-remaining-custom-helpers-with-crates.md` | **new** |
