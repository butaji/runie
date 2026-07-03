# AGENTS.md and frontmatter-based declarative configuration

**Status**: done
**Milestone**: R4
**Category**: Core / State
**Priority**: P2

**Depends on**: leader-actor-shared-runtime
**Blocks**: add-runie-inspect-command

## Summary

Replace imperative command/skill DSL registration with markdown files that use YAML frontmatter to declare slash commands, skills, agent profiles, hooks, and behaviors.

## Implementation Progress

### Completed
- [x] Generic `declarative/` module created
- [x] `DeclarativeLoader` struct for loading from multiple directories
- [x] `load_skills_from_dir()` for markdown files with YAML frontmatter
- [x] `load_commands_from_dir()` for YAML files
- [x] `SkillDef` and `CommandDef` types with triggers support
- [x] Frontmatter parsing (`extract_frontmatter()`, `parse_yaml_line()`, etc.)
- [x] Trigger parsing (`parse_triggers()`) for commands, file patterns, shortcuts
- [x] Comprehensive tests (57 passing)
- [x] Added to `arch_guardrails.rs` production allow list
- [x] Handler registration mechanism for frontmatter-defined commands (`register.rs`)
- [x] Intent-to-event dispatch table for declarative commands
- [x] Skill conversion from declarative definitions to runtime `Skill` objects

## Skill format (implemented)

```markdown
---
name: check-work
description: Verify changes with a subagent.
context: This skill verifies code changes.
triggers:
  - command: /check-work
  - command: /verify
invocation: user can invoke this with /check-work
---

## Usage

`/check-work [focus area]`
```

## Command format (implemented)

```yaml
# .runie/commands/bookmark.yaml
name: bookmark
description: Bookmark the current assistant message.
intent: BookmarkMessage
shortcut: Ctrl+b
category: Session
```

## Supported intents

The `register` module provides an intent-to-event dispatch table supporting:

- Session: `SaveCommand`, `LoadCommand`, `DeleteCommand`, `ExportCommand`, `ImportCommand`, `ForkCommand`, `CompactCommand`, `NameCommand`, `PromptCommand`
- Model: `ModelCommand`, `ThinkingCommand`
- Skills: `SkillCommand`
- System: `SwitchTheme`, `CopyToClipboard`, `ClearQueues`, `ToggleSettingsDialog`, `ShowDiagnostics`, `ToggleSessionTree`, `ShareSession`, `ApproveEdit`, `RejectEdit`, `ReloadAll`, `ProvidersDialog`

## Acceptance Criteria

- [x] The loader parses `.md` files with YAML frontmatter for skills and `.yaml` files for simple commands.
- [x] `declarative/` module provides generic loader infrastructure.
- [x] `register` module provides handler registration for frontmatter-defined commands.
- [x] Triggers include slash commands, file patterns, and explicit user invocation.
- [x] `register_declarative_commands()` converts YAML commands to dsl::CommandDef.
- [x] `register_declarative_skills()` converts declarative skills to runtime Skill objects.
- [x] `cargo check --workspace` is green.
- [x] 57 tests passing.

## Tests

### Layer 1 — State/Logic
- [x] `frontmatter_parses_name_and_description`
- [x] `frontmatter_strips_quotes`
- [x] `frontmatter_returns_none_without_delimiters`
- [x] `yaml_line_parses_key_value`
- [x] `yaml_line_handles_colons_in_values`
- [x] `triggers_parse_command`
- [x] `triggers_parse_command_list`
- [x] `triggers_parse_mixed_list`
- [x] `command_category_parses_known_values`
- [x] `skill_md_parsing_integration`
- [x] `loader_loads_command_yaml`
- [x] `loader_handles_invalid_yaml_gracefully`
- [x] `test_category_conversion`
- [x] `test_compact_args_parsing`
- [x] `test_register_command_creates_dsl_command`
- [x] `test_event_builder_for_save_command`
- [x] `test_event_builder_for_compact`
- [x] `test_model_command_no_args_opens_selector`

### Layer 2 — Event Handling
- [x] `test_register_command_creates_dsl_command` — verifies declarative command registration
- [x] `test_event_builder_for_save_command` — verifies event emission
- [x] `test_event_builder_for_compact` — verifies compound args parsing

### Layer 3 — Rendering
N/A

### Layer 4 — Provider Replay / Mock-Tool E2E
N/A

## Files Changed

- `crates/runie-core/src/declarative/mod.rs` — module exports
- `crates/runie-core/src/declarative/types.rs` — SkillDef, CommandDef, Trigger types
- `crates/runie-core/src/declarative/loader.rs` — generic loader implementation
- `crates/runie-core/src/declarative/register.rs` — handler registration and event dispatch
- `crates/runie-core/src/declarative/tests.rs` — comprehensive tests
- `crates/runie-core/src/lib.rs` — added declarative module exports
- `crates/runie-core/src/tests/arch_guardrails.rs` — added declarative/ to allow list
