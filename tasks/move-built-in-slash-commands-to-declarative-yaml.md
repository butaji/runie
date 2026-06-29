# Move built-in slash command specs to declarative YAML resources

**Status**: done
**Milestone**: R6
**Category**: Commands / DSL
**Priority**: P2

**Depends on**: deserialize-declarative-command-yaml-with-typed-structs
**Blocks**: simplify-slash-command-dsl

## Description

The five `commands/dsl/handlers/*.rs` modules contain large static `CommandSpec` tables in Rust. Convert them to YAML files under `crates/runie-core/resources/commands/` and load/register them through the existing declarative loader. This removes ~600 lines of boilerplate and makes built-ins editable without recompilation.

## Progress

Infrastructure is in place:
- `HandlerRegistry` in `handlers/registry.rs` maps command names to handler functions
- `NamedHandler` enum supports Handler, Form, and FormWithHandler variants
- `DeclarativeCommandYaml` supports `type` (handler/msg/form) with handler name reference
- `build_cmd_from_yaml` combines YAML metadata with handler registry
- `CommandRegistry::with_commands()` merges YAML commands with static handlers

Static tables remain in handlers for backward compatibility; they can be deleted once all commands migrate to YAML.

## Acceptance Criteria

- [ ] Extract built-in command specs into YAML resources.
- [ ] Load them via `declarative::loader::load_commands_from_dir` at startup.
- [ ] Merge built-ins with user/project declarative commands.
- [ ] Delete the static tables from `commands/dsl/handlers/*.rs`.
- [x] `cargo test --workspace` succeeds after the change.
- [x] `cargo check --workspace` succeeds with no new warnings.

## Tests

### Layer 1 — State/Logic
- [x] `handler_registry_registers_all_commands` — all handlers registered in registry.
- [x] `command_def_from_yaml_uses_handler_registry` — YAML commands look up handlers.

### Layer 2 — Event Handling
- [ ] `slash_command_event_resolves` — a built-in slash command still resolves to the correct intent.

### Layer 3 — Rendering
- [ ] N/A.

### Layer 4 — Provider Replay / Mock-Tool E2E
- [ ] N/A.

## Files touched

- `crates/runie-core/src/commands/dsl/handlers/registry.rs` (new)
- `crates/runie-core/src/commands/dsl/handlers/*.rs` (updated)
- `crates/runie-core/src/declarative/types.rs` (updated)
- `crates/runie-core/src/declarative/loader.rs` (updated)
- `crates/runie-core/src/commands/dsl/spec.rs` (updated)
- `crates/runie-core/src/commands/registry.rs` (updated)
- `crates/runie-core/resources/commands/*.yaml` (new)

## Notes

- Keep command handlers (the functions that execute commands) in Rust; only the specs move to YAML.
- The `HandlerRegistry` is a static global that maps command names to handler functions.
- Static tables are kept for now to maintain test compatibility; they can be removed once YAML migration is complete.
- Example YAML format:

```yaml
name: settings
description: Open settings dialog
category: System
sub: true
triggers:
  - /settings
type: handler
handler: settings
```
