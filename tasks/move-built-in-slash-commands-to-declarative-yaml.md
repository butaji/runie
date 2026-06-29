# Move built-in slash command specs to declarative YAML resources

**Status**: todo
**Milestone**: R6
**Category**: Commands / DSL
**Priority": P2

**Depends on**: deserialize-declarative-command-yaml-with-typed-structs
**Blocks**: simplify-slash-command-dsl

## Description

The five `commands/dsl/handlers/*.rs` modules contain large static `CommandSpec` tables in Rust. Convert them to YAML files under `crates/runie-core/resources/commands/` and load/register them through the existing declarative loader. This removes ~600 lines of boilerplate and makes built-ins editable without recompilation.

## Acceptance Criteria

- [ ] Extract built-in command specs into YAML resources.
- [ ] Load them via `declarative::loader::load_commands_from_dir` at startup.
- [ ] Merge built-ins with user/project declarative commands.
- [ ] Delete the static tables from `commands/dsl/handlers/*.rs`.
- [ ] `cargo test --workspace` succeeds after the change.
- [ ] `cargo check --workspace` succeeds with no new warnings.

## Tests

### Layer 1 — State/Logic
- [ ] `built_in_commands_load_from_yaml` — built-ins are present after load.
- [ ] `built_in_command_count_matches` — count equals the previous static table.

### Layer 2 — Event Handling
- [ ] `slash_command_event_resolves` — a built-in slash command still resolves to the correct intent.

### Layer 3 — Rendering
- [ ] N/A.

### Layer 4 — Provider Replay / Mock-Tool E2E
- [ ] N/A.

## Files touched

- `crates/runie-core/src/commands/dsl/handlers/*.rs`
- `crates/runie-core/src/declarative/register.rs`
- `crates/runie-core/src/declarative/loader.rs`
- `crates/runie-core/resources/commands/*.yaml` (new)

## Notes

- Keep command handlers (the functions that execute commands) in Rust; only the specs move to YAML.
- This is a large task; consider splitting per handler category if it grows.
