# Deserialize declarative command YAML with typed structs

**Status**: todo
**Milestone**: R6
**Category**: Core / Declarative DSL
**Priority": P2

**Depends on**: use-pulldown-cmark-frontmatter-for-resource-loader
**Blocks**: move-built-in-slash-commands-to-declarative-yaml

## Description

`declarative::loader::parse_command_yaml` manually walks `serde_yaml::Mapping` and `parse_triggers` parses trigger lists as flat strings. Add typed `#[derive(Deserialize)]` structs and deserialize `triggers` as a real YAML list.

## Acceptance Criteria

- [ ] Define `DeclarativeCommandYaml` (or similar) with `serde::Deserialize`.
- [ ] Deserialize `triggers` as `Vec<Trigger>`.
- [ ] Delete `parse_category` once `CommandCategory` derives `EnumString`.
- [ ] `cargo test --workspace` succeeds after the change.
- [ ] `cargo check --workspace` succeeds with no new warnings.

## Tests

### Layer 1 — State/Logic
- [ ] `declarative_command_yaml_deserializes` — a YAML command spec parses correctly.
- [ ] `declarative_command_triggers_as_list` — triggers are a YAML list, not flat strings.

### Layer 2 — Event Handling
- [ ] N/A.

### Layer 3 — Rendering
- [ ] N/A.

### Layer 4 — Provider Replay / Mock-Tool E2E
- [ ] N/A.

## Files touched

- `crates/runie-core/src/declarative/loader.rs`
- `crates/runie-core/src/declarative/types.rs`
- `crates/runie-core/src/commands/dsl/category.rs`

## Notes

- Coordinate with `replace-remaining-custom-parsers-and-macros-with-strum.md` for `CommandCategory`.
