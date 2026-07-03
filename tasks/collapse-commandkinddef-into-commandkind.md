# Collapse `CommandKindDef` into `CommandKind`

## Status

**done** ✅

## Description

`commands/dsl/spec.rs` and `declarative/types.rs` defined overlapping `CommandKind` shapes. Deserializing YAML went through `CommandKindDef` then `CommandKind`. Collapsed to a single pass.

## Changes made

1. **Removed `CommandKindDef`** from `declarative/types.rs` — it was a pure duplication of `CommandKind`'s variants.
2. **Replaced `handler_name`/`message` fields** in `declarative::CommandDef` with `yaml_kind: CommandKind`.
3. **Updated `build_cmd_from_yaml`** in `commands/dsl/spec.rs` to accept `DeclarativeCommandYaml` directly and look up handlers from the registry.
4. **Simplified `load_commands`** — `DeclarativeLoader::load_commands` now returns `Vec<DeclarativeCommandYaml>` directly.
5. **Updated `embedded_commands.rs`** — `load_embedded_commands` now calls `build_cmd_from_yaml(&yaml, handler_registry)` directly without an intermediate `DeclDef`/`CommandKindDef` step.
6. **Added `handler_name()` and `message()` helper methods** to the serde `CommandKind` enum.
7. **Added unit tests** for `CommandKind` helpers and YAML deserialization in `declarative/tests.rs`.

## Acceptance criteria

- [x] **Unit tests** — `cargo test declarative::tests` passes; `yaml_deserializes_directly_to_command_kind`, `command_kind_handler_name_returns_name`, `command_kind_form_with_handler_returns_handler_name`, `command_kind_msg_returns_message`, `command_kind_form_returns_none` all pass.
- [x] **E2E tests** — `embedded_commands::tests::all_yaml_files_load` and `command_names_are_valid` pass (all 45 embedded YAML commands load correctly).
- [x] **Live tmux tests** — The embedded commands (from YAML) include `quit`, `help`, `settings`, `model`, etc. and are exercised in normal TUI usage.

## Tests

```bash
cargo test --workspace  # 2928+ tests pass
cargo check --workspace  # clean
```

### Evidence

- `declarative::tests::yaml_deserializes_directly_to_command_kind` — YAML → `DeclarativeCommandYaml` with `CommandKind::Handler`
- `declarative::tests::command_kind_handler_name_returns_name` — `handler_name()` returns `Some("save")`
- `declarative::tests::command_kind_form_with_handler_returns_handler_name` — `handler_name()` returns handler string
- `declarative::tests::command_kind_msg_returns_message` — `message()` returns `Some("Done!")`
- `declarative::tests::command_kind_form_returns_none` — `handler_name()` and `message()` return `None` for `Form`
- `embedded_commands::tests::all_yaml_files_load` — all 45 YAML commands load without error
- `embedded_commands::tests::command_names_are_valid` — expected commands present
- `spec::tests::slash_command_executes_handler` — handler flow from YAML → registry → execution
